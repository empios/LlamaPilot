use std::fmt;

use serde::{Serialize, Serializer};

/// Closed set of failure kinds the frontend is allowed to branch on.
///
/// Adding a variant here is intentional friction: it forces us to decide what the UI should say
/// instead of leaking a raw error string into a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    Io,
    Config,
    InvalidPath,
    ProcessNotFound,
    ProcessFailed,
    GitNotFound,
    GitFailed,
    NotARepository,
    DirectoryNotEmpty,
    DirtyWorktree,
    DetachedHead,
    NoUpstream,
    NotFastForward,
    UnknownRef,
    RemoteExists,
    RemoteNotFound,
    SourceNotFound,
    SourceExists,
    ToolchainIncomplete,
    ConfigureFailed,
    BuildFailed,
    BuildCancelled,
    BuildArtifactMissing,
    BuildInProgress,
    RuntimeExists,
    RuntimeNotFound,
    CapabilityDiscoveryFailed,
    InvalidGguf,
    ModelScanFailed,
    ModelDownloadFailed,
    ModelDownloadInProgress,
    ModelDownloadCancelled,
    InvalidProfile,
    ProfileNotFound,
    PortInUse,
    ServerAlreadyRunning,
    ServerNotRunning,
    ServerStartFailed,
    BenchmarkFailed,
    BenchmarkInProgress,
    BenchmarkCancelled,
    Unsupported,
    Internal,
}

/// Structured, user-facing failure.
///
/// `message` is the sentence shown to the user, `hint` is the suggested next action and
/// `details` carries untouched raw tool output so nothing is ever hidden.
#[derive(Debug, Clone)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub hint: Option<String>,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
            details: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        let details = details.into();
        if !details.trim().is_empty() {
            self.details = Some(details);
        }
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }

    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidPath, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unsupported, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppErrorPayload<'a> {
    code: ErrorCode,
    message: &'a str,
    hint: Option<&'a str>,
    details: Option<&'a str>,
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload {
            code: self.code,
            message: &self.message,
            hint: self.hint.as_deref(),
            details: self.details.as_deref(),
        }
        .serialize(serializer)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::new(ErrorCode::Io, value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::new(
            ErrorCode::Config,
            "Failed to read or write a configuration file.",
        )
        .with_details(value.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        AppError::internal(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_a_stable_camel_case_shape() {
        let error = AppError::new(ErrorCode::DirtyWorktree, "Uncommitted changes present.")
            .with_hint("Commit or stash them first.")
            .with_details("error: cannot switch");

        let json = serde_json::to_value(&error).expect("serializable");

        assert_eq!(json["code"], "dirtyWorktree");
        assert_eq!(json["message"], "Uncommitted changes present.");
        assert_eq!(json["hint"], "Commit or stash them first.");
        assert_eq!(json["details"], "error: cannot switch");
    }

    #[test]
    fn blank_details_are_dropped() {
        let error = AppError::internal("boom").with_details("   \n ");
        assert!(error.details.is_none());
    }
}
