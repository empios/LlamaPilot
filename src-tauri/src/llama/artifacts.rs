use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::config::store::write_json_atomic;
use crate::error::{AppError, AppResult, ErrorCode};

use super::capabilities::{
    LlamaCapabilities, LlamaRawOutputs, RawCommandOutput, RuntimeCapabilitySummary,
    RuntimeInspection,
};

pub const CAPABILITIES_DIRECTORY: &str = "capabilities";
pub const CAPABILITIES_MANIFEST: &str = "manifest.json";

const VERSION_STDOUT: &str = "version.stdout.txt";
const VERSION_STDERR: &str = "version.stderr.txt";
const HELP_STDOUT: &str = "help.stdout.txt";
const HELP_STDERR: &str = "help.stderr.txt";
const DEVICES_STDOUT: &str = "devices.stdout.txt";
const DEVICES_STDERR: &str = "devices.stderr.txt";

/// Writes an immutable, versioned inspection directory and returns the lightweight summary that
/// is embedded in runtime metadata. A later re-inspection writes a new id; readers never observe
/// a partially replaced set of raw files.
pub fn persist(
    runtime_directory: &Path,
    inspection: &RuntimeInspection,
) -> AppResult<RuntimeCapabilitySummary> {
    let inspection_id = uuid::Uuid::new_v4().to_string();
    let directory = inspection_directory(runtime_directory, &inspection_id);
    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not create {}.", directory.display()),
        )
        .with_details(error.to_string())
    })?;

    let result = write_inspection(&directory, inspection);
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }

    Ok(RuntimeCapabilitySummary::from_capabilities(
        inspection_id,
        chrono::Utc::now().to_rfc3339(),
        &inspection.capabilities,
    ))
}

pub fn load(runtime_directory: &Path, inspection_id: &str) -> AppResult<RuntimeInspection> {
    validate_inspection_id(inspection_id)?;
    let directory = inspection_directory(runtime_directory, inspection_id);
    let mut capabilities: LlamaCapabilities = read_json(&directory.join(CAPABILITIES_MANIFEST))?;
    capabilities.refresh_known_registry();
    let raw = LlamaRawOutputs {
        version: read_raw(&directory, VERSION_STDOUT, VERSION_STDERR)?,
        help: read_raw(&directory, HELP_STDOUT, HELP_STDERR)?,
        devices: read_raw(&directory, DEVICES_STDOUT, DEVICES_STDERR)?,
    };
    Ok(RuntimeInspection { capabilities, raw })
}

/// Removes only a superseded inspection directory. Runtime binaries and the current manifest are
/// never touched by this cleanup.
pub fn remove(runtime_directory: &Path, inspection_id: &str) {
    if validate_inspection_id(inspection_id).is_err() {
        return;
    }
    let directory = inspection_directory(runtime_directory, inspection_id);
    let root = runtime_directory.join(CAPABILITIES_DIRECTORY);
    if directory.parent() == Some(root.as_path()) && directory.is_dir() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

/// Best-effort startup cleanup for inspection directories that were superseded or written just
/// before a crash. Only UUID-named directories inside the managed capability root are eligible.
pub fn cleanup_superseded(runtime_directory: &Path, current_id: Option<&str>) -> usize {
    let root = runtime_directory.join(CAPABILITIES_DIRECTORY);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return 0;
    };

    let mut removed = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if current_id == Some(name)
            || uuid::Uuid::parse_str(name).is_err()
            || !entry.path().is_dir()
        {
            continue;
        }
        if std::fs::remove_dir_all(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn write_inspection(directory: &Path, inspection: &RuntimeInspection) -> AppResult<()> {
    write_text(
        &directory.join(VERSION_STDOUT),
        &inspection.raw.version.stdout,
    )?;
    write_text(
        &directory.join(VERSION_STDERR),
        &inspection.raw.version.stderr,
    )?;
    write_text(&directory.join(HELP_STDOUT), &inspection.raw.help.stdout)?;
    write_text(&directory.join(HELP_STDERR), &inspection.raw.help.stderr)?;
    write_text(
        &directory.join(DEVICES_STDOUT),
        &inspection.raw.devices.stdout,
    )?;
    write_text(
        &directory.join(DEVICES_STDERR),
        &inspection.raw.devices.stderr,
    )?;
    // The manifest is the commit marker and is deliberately written last.
    write_json_atomic(
        &directory.join(CAPABILITIES_MANIFEST),
        &inspection.capabilities,
    )
}

fn write_text(path: &Path, contents: &str) -> AppResult<()> {
    std::fs::write(path, contents.as_bytes()).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not write {}.", path.display()),
        )
        .with_details(error.to_string())
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<T> {
    let contents = std::fs::read(path).map_err(|error| {
        AppError::new(ErrorCode::Io, format!("Could not read {}.", path.display()))
            .with_details(error.to_string())
    })?;
    serde_json::from_slice(&contents).map_err(|error| {
        AppError::new(
            ErrorCode::Config,
            format!("{} is not a valid capability manifest.", path.display()),
        )
        .with_details(error.to_string())
    })
}

fn read_raw(directory: &Path, stdout: &str, stderr: &str) -> AppResult<RawCommandOutput> {
    Ok(RawCommandOutput {
        stdout: read_text(&directory.join(stdout))?,
        stderr: read_text(&directory.join(stderr))?,
    })
}

fn read_text(path: &Path) -> AppResult<String> {
    std::fs::read_to_string(path).map_err(|error| {
        AppError::new(ErrorCode::Io, format!("Could not read {}.", path.display()))
            .with_details(error.to_string())
    })
}

fn inspection_directory(runtime_directory: &Path, inspection_id: &str) -> PathBuf {
    runtime_directory
        .join(CAPABILITIES_DIRECTORY)
        .join(inspection_id)
}

fn validate_inspection_id(inspection_id: &str) -> AppResult<()> {
    uuid::Uuid::parse_str(inspection_id).map_err(|_| {
        AppError::internal("The runtime contains an invalid capability inspection id.")
            .with_details(inspection_id.to_string())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::llama::capabilities::{LlamaCapabilities, CAPABILITIES_SCHEMA_VERSION};

    fn inspection() -> RuntimeInspection {
        RuntimeInspection {
            capabilities: LlamaCapabilities {
                schema_version: CAPABILITIES_SCHEMA_VERSION,
                version: "b7900-deadbeef".into(),
                commit: Some("deadbeef".into()),
                options: BTreeMap::new(),
                speculative_types: vec!["none".into()],
                devices: Vec::new(),
            },
            raw: LlamaRawOutputs {
                version: RawCommandOutput {
                    stdout: String::new(),
                    stderr: "version: b7900-deadbeef\n".into(),
                },
                help: RawCommandOutput {
                    stdout: "--help  print help\n".into(),
                    stderr: String::new(),
                },
                devices: RawCommandOutput {
                    stdout: "Available devices:\n  (none)\n".into(),
                    stderr: String::new(),
                },
            },
        }
    }

    #[test]
    fn persists_and_reloads_manifest_and_both_raw_streams() {
        let temp = tempfile::tempdir().expect("temp dir");
        let expected = inspection();
        let summary = persist(temp.path(), &expected).expect("persist");
        let loaded = load(temp.path(), &summary.inspection_id).expect("load");

        assert_eq!(loaded, expected);
        assert_eq!(summary.version, "b7900-deadbeef");
        assert!(temp
            .path()
            .join(CAPABILITIES_DIRECTORY)
            .join(&summary.inspection_id)
            .join(CAPABILITIES_MANIFEST)
            .is_file());
    }

    #[test]
    fn rejects_path_traversal_as_an_inspection_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let error = load(temp.path(), "../metadata.json").expect_err("must reject");
        assert_eq!(error.code, ErrorCode::Internal);
    }

    #[test]
    fn startup_cleanup_removes_only_superseded_uuid_directories() {
        let temp = tempfile::tempdir().expect("temp dir");
        let current = persist(temp.path(), &inspection()).expect("current");
        let stale = persist(temp.path(), &inspection()).expect("stale");
        let user_named = temp.path().join(CAPABILITIES_DIRECTORY).join("notes");
        std::fs::create_dir_all(&user_named).expect("user-like directory");

        assert_eq!(
            cleanup_superseded(temp.path(), Some(&current.inspection_id)),
            1
        );
        assert!(inspection_directory(temp.path(), &current.inspection_id).is_dir());
        assert!(!inspection_directory(temp.path(), &stale.inspection_id).exists());
        assert!(user_named.is_dir());
    }
}
