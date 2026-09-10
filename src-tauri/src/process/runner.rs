use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::platform::{self, ProcessGroup};

use super::command::CommandSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputLine {
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl CommandOutput {
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Combined output, preferring stderr because tools report failures there.
    pub fn diagnostics(&self) -> String {
        let mut combined = String::new();
        if !self.stderr.trim().is_empty() {
            combined.push_str(self.stderr.trim_end());
        }
        if !self.stdout.trim().is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(self.stdout.trim_end());
        }
        combined
    }
}

/// Runs a process to completion, capturing both streams.
pub async fn capture(spec: &CommandSpec) -> AppResult<CommandOutput> {
    run(spec, None, None).await
}

/// Captures output while placing the process in an existing supervised group.
pub async fn capture_in_group(
    spec: &CommandSpec,
    group: &ProcessGroup,
) -> AppResult<CommandOutput> {
    run(spec, None, Some(group)).await
}

/// Runs a process to completion, forwarding every output line as it is produced.
///
/// Lines are also accumulated so callers still receive the full output at the end.
pub async fn stream(
    spec: &CommandSpec,
    sink: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<CommandOutput> {
    run(spec, Some(sink), None).await
}

/// Streams output while placing the child, and everything it spawns, into a process group.
///
/// Needed for builds: terminating CMake alone would leave MSBuild and its compilers running.
pub async fn stream_in_group(
    spec: &CommandSpec,
    sink: mpsc::UnboundedSender<OutputLine>,
    group: &ProcessGroup,
) -> AppResult<CommandOutput> {
    run(spec, Some(sink), Some(group)).await
}

async fn run(
    spec: &CommandSpec,
    sink: Option<mpsc::UnboundedSender<OutputLine>>,
    group: Option<&ProcessGroup>,
) -> AppResult<CommandOutput> {
    let started = Instant::now();

    let resolved = platform::resolve_tool(&spec.program);
    let mut command = Command::new(
        resolved
            .as_deref()
            .map(|p| p.as_os_str())
            .unwrap_or(&spec.program),
    );
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(directory) = &spec.working_directory {
        command.current_dir(directory);
    }
    command.env("PATH", platform::tool_path());
    for (key, value) in &spec.environment {
        command.env(key, value);
    }
    platform::hide_console_window(&mut command);

    if group.is_some_and(ProcessGroup::is_terminated) {
        return Err(AppError::new(
            ErrorCode::BuildCancelled,
            "The build was cancelled.",
        ));
    }

    let owned_group = if group.is_none() {
        Some(ProcessGroup::new()?)
    } else {
        None
    };
    let group = group.or(owned_group.as_ref());
    if let Some(group) = group {
        group.prepare(&mut command);
    }

    let mut child = command
        .spawn()
        .map_err(|error| map_spawn_error(spec, error))?;

    if let (Some(group), Some(process_id)) = (group, child.id()) {
        if let Err(error) = group.adopt(process_id) {
            let _ = child.kill().await;
            return Err(error);
        }
    }

    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::internal(format!(
            "Could not capture stdout of {}.",
            spec.program_display()
        ))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AppError::internal(format!(
            "Could not capture stderr of {}.",
            spec.program_display()
        ))
    })?;

    let stdout_task = tokio::spawn(pump(stdout, OutputStream::Stdout, sink.clone()));
    let stderr_task = tokio::spawn(pump(stderr, OutputStream::Stderr, sink));

    let process_id = child.id();
    let status = child.wait().await;
    if let (Some(group), Some(pid)) = (group, process_id) {
        group.release(pid);
    }
    let status = status.map_err(|error| {
        AppError::new(
            ErrorCode::ProcessFailed,
            format!("{} did not run to completion.", spec.program_display()),
        )
        .with_details(error.to_string())
    })?;

    let stdout = stdout_task.await.unwrap_or_default();
    let stderr = stderr_task.await.unwrap_or_default();

    Ok(CommandOutput {
        exit_code: status.code(),
        stdout,
        stderr,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn map_spawn_error(spec: &CommandSpec, error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return AppError::new(
            ErrorCode::ProcessNotFound,
            format!("{} was not found.", spec.program_display()),
        )
        .with_hint("Install it, or set an explicit path in Settings.")
        .with_details(error.to_string());
    }

    AppError::new(
        ErrorCode::ProcessFailed,
        format!("Could not start {}.", spec.program_display()),
    )
    .with_details(error.to_string())
}

/// Reads a stream and emits one event per line.
///
/// Splits on both `\n` and `\r` because progress-reporting tools such as `git clone` overwrite a
/// single line with carriage returns; splitting on newlines alone would withhold all progress
/// until the operation finished.
async fn pump<R>(
    reader: R,
    stream: OutputStream,
    sink: Option<mpsc::UnboundedSender<OutputLine>>,
) -> String
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut reader = reader;
    let mut buffer = [0_u8; 4096];
    let mut pending: Vec<u8> = Vec::new();
    let mut collected = String::new();

    loop {
        let read = match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };

        for &byte in &buffer[..read] {
            if byte == b'\n' || byte == b'\r' {
                emit(&mut pending, &mut collected, stream, sink.as_ref());
            } else {
                pending.push(byte);
            }
        }
    }

    emit(&mut pending, &mut collected, stream, sink.as_ref());
    collected
}

fn emit(
    pending: &mut Vec<u8>,
    collected: &mut String,
    stream: OutputStream,
    sink: Option<&mpsc::UnboundedSender<OutputLine>>,
) {
    if pending.is_empty() {
        return;
    }

    let text = String::from_utf8_lossy(pending).into_owned();
    pending.clear();

    collected.push_str(&text);
    collected.push('\n');

    if let Some(sink) = sink {
        let _ = sink.send(OutputLine { stream, text });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_prefer_stderr_but_keep_stdout() {
        let output = CommandOutput {
            exit_code: Some(1),
            stdout: "some progress\n".into(),
            stderr: "fatal: not a git repository\n".into(),
            duration_ms: 4,
        };

        assert_eq!(
            output.diagnostics(),
            "fatal: not a git repository\nsome progress"
        );
        assert!(!output.succeeded());
    }

    #[test]
    fn diagnostics_are_empty_when_nothing_was_written() {
        let output = CommandOutput {
            exit_code: Some(0),
            stdout: "  \n".into(),
            stderr: String::new(),
            duration_ms: 1,
        };

        assert!(output.diagnostics().is_empty());
        assert!(output.succeeded());
    }

    #[tokio::test]
    async fn missing_programs_report_a_dedicated_error_code() {
        let spec = CommandSpec::new("llamapilot-definitely-missing-binary");
        let error = capture(&spec).await.expect_err("must fail");

        assert_eq!(error.code, ErrorCode::ProcessNotFound);
    }
}
