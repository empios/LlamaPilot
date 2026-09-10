use std::collections::BTreeMap;
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::ipc::Channel;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::platform::{self, ProcessGroup};
use crate::process::CommandSpec;

use super::logs::ServerLogBuffer;
use super::model::{
    ServerEvent, ServerLifecycleState, ServerLogStream, ServerLogsSnapshot, ServerSnapshot,
};
use super::probe;

#[derive(Debug, Clone)]
pub struct ServerLaunch {
    pub profile_id: String,
    pub profile_name: String,
    pub runtime_id: String,
    pub runtime_label: String,
    pub model_name: String,
    pub host: String,
    pub port: u16,
    pub command: CommandSpec,
    pub warnings: Vec<String>,
}

struct ActiveRun {
    generation: u64,
    group: Arc<ProcessGroup>,
    stop_requested: bool,
}

#[derive(Default)]
struct SupervisorInner {
    snapshot: ServerSnapshot,
    active: Option<ActiveRun>,
}

/// Owns the single llama-server process and all state derived from it.
///
/// The child is assigned to a platform process group, so closing the app or stopping the server
/// cannot leave a detached llama-server process behind.
pub struct ServerSupervisor {
    inner: AsyncMutex<SupervisorInner>,
    logs: Mutex<ServerLogBuffer>,
    subscribers: Mutex<BTreeMap<String, Channel<ServerEvent>>>,
    client: reqwest::Client,
}

impl ServerSupervisor {
    pub fn new(logs_directory: std::path::PathBuf) -> AppResult<Arc<Self>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(1_500))
            .build()
            .map_err(|error| {
                AppError::internal("Could not create the llama-server health client.")
                    .with_details(error.to_string())
            })?;
        Ok(Arc::new(Self {
            inner: AsyncMutex::new(SupervisorInner::default()),
            logs: Mutex::new(ServerLogBuffer::new(logs_directory)),
            subscribers: Mutex::new(BTreeMap::new()),
            client,
        }))
    }

    pub async fn snapshot(&self) -> ServerSnapshot {
        self.inner.lock().await.snapshot.clone()
    }

    pub fn logs_snapshot(&self) -> ServerLogsSnapshot {
        self.lock_logs().snapshot()
    }

    pub async fn subscribe(&self, channel: Channel<ServerEvent>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let snapshot = self.snapshot().await;
        let _ = channel.send(ServerEvent::Status {
            snapshot: Box::new(snapshot),
        });
        self.lock_subscribers().insert(id.clone(), channel);
        id
    }

    pub fn unsubscribe(&self, id: &str) -> bool {
        self.lock_subscribers().remove(id).is_some()
    }

    pub fn clear_logs(&self) {
        let next_sequence = self.lock_logs().clear();
        self.publish(ServerEvent::LogsCleared { next_sequence });
    }

    /// Reserves the launch policy decision without holding the port open. llama-server still owns
    /// the definitive bind; a race is reported through its normal process output and crash state.
    pub async fn select_port(&self, host: &str, preferred: u16, auto: bool) -> AppResult<u16> {
        if self.snapshot().await.state.is_active() {
            return Err(server_already_running());
        }

        if port_is_available(host, preferred) {
            return Ok(preferred);
        }
        if !auto {
            return Err(AppError::new(
                ErrorCode::PortInUse,
                format!("Port {preferred} on {host} is already in use."),
            )
            .with_hint("Stop the process using it or enable automatic port selection."));
        }

        TcpListener::bind((host, 0))
            .and_then(|listener| listener.local_addr())
            .map(|address| address.port())
            .map_err(|error| {
                AppError::new(
                    ErrorCode::PortInUse,
                    format!("Could not find a free port on {host}."),
                )
                .with_details(error.to_string())
            })
    }

    pub async fn start(self: &Arc<Self>, launch: ServerLaunch) -> AppResult<ServerSnapshot> {
        let mut inner = self.inner.lock().await;
        if inner.snapshot.state.is_active() || inner.active.is_some() {
            return Err(server_already_running());
        }

        let generation = inner.snapshot.generation.saturating_add(1);
        let log_file = self
            .lock_logs()
            .begin_run(&launch.profile_name, generation)?;
        inner.snapshot = ServerSnapshot {
            generation,
            state: ServerLifecycleState::Starting,
            pid: None,
            profile_id: Some(launch.profile_id.clone()),
            profile_name: Some(launch.profile_name.clone()),
            runtime_id: Some(launch.runtime_id.clone()),
            runtime_label: Some(launch.runtime_label.clone()),
            model_name: Some(launch.model_name.clone()),
            host: Some(launch.host.clone()),
            port: Some(launch.port),
            started_at: Some(now()),
            stopped_at: None,
            exit_code: None,
            health_status: None,
            health_message: Some("Waiting for llama-server.".into()),
            last_error: None,
            log_file: Some(log_file),
            telemetry: Default::default(),
        };

        let group = match ProcessGroup::new() {
            Ok(group) => Arc::new(group),
            Err(error) => {
                inner.snapshot.state = ServerLifecycleState::Crashed;
                inner.snapshot.stopped_at = Some(now());
                inner.snapshot.health_message = None;
                inner.snapshot.last_error = Some(error.to_string());
                let snapshot = inner.snapshot.clone();
                drop(inner);
                self.append_log(
                    ServerLogStream::System,
                    format!("Could not initialize process supervision: {error}"),
                );
                self.lock_logs().finish_run();
                self.publish(ServerEvent::Status {
                    snapshot: Box::new(snapshot),
                });
                return Err(error);
            }
        };
        let mut command = Command::new(&launch.command.program);
        command
            .args(&launch.command.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(directory) = &launch.command.working_directory {
            command.current_dir(directory);
        }
        command.env("PATH", platform::tool_path());
        for (key, value) in &launch.command.environment {
            command.env(key, value);
        }
        platform::hide_console_window(&mut command);

        group.prepare(&mut command);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                inner.snapshot.state = ServerLifecycleState::Crashed;
                inner.snapshot.stopped_at = Some(now());
                inner.snapshot.last_error = Some(error.to_string());
                inner.snapshot.health_message = None;
                let snapshot = inner.snapshot.clone();
                drop(inner);
                self.append_log(
                    ServerLogStream::System,
                    format!("Could not start llama-server: {error}"),
                );
                self.lock_logs().finish_run();
                self.publish(ServerEvent::Status {
                    snapshot: Box::new(snapshot),
                });
                return Err(AppError::new(
                    ErrorCode::ServerStartFailed,
                    "Could not start llama-server.",
                )
                .with_details(error.to_string()));
            }
        };

        let pid = child.id().ok_or_else(|| {
            group.terminate();
            AppError::new(
                ErrorCode::ServerStartFailed,
                "llama-server started without a process identifier.",
            )
        })?;
        if let Err(error) = group.adopt(pid) {
            let _ = child.start_kill();
            inner.snapshot.state = ServerLifecycleState::Crashed;
            inner.snapshot.stopped_at = Some(now());
            inner.snapshot.last_error = Some(error.to_string());
            let snapshot = inner.snapshot.clone();
            drop(inner);
            self.append_log(
                ServerLogStream::System,
                format!("Could not supervise llama-server: {error}"),
            );
            self.lock_logs().finish_run();
            self.publish(ServerEvent::Status {
                snapshot: Box::new(snapshot),
            });
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "Could not place llama-server under process supervision.",
            )
            .with_details(error.to_string()));
        }

        let stdout = child.stdout.take().ok_or_else(|| {
            group.terminate();
            AppError::internal("Could not capture llama-server stdout.")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            group.terminate();
            AppError::internal("Could not capture llama-server stderr.")
        })?;

        inner.snapshot.pid = Some(pid);
        let exit_group = Arc::clone(&group);
        inner.active = Some(ActiveRun {
            generation,
            group,
            stop_requested: false,
        });
        let snapshot = inner.snapshot.clone();
        drop(inner);

        self.append_log(
            ServerLogStream::System,
            format!(
                "Starting profile \"{}\" on {}:{}.",
                launch.profile_name, launch.host, launch.port
            ),
        );
        for warning in launch.warnings {
            self.append_log(ServerLogStream::System, format!("Warning: {warning}"));
        }
        self.publish(ServerEvent::Status {
            snapshot: Box::new(snapshot.clone()),
        });

        let stdout_supervisor = Arc::clone(self);
        let stdout_task = tokio::spawn(async move {
            pump_stream(stdout, ServerLogStream::Stdout, stdout_supervisor).await;
        });
        let stderr_supervisor = Arc::clone(self);
        let stderr_task = tokio::spawn(async move {
            pump_stream(stderr, ServerLogStream::Stderr, stderr_supervisor).await;
        });

        let exit_supervisor = Arc::clone(self);
        tokio::spawn(async move {
            let result = child.wait().await;
            exit_group.release(pid);
            let _ = tokio::join!(stdout_task, stderr_task);
            exit_supervisor.handle_exit(generation, result).await;
        });

        let health_supervisor = Arc::clone(self);
        tokio::spawn(async move {
            health_supervisor.health_loop(generation).await;
        });

        Ok(snapshot)
    }

    pub async fn stop(&self) -> AppResult<ServerSnapshot> {
        let mut inner = self.inner.lock().await;
        let Some(active) = inner.active.as_mut() else {
            return Err(AppError::new(
                ErrorCode::ServerNotRunning,
                "llama-server is not running.",
            ));
        };
        active.stop_requested = true;
        let group = Arc::clone(&active.group);
        inner.snapshot.state = ServerLifecycleState::Stopping;
        inner.snapshot.health_message = Some("Stopping llama-server.".into());
        let snapshot = inner.snapshot.clone();
        drop(inner);

        self.append_log(ServerLogStream::System, "Stop requested.".into());
        self.publish(ServerEvent::Status {
            snapshot: Box::new(snapshot.clone()),
        });
        group.terminate();
        Ok(snapshot)
    }

    pub async fn wait_until_stopped(&self, timeout: Duration) -> AppResult<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !self.snapshot().await.state.is_active() {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(AppError::new(
                    ErrorCode::ServerStartFailed,
                    "Timed out while waiting for llama-server to stop.",
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn handle_exit(
        &self,
        generation: u64,
        result: std::io::Result<std::process::ExitStatus>,
    ) {
        let mut inner = self.inner.lock().await;
        let Some(active) = inner
            .active
            .as_ref()
            .filter(|active| active.generation == generation)
        else {
            return;
        };
        let requested = active.stop_requested;
        let (exit_code, wait_error) = match result {
            Ok(status) => (status.code(), None),
            Err(error) => (None, Some(error.to_string())),
        };
        inner.active = None;
        inner.snapshot.pid = None;
        inner.snapshot.stopped_at = Some(now());
        inner.snapshot.exit_code = exit_code;
        inner.snapshot.health_status = None;
        inner.snapshot.telemetry = Default::default();
        if requested {
            inner.snapshot.state = ServerLifecycleState::Stopped;
            inner.snapshot.health_message = None;
            inner.snapshot.last_error = None;
        } else {
            inner.snapshot.state = ServerLifecycleState::Crashed;
            inner.snapshot.health_message = None;
            inner.snapshot.last_error = Some(wait_error.unwrap_or_else(|| match exit_code {
                Some(code) => format!("llama-server exited unexpectedly with code {code}."),
                None => "llama-server exited unexpectedly.".into(),
            }));
        }
        let snapshot = inner.snapshot.clone();
        drop(inner);

        let message = if requested {
            "llama-server stopped.".to_string()
        } else {
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "llama-server crashed.".into())
        };
        self.append_log(ServerLogStream::System, message);
        self.lock_logs().finish_run();
        self.publish(ServerEvent::Status {
            snapshot: Box::new(snapshot),
        });
    }

    async fn health_loop(self: Arc<Self>, generation: u64) {
        loop {
            let snapshot = self.snapshot().await;
            if snapshot.generation != generation || !snapshot.state.is_active() {
                break;
            }
            if snapshot.state == ServerLifecycleState::Stopping {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            let Some(host) = snapshot.host.as_deref() else {
                break;
            };
            let Some(port) = snapshot.port else {
                break;
            };
            let base_url = base_url(host, port);
            let include_details = matches!(
                snapshot.state,
                ServerLifecycleState::Ready | ServerLifecycleState::Busy
            );
            let result = probe::probe(&self.client, &base_url, include_details).await;

            let mut inner = self.inner.lock().await;
            if inner.snapshot.generation != generation
                || !inner.snapshot.state.is_active()
                || inner.snapshot.state == ServerLifecycleState::Stopping
            {
                break;
            }
            let previous = inner.snapshot.clone();
            let previous_state = previous.state;
            inner.snapshot.health_status = result.health_status;
            inner.snapshot.health_message = result.health_message;
            if result.health_loading {
                inner.snapshot.state = ServerLifecycleState::Loading;
                inner.snapshot.health_message = Some("Loading model.".into());
            } else if result.health_ready {
                if include_details {
                    inner.snapshot.telemetry = result.telemetry;
                }
                inner.snapshot.state = if inner.snapshot.telemetry.is_busy() {
                    ServerLifecycleState::Busy
                } else {
                    ServerLifecycleState::Ready
                };
                inner.snapshot.health_message = Some("Healthy.".into());
            }
            let next_state = inner.snapshot.state;
            let updated = inner.snapshot.clone();
            drop(inner);

            if previous_state != next_state {
                self.append_log(
                    ServerLogStream::System,
                    format!("Server state changed to {}.", state_label(next_state)),
                );
            }
            if previous != updated {
                self.publish(ServerEvent::Status {
                    snapshot: Box::new(updated),
                });
            }

            let interval = if matches!(
                next_state,
                ServerLifecycleState::Ready | ServerLifecycleState::Busy
            ) {
                Duration::from_secs(1)
            } else {
                Duration::from_millis(350)
            };
            tokio::time::sleep(interval).await;
        }
    }

    fn append_log(&self, stream: ServerLogStream, text: String) {
        let entry = self.lock_logs().append(stream, text);
        self.publish(ServerEvent::Log { entry });
    }

    fn publish(&self, event: ServerEvent) {
        self.lock_subscribers()
            .retain(|_, channel| channel.send(event.clone()).is_ok());
    }

    fn lock_logs(&self) -> std::sync::MutexGuard<'_, ServerLogBuffer> {
        self.logs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_subscribers(
        &self,
    ) -> std::sync::MutexGuard<'_, BTreeMap<String, Channel<ServerEvent>>> {
        self.subscribers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for ServerSupervisor {
    fn drop(&mut self) {
        if let Ok(inner) = self.inner.try_lock() {
            if let Some(active) = &inner.active {
                active.group.terminate();
            }
        }
    }
}

async fn pump_stream<R>(mut reader: R, stream: ServerLogStream, supervisor: Arc<ServerSupervisor>)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 4096];
    let mut pending = Vec::new();
    loop {
        let count = match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        for &byte in &buffer[..count] {
            if byte == b'\n' || byte == b'\r' {
                emit_line(&mut pending, stream, &supervisor);
            } else {
                pending.push(byte);
            }
        }
    }
    emit_line(&mut pending, stream, &supervisor);
}

fn emit_line(pending: &mut Vec<u8>, stream: ServerLogStream, supervisor: &ServerSupervisor) {
    if pending.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(pending).into_owned();
    pending.clear();
    supervisor.append_log(stream, text);
}

fn port_is_available(host: &str, port: u16) -> bool {
    TcpListener::bind((host, port)).is_ok()
}

fn base_url(host: &str, port: u16) -> String {
    let host = match host {
        "0.0.0.0" | "*" => "127.0.0.1".to_string(),
        "::" | "[::]" => "[::1]".to_string(),
        value if value.contains(':') && !value.starts_with('[') => format!("[{value}]"),
        value => value.to_string(),
    };
    format!("http://{host}:{port}")
}

fn server_already_running() -> AppError {
    AppError::new(
        ErrorCode::ServerAlreadyRunning,
        "A llama-server process is already active.",
    )
    .with_hint("Stop the current server before starting another profile.")
}

fn state_label(state: ServerLifecycleState) -> &'static str {
    match state {
        ServerLifecycleState::Stopped => "stopped",
        ServerLifecycleState::Starting => "starting",
        ServerLifecycleState::Loading => "loading",
        ServerLifecycleState::Ready => "ready",
        ServerLifecycleState::Busy => "busy",
        ServerLifecycleState::Stopping => "stopping",
        ServerLifecycleState::Crashed => "crashed",
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_url_uses_loopback_for_wildcard_hosts() {
        assert_eq!(base_url("0.0.0.0", 8080), "http://127.0.0.1:8080");
        assert_eq!(base_url("::", 8080), "http://[::1]:8080");
        assert_eq!(base_url("::1", 8080), "http://[::1]:8080");
    }

    #[test]
    fn occupied_ports_are_detected() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test listener");
        let port = listener.local_addr().expect("address").port();
        assert!(!port_is_available("127.0.0.1", port));
    }

    #[tokio::test]
    async fn automatic_port_policy_replaces_an_occupied_preference() {
        let temp = tempfile::tempdir().expect("temp");
        let supervisor = ServerSupervisor::new(temp.path().to_path_buf()).expect("supervisor");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test listener");
        let occupied = listener.local_addr().expect("address").port();

        let error = supervisor
            .select_port("127.0.0.1", occupied, false)
            .await
            .expect_err("fixed occupied port");
        assert_eq!(error.code, ErrorCode::PortInUse);

        let selected = supervisor
            .select_port("127.0.0.1", occupied, true)
            .await
            .expect("automatic port");
        assert_ne!(selected, occupied);
        assert_ne!(selected, 0);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn captures_both_streams_and_reaches_stopped_after_termination() {
        let temp = tempfile::tempdir().expect("temp");
        let supervisor = ServerSupervisor::new(temp.path().to_path_buf()).expect("supervisor");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("test listener");
        let port = listener.local_addr().expect("address").port();
        drop(listener);
        // Avoid cold PowerShell/.NET startup in a test of process supervision, not shell startup.
        let command = CommandSpec::new("cmd.exe").args([
            "/D",
            "/C",
            "echo fixture stdout&1>&2 echo fixture stderr&ping -n 31 127.0.0.1 >nul",
        ]);

        supervisor
            .start(ServerLaunch {
                profile_id: "profile-fixture".into(),
                profile_name: "Supervisor fixture".into(),
                runtime_id: "runtime-fixture".into(),
                runtime_label: "fixture runtime".into(),
                model_name: "fixture model".into(),
                host: "127.0.0.1".into(),
                port,
                command,
                warnings: Vec::new(),
            })
            .await
            .expect("start fixture");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let logs = supervisor.logs_snapshot();
            let stdout = logs
                .entries
                .iter()
                .any(|entry| entry.text == "fixture stdout");
            let stderr = logs
                .entries
                .iter()
                .any(|entry| entry.text == "fixture stderr");
            if stdout && stderr {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "fixture output timed out: {:?}",
                logs.entries
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        supervisor.stop().await.expect("stop fixture");
        supervisor
            .wait_until_stopped(Duration::from_secs(5))
            .await
            .expect("fixture stopped");
        assert_eq!(
            supervisor.snapshot().await.state,
            ServerLifecycleState::Stopped
        );
    }
}
