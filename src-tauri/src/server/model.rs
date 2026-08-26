use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerLifecycleState {
    Stopped,
    Starting,
    Loading,
    Ready,
    Busy,
    Stopping,
    Crashed,
}

impl ServerLifecycleState {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Loading | Self::Ready | Self::Busy | Self::Stopping
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTelemetry {
    pub props_available: Option<bool>,
    pub slots_available: Option<bool>,
    pub metrics_available: Option<bool>,
    pub total_slots: Option<u64>,
    pub busy_slots: Option<u64>,
    pub requests_processing: Option<f64>,
    pub requests_deferred: Option<f64>,
    pub prompt_tokens_per_second: Option<f64>,
    pub predicted_tokens_per_second: Option<f64>,
    pub build_info: Option<String>,
}

impl ServerTelemetry {
    pub fn is_busy(&self) -> bool {
        self.busy_slots.unwrap_or(0) > 0 || self.requests_processing.unwrap_or(0.0) > 0.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSnapshot {
    pub generation: u64,
    pub state: ServerLifecycleState,
    pub pid: Option<u32>,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub runtime_id: Option<String>,
    pub runtime_label: Option<String>,
    pub model_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub exit_code: Option<i32>,
    pub health_status: Option<u16>,
    pub health_message: Option<String>,
    pub last_error: Option<String>,
    pub log_file: Option<PathBuf>,
    pub telemetry: ServerTelemetry,
}

impl Default for ServerSnapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            state: ServerLifecycleState::Stopped,
            pid: None,
            profile_id: None,
            profile_name: None,
            runtime_id: None,
            runtime_label: None,
            model_name: None,
            host: None,
            port: None,
            started_at: None,
            stopped_at: None,
            exit_code: None,
            health_status: None,
            health_message: None,
            last_error: None,
            log_file: None,
            telemetry: ServerTelemetry::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerLogStream {
    System,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerLogFact {
    ModelLoad,
    GpuOffload,
    KvCache,
    Listening,
    Throughput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLogEntry {
    pub sequence: u64,
    pub timestamp: String,
    pub stream: ServerLogStream,
    pub level: ServerLogLevel,
    pub fact: Option<ServerLogFact>,
    /// Untouched child output. Classification is stored beside it and never rewrites it.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLogsSnapshot {
    pub entries: Vec<ServerLogEntry>,
    pub dropped_entries: u64,
    pub next_sequence: u64,
    pub current_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ServerEvent {
    Status { snapshot: Box<ServerSnapshot> },
    Log { entry: ServerLogEntry },
    LogsCleared { next_sequence: u64 },
}
