use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::build::profile::{BuildBackend, BuildConfiguration};
use crate::llama::RuntimeCapabilitySummary;

/// A commit-keyed snapshot of a built llama.cpp server.
///
/// Its executable and libraries are immutable. Rebuilding the same commit produces a new
/// snapshot rather than overwriting this one, which keeps a working runtime working and avoids
/// Windows file locks. Capability inspection may refresh only versioned sidecars and this
/// metadata pointer; it never changes runnable artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRecord {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub repository: String,
    pub commit: String,
    pub short_commit: String,
    pub branch: String,
    pub backend: BuildBackend,
    pub configuration: BuildConfiguration,
    pub generator: String,
    pub build_date: String,
    pub directory: PathBuf,
    pub executable: PathBuf,
    /// Total size of the copied artifacts, for the runtime list.
    pub size_bytes: u64,
    pub file_count: usize,
    /// Lightweight pointer to the immutable capability sidecar for this exact executable.
    #[serde(default)]
    pub capabilities: Option<RuntimeCapabilitySummary>,
}

impl RuntimeRecord {
    /// Human-readable identity, e.g. `master @ abc1234 · CUDA`.
    pub fn label(&self) -> String {
        format!(
            "{} @ {} · {}",
            self.branch,
            self.short_commit,
            self.backend.label()
        )
    }

    /// A runtime is only usable while its executable is still on disk.
    pub fn is_available(&self) -> bool {
        self.executable.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> RuntimeRecord {
        RuntimeRecord {
            id: "r1".into(),
            source_id: "s1".into(),
            source_name: "llama.cpp".into(),
            repository: "https://github.com/ggml-org/llama.cpp".into(),
            commit: "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678".into(),
            short_commit: "a1b2c3d".into(),
            branch: "master".into(),
            backend: BuildBackend::Cuda,
            configuration: BuildConfiguration::Release,
            generator: "Visual Studio 18 2026".into(),
            build_date: "2026-08-25T12:00:00Z".into(),
            directory: PathBuf::from("/data/runtimes/a1b2c3d/cuda"),
            executable: PathBuf::from("/data/runtimes/a1b2c3d/cuda/llama-server.exe"),
            size_bytes: 1024,
            file_count: 7,
            capabilities: None,
        }
    }

    #[test]
    fn the_label_identifies_branch_commit_and_backend() {
        assert_eq!(record().label(), "master @ a1b2c3d · CUDA");
    }

    #[test]
    fn a_runtime_whose_executable_is_gone_is_not_available() {
        assert!(!record().is_available());
    }

    #[test]
    fn serializes_with_camel_case_keys_for_the_frontend() {
        let json = serde_json::to_value(record()).expect("serializable");

        assert_eq!(json["shortCommit"], "a1b2c3d");
        assert_eq!(json["backend"], "cuda");
        assert_eq!(json["configuration"], "release");
        assert_eq!(json["sourceName"], "llama.cpp");
        assert!(json["capabilities"].is_null());
    }

    #[test]
    fn phase_three_metadata_without_capabilities_remains_readable() {
        let mut json = serde_json::to_value(record()).expect("serializable");
        json.as_object_mut()
            .expect("record object")
            .remove("capabilities");

        let restored: RuntimeRecord = serde_json::from_value(json).expect("backwards compatible");
        assert!(restored.capabilities.is_none());
    }
}
