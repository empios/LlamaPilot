use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult, ErrorCode};

/// Folder created inside the user's home directory for large, user-owned artifacts.
///
/// Kept separate from the product name so renaming the app does not move a user's checkouts.
const WORKSPACE_DIR_NAME: &str = "LlamaControl";

/// Resolved on-disk layout for everything the application owns.
///
/// Small state lives under the Tauri app-data directory. Git checkouts and build trees are large
/// and live in a user-configurable workspace directory instead.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub settings_file: PathBuf,
    pub sources_dir: PathBuf,
    pub sources_metadata_file: PathBuf,
    pub builds_dir: PathBuf,
    pub builds_metadata_file: PathBuf,
    pub profiles_dir: PathBuf,
    pub runtimes_dir: PathBuf,
    pub runtimes_metadata_file: PathBuf,
    pub models_dir: PathBuf,
    pub models_metadata_file: PathBuf,
    pub cache_dir: PathBuf,
    pub model_metadata_cache_file: PathBuf,
    pub logs_dir: PathBuf,
    pub default_workspace_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> AppResult<Self> {
        let data_dir = app.path().app_data_dir().map_err(|error| {
            AppError::new(
                ErrorCode::Config,
                "Could not determine the application data directory.",
            )
            .with_details(error.to_string())
        })?;

        let home_dir = app
            .path()
            .home_dir()
            .unwrap_or_else(|_| data_dir.join("workspace"));

        Ok(Self::from_roots(
            data_dir,
            home_dir.join(WORKSPACE_DIR_NAME),
        ))
    }

    pub fn from_roots(data_dir: PathBuf, default_workspace_dir: PathBuf) -> Self {
        Self {
            settings_file: data_dir.join("settings.json"),
            sources_metadata_file: data_dir.join("sources").join("metadata.json"),
            sources_dir: data_dir.join("sources"),
            builds_metadata_file: data_dir.join("builds").join("metadata.json"),
            builds_dir: data_dir.join("builds"),
            profiles_dir: data_dir.join("profiles"),
            runtimes_metadata_file: data_dir.join("runtimes").join("metadata.json"),
            runtimes_dir: data_dir.join("runtimes"),
            models_metadata_file: data_dir.join("models").join("metadata.json"),
            models_dir: data_dir.join("models"),
            model_metadata_cache_file: data_dir.join("cache").join("model-metadata.json"),
            cache_dir: data_dir.join("cache"),
            logs_dir: data_dir.join("logs"),
            data_dir,
            default_workspace_dir,
        }
    }

    /// Creates every directory the application writes to. Safe to call repeatedly.
    pub fn ensure_directories(&self) -> AppResult<()> {
        for directory in [
            &self.data_dir,
            &self.sources_dir,
            &self.builds_dir,
            &self.profiles_dir,
            &self.runtimes_dir,
            &self.models_dir,
            &self.cache_dir,
            &self.logs_dir,
        ] {
            std::fs::create_dir_all(directory).map_err(|error| {
                AppError::new(
                    ErrorCode::Io,
                    format!("Could not create {}.", directory.display()),
                )
                .with_details(error.to_string())
            })?;
        }
        Ok(())
    }

    pub fn default_sources_workspace(&self) -> PathBuf {
        self.default_workspace_dir.join("sources")
    }

    pub fn default_builds_workspace(&self) -> PathBuf {
        self.default_workspace_dir.join("builds")
    }
}

/// Rejects paths that are empty, relative, or contain parent traversal segments.
///
/// Every path that crosses the IPC boundary goes through this before being handed to a process.
pub fn validate_absolute_path(candidate: &Path, label: &str) -> AppResult<PathBuf> {
    if candidate.as_os_str().is_empty() {
        return Err(AppError::invalid_path(format!("{label} is required.")));
    }

    if !candidate.is_absolute() {
        return Err(
            AppError::invalid_path(format!("{label} must be an absolute path."))
                .with_details(candidate.display().to_string()),
        );
    }

    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(
            AppError::invalid_path(format!("{label} must not contain \"..\" segments."))
                .with_details(candidate.display().to_string()),
        );
    }

    Ok(candidate.to_path_buf())
}

pub fn require_existing_directory(candidate: &Path, label: &str) -> AppResult<PathBuf> {
    let path = validate_absolute_path(candidate, label)?;
    if !path.is_dir() {
        return Err(AppError::invalid_path(format!(
            "{label} does not exist or is not a directory."
        ))
        .with_details(path.display().to_string()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_derived_from_the_data_directory() {
        let paths = AppPaths::from_roots(PathBuf::from("/data"), PathBuf::from("/home/ws"));

        assert_eq!(paths.settings_file, PathBuf::from("/data/settings.json"));
        assert_eq!(
            paths.sources_metadata_file,
            PathBuf::from("/data/sources/metadata.json")
        );
        assert_eq!(paths.runtimes_dir, PathBuf::from("/data/runtimes"));
        assert_eq!(
            paths.model_metadata_cache_file,
            PathBuf::from("/data/cache/model-metadata.json")
        );
        assert_eq!(
            paths.default_sources_workspace(),
            PathBuf::from("/home/ws/sources")
        );
    }

    #[test]
    fn relative_paths_are_rejected() {
        let error = validate_absolute_path(Path::new("relative/dir"), "Destination")
            .expect_err("relative paths must fail");
        assert_eq!(error.code, ErrorCode::InvalidPath);
    }

    #[test]
    fn parent_traversal_is_rejected() {
        let candidate = if cfg!(windows) {
            Path::new(r"C:\repos\..\windows")
        } else {
            Path::new("/repos/../etc")
        };

        let error =
            validate_absolute_path(candidate, "Destination").expect_err("traversal must fail");
        assert_eq!(error.code, ErrorCode::InvalidPath);
    }

    #[test]
    fn absolute_paths_are_accepted() {
        let candidate = if cfg!(windows) {
            Path::new(r"C:\repos\llama.cpp")
        } else {
            Path::new("/repos/llama.cpp")
        };

        assert!(validate_absolute_path(candidate, "Destination").is_ok());
    }
}
