use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::store::write_json_atomic;
use crate::error::{AppError, AppResult, ErrorCode};

use super::record::RuntimeRecord;

/// Executable produced by the `llama-server` target.
pub const SERVER_EXECUTABLE: &str = if cfg!(windows) {
    "llama-server.exe"
} else {
    "llama-server"
};

pub const RUNTIME_METADATA_FILE: &str = "metadata.json";

const STAGING_MARKER: &str = ".staging-";

/// File extensions that belong to a runnable runtime.
///
/// Import libraries, PDBs, and CMake bookkeeping are excluded: they inflate the snapshot without
/// being needed to run the server.
fn is_runtime_artifact(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains(".so."))
    {
        return true;
    }
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        // Extensionless files are Unix executables such as `llama-server`.
        return cfg!(not(windows));
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "exe" | "dll" | "so" | "dylib" | "metallib" | "metal"
    )
}

/// Finds the directory containing the freshly built server binary.
pub fn locate_artifacts(candidates: &[PathBuf]) -> AppResult<PathBuf> {
    for candidate in candidates {
        if candidate.join(SERVER_EXECUTABLE).is_file() {
            return Ok(candidate.clone());
        }
    }

    Err(AppError::new(
        ErrorCode::BuildArtifactMissing,
        format!("The build finished but {SERVER_EXECUTABLE} was not produced."),
    )
    .with_hint("Check the build log for the real failure; CMake can report success after a target is skipped.")
    .with_details(
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

#[derive(Debug, Clone)]
pub struct CopiedArtifacts {
    pub executable: PathBuf,
    pub file_count: usize,
    pub size_bytes: u64,
}

/// A complete runtime hidden in a staging directory until its metadata is durable.
#[derive(Debug)]
pub struct RuntimeSnapshotStage {
    destination: PathBuf,
    staging_directory: PathBuf,
    file_count: usize,
    size_bytes: u64,
}

impl RuntimeSnapshotStage {
    pub fn directory(&self) -> &Path {
        &self.staging_directory
    }

    pub fn executable(&self) -> PathBuf {
        self.staging_directory.join(SERVER_EXECUTABLE)
    }

    pub fn file_count(&self) -> usize {
        self.file_count
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Writes the self-describing metadata and atomically publishes the completed directory.
    pub fn commit<T: Serialize>(self, metadata: &T) -> AppResult<CopiedArtifacts> {
        let serialized = serde_json::to_vec_pretty(metadata)?;
        let metadata_path = self.staging_directory.join(RUNTIME_METADATA_FILE);
        std::fs::write(&metadata_path, serialized).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not write {}.", metadata_path.display()),
            )
            .with_details(error.to_string())
        })?;

        std::fs::rename(&self.staging_directory, &self.destination).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!(
                    "Could not publish the runtime at {}.",
                    self.destination.display()
                ),
            )
            .with_details(error.to_string())
        })?;

        Ok(CopiedArtifacts {
            executable: self.destination.join(SERVER_EXECUTABLE),
            file_count: self.file_count,
            size_bytes: self.size_bytes,
        })
    }
}

/// Refreshes only self-describing metadata. Executables and runtime libraries remain immutable.
pub fn write_runtime_metadata(runtime: &RuntimeRecord) -> AppResult<()> {
    if !runtime.directory.is_dir()
        || runtime.executable != runtime.directory.join(SERVER_EXECUTABLE)
        || !runtime.executable.is_file()
    {
        return Err(AppError::invalid_path(
            "The runtime cannot be updated because its files are incomplete.",
        )
        .with_details(runtime.directory.display().to_string()));
    }
    write_json_atomic(&runtime.directory.join(RUNTIME_METADATA_FILE), runtime)
}

impl Drop for RuntimeSnapshotStage {
    fn drop(&mut self) {
        if self.staging_directory.is_dir() {
            let _ = std::fs::remove_dir_all(&self.staging_directory);
        }
    }
}

/// Copies the complete runnable artifact set into a private staging directory.
///
/// Nothing becomes visible at `destination` until [`RuntimeSnapshotStage::commit`] has written
/// metadata and renamed the completed directory. Any error drops and removes the staging tree.
pub fn stage_runtime(
    source_directory: &Path,
    destination: &Path,
) -> AppResult<RuntimeSnapshotStage> {
    if destination.exists() {
        return Err(AppError::new(
            ErrorCode::RuntimeExists,
            "A runtime snapshot already exists at that location.",
        )
        .with_hint("Build again to create a new snapshot with a different runtime id.")
        .with_details(destination.display().to_string()));
    }

    let parent = destination.parent().ok_or_else(|| {
        AppError::invalid_path("The runtime destination has no parent directory.")
            .with_details(destination.display().to_string())
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not create {}.", parent.display()),
        )
        .with_details(error.to_string())
    })?;

    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("runtime");
    let staging_directory = parent.join(format!(".{name}{STAGING_MARKER}{}", uuid::Uuid::new_v4()));

    std::fs::create_dir(&staging_directory).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not create {}.", staging_directory.display()),
        )
        .with_details(error.to_string())
    })?;

    let mut stage = RuntimeSnapshotStage {
        destination: destination.to_path_buf(),
        staging_directory,
        file_count: 0,
        size_bytes: 0,
    };

    let entries = std::fs::read_dir(source_directory).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not read {}.", source_directory.display()),
        )
        .with_details(error.to_string())
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not enumerate {}.", source_directory.display()),
            )
            .with_details(error.to_string())
        })?;
        let path = entry.path();
        if !path.is_file() || !is_runtime_artifact(&path) {
            continue;
        }

        let Some(name) = path.file_name() else {
            continue;
        };

        let target = stage.staging_directory.join(name);
        // Materialize symlinks so a snapshot never depends on its build directory.
        let copied = std::fs::copy(&path, &target).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not copy {} into the runtime.", path.display()),
            )
            .with_details(error.to_string())
        })?;

        stage.file_count += 1;
        stage.size_bytes += copied;
    }

    let executable = stage.staging_directory.join(SERVER_EXECUTABLE);
    if !executable.is_file() {
        return Err(AppError::new(
            ErrorCode::BuildArtifactMissing,
            format!("{SERVER_EXECUTABLE} was not copied into the runtime."),
        )
        .with_details(source_directory.display().to_string()));
    }

    Ok(stage)
}

/// Directory for one immutable build, grouped by commit and backend and made unique by runtime id.
pub fn snapshot_directory(
    runtimes_root: &Path,
    commit: &str,
    backend: &str,
    runtime_id: &str,
) -> PathBuf {
    let short: String = commit.chars().take(12).collect();
    runtimes_root.join(short).join(backend).join(runtime_id)
}

/// Removes staging trees left by a process crash. Called once during application startup, before
/// any build can be active.
pub fn cleanup_staging_directories(runtimes_root: &Path) -> AppResult<usize> {
    visit_runtime_tree(runtimes_root, &mut |directory| {
        let is_staging = directory
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(STAGING_MARKER));

        if !is_staging {
            return Ok(false);
        }

        std::fs::remove_dir_all(directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!(
                    "Could not remove stale staging directory {}.",
                    directory.display()
                ),
            )
            .with_details(error.to_string())
        })?;
        Ok(true)
    })
}

/// Recovers completed snapshots that reached disk before an interrupted registry update.
pub fn discover_runtime_records(runtimes_root: &Path) -> Vec<RuntimeRecord> {
    let mut records = Vec::new();
    let _ = visit_runtime_tree(runtimes_root, &mut |directory| {
        let metadata_path = directory.join(RUNTIME_METADATA_FILE);
        if !metadata_path.is_file() {
            return Ok(false);
        }

        let Ok(contents) = std::fs::read(&metadata_path) else {
            return Ok(true);
        };
        let Ok(record) = serde_json::from_slice::<RuntimeRecord>(&contents) else {
            return Ok(true);
        };

        // Never trust paths embedded in a file more than its actual location. This also keeps a
        // copied metadata file from registering an unrelated directory.
        if record.directory == directory
            && record.executable == directory.join(SERVER_EXECUTABLE)
            && record.executable.is_file()
        {
            records.push(record);
        }

        Ok(true)
    });
    records
}

/// Visits directories depth-first. Returning `true` means the callback handled the directory and
/// recursion should stop there. The count is useful for startup cleanup diagnostics.
fn visit_runtime_tree(
    root: &Path,
    visitor: &mut impl FnMut(&Path) -> AppResult<bool>,
) -> AppResult<usize> {
    if !root.is_dir() {
        return Ok(0);
    }

    let mut handled = 0;
    let entries = std::fs::read_dir(root).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not inspect {}.", root.display()),
        )
        .with_details(error.to_string())
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not enumerate {}.", root.display()),
            )
            .with_details(error.to_string())
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if visitor(&path)? {
            handled += 1;
        } else {
            handled += visit_runtime_tree(&path, visitor)?;
        }
    }

    Ok(handled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_are_keyed_by_commit_and_backend() {
        let directory = snapshot_directory(
            Path::new("/data/runtimes"),
            "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
            "cuda",
            "runtime-1",
        );

        assert_eq!(
            directory,
            PathBuf::from("/data/runtimes/a1b2c3d4e5f6/cuda/runtime-1")
        );
    }

    #[test]
    fn different_backends_of_one_commit_do_not_collide() {
        let cuda = snapshot_directory(Path::new("/r"), "abc123456789", "cuda", "r1");
        let cpu = snapshot_directory(Path::new("/r"), "abc123456789", "cpu", "r1");

        assert_ne!(cuda, cpu);
    }

    #[test]
    fn repeated_builds_of_one_commit_get_distinct_directories() {
        let first = snapshot_directory(Path::new("/r"), "abc123456789", "cuda", "r1");
        let second = snapshot_directory(Path::new("/r"), "abc123456789", "cuda", "r2");

        assert_ne!(first, second);
    }

    #[test]
    fn locating_artifacts_prefers_the_first_matching_directory() {
        let temp = tempfile::tempdir().expect("temp dir");
        let release = temp.path().join("bin").join("Release");
        std::fs::create_dir_all(&release).expect("create");
        std::fs::write(release.join(SERVER_EXECUTABLE), b"binary").expect("write");

        let located =
            locate_artifacts(&[release.clone(), temp.path().join("bin")]).expect("should locate");

        assert_eq!(located, release);
    }

    #[test]
    fn a_missing_server_binary_is_reported_with_the_searched_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let error = locate_artifacts(&[temp.path().join("bin")]).expect_err("must fail");

        assert_eq!(error.code, ErrorCode::BuildArtifactMissing);
        assert!(error.details.expect("details").contains("bin"));
    }

    #[test]
    fn copying_takes_executables_and_libraries_but_not_build_leftovers() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("bin");
        std::fs::create_dir_all(&source).expect("create");

        std::fs::write(source.join(SERVER_EXECUTABLE), b"server").expect("write");
        std::fs::write(source.join("ggml.dll"), b"ggml").expect("write");
        std::fs::write(source.join("llama.lib"), b"import library").expect("write");
        std::fs::write(source.join("llama-server.pdb"), b"symbols").expect("write");

        let destination = temp.path().join("runtime");
        let stage = stage_runtime(&source, &destination).expect("stage artifacts");
        let copied = stage
            .commit(&serde_json::json!({ "runtime": "test" }))
            .expect("publish snapshot");

        assert!(destination.join(SERVER_EXECUTABLE).is_file());
        assert!(destination.join("ggml.dll").is_file());
        assert!(destination.join(RUNTIME_METADATA_FILE).is_file());
        assert!(!destination.join("llama.lib").exists());
        assert!(!destination.join("llama-server.pdb").exists());
        assert_eq!(copied.file_count, 2);
        assert!(copied.size_bytes > 0);
    }

    #[test]
    fn an_existing_snapshot_is_never_overwritten() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("bin");
        std::fs::create_dir_all(&source).expect("create");
        std::fs::write(source.join(SERVER_EXECUTABLE), b"server").expect("write");

        let destination = temp.path().join("runtime");
        stage_runtime(&source, &destination)
            .expect("first stage")
            .commit(&serde_json::json!({ "runtime": "original" }))
            .expect("first publish");

        let error = stage_runtime(&source, &destination).expect_err("second must fail");
        assert_eq!(error.code, ErrorCode::RuntimeExists);

        // The original snapshot survived untouched.
        assert!(destination.join(SERVER_EXECUTABLE).is_file());
    }

    #[test]
    fn copying_a_directory_without_the_server_binary_fails() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("bin");
        std::fs::create_dir_all(&source).expect("create");
        std::fs::write(source.join("ggml.dll"), b"ggml").expect("write");

        let destination = temp.path().join("runtime");
        let error = stage_runtime(&source, &destination).expect_err("must fail");
        assert_eq!(error.code, ErrorCode::BuildArtifactMissing);
        assert!(!destination.exists());
        assert_eq!(
            std::fs::read_dir(temp.path())
                .expect("read temp")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(STAGING_MARKER))
                .count(),
            0
        );
    }

    #[test]
    fn startup_cleanup_removes_only_staging_directories() {
        let temp = tempfile::tempdir().expect("temp dir");
        let backend = temp.path().join("abc123").join("cuda");
        let staging = backend.join(".runtime.staging-dead-build");
        let completed = backend.join("runtime-1");
        std::fs::create_dir_all(&staging).expect("create stale stage");
        std::fs::create_dir_all(&completed).expect("create completed runtime");

        let removed = cleanup_staging_directories(temp.path()).expect("cleanup succeeds");

        assert_eq!(removed, 1);
        assert!(!staging.exists());
        assert!(completed.exists());
    }

    #[test]
    fn completed_metadata_can_rebuild_a_missing_registry_entry() {
        let temp = tempfile::tempdir().expect("temp dir");
        let directory = temp.path().join("abc123").join("cuda").join("r1");
        std::fs::create_dir_all(&directory).expect("create runtime");
        std::fs::write(directory.join(SERVER_EXECUTABLE), b"server").expect("write server");

        let record = RuntimeRecord {
            id: "r1".into(),
            source_id: "s1".into(),
            source_name: "llama.cpp".into(),
            repository: "https://github.com/ggml-org/llama.cpp".into(),
            commit: "abc1230000000000000000000000000000000000".into(),
            short_commit: "abc1230".into(),
            branch: "master".into(),
            backend: crate::build::profile::BuildBackend::Cuda,
            configuration: crate::build::profile::BuildConfiguration::Release,
            generator: "Ninja".into(),
            build_date: "2026-08-25T12:00:00Z".into(),
            directory: directory.clone(),
            executable: directory.join(SERVER_EXECUTABLE),
            size_bytes: 6,
            file_count: 1,
            capabilities: None,
        };
        std::fs::write(
            directory.join(RUNTIME_METADATA_FILE),
            serde_json::to_vec_pretty(&record).expect("serialize metadata"),
        )
        .expect("write metadata");

        assert_eq!(discover_runtime_records(temp.path()), vec![record]);
    }
}
