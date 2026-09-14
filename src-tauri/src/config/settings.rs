use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::paths::AppPaths;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub auto_download: bool,
    pub auto_install: bool,
}
impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_download: false,
            auto_install: false,
        }
    }
}

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

pub const DEFAULT_LLAMA_REPOSITORY: &str = "https://github.com/ggml-org/llama.cpp";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    pub theme: ThemePreference,
    pub compact_density: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            compact_density: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkspaceSettings {
    /// Where new llama.cpp clones are placed. Empty means "use the built-in default".
    pub sources_directory: Option<PathBuf>,
    /// Where CMake build trees are placed. Empty means "use the built-in default".
    pub builds_directory: Option<PathBuf>,
    pub model_directories: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitSettings {
    pub default_repository: String,
    /// Explicit path to `git`. When absent the executable is resolved from PATH.
    pub executable: Option<PathBuf>,
}

impl Default for GitSettings {
    fn default() -> Self {
        Self {
            default_repository: DEFAULT_LLAMA_REPOSITORY.to_string(),
            executable: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildSettings {
    /// Explicit path to `cmake`. When absent the executable is resolved from PATH.
    pub cmake_executable: Option<PathBuf>,
    /// `None` lets CMake decide the job count.
    pub parallel_jobs: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerSettings {
    pub default_host: String,
    pub default_port: u16,
    pub auto_select_port: bool,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            default_host: "127.0.0.1".to_string(),
            default_port: 8080,
            auto_select_port: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub updates: UpdateSettings,
    pub schema_version: u32,
    pub appearance: AppearanceSettings,
    pub workspace: WorkspaceSettings,
    pub git: GitSettings,
    pub build: BuildSettings,
    pub server: ServerSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            updates: UpdateSettings::default(),
            appearance: AppearanceSettings::default(),
            workspace: WorkspaceSettings::default(),
            git: GitSettings::default(),
            build: BuildSettings::default(),
            server: ServerSettings::default(),
        }
    }
}

impl Settings {
    pub fn sources_directory(&self, paths: &AppPaths) -> PathBuf {
        self.workspace
            .sources_directory
            .clone()
            .unwrap_or_else(|| paths.default_sources_workspace())
    }

    pub fn builds_directory(&self, paths: &AppPaths) -> PathBuf {
        self.workspace
            .builds_directory
            .clone()
            .unwrap_or_else(|| paths.default_builds_workspace())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_document_deserializes_to_defaults() {
        let settings: Settings = serde_json::from_str("{}").expect("defaults apply");
        assert_eq!(settings, Settings::default());
        assert_eq!(settings.git.default_repository, DEFAULT_LLAMA_REPOSITORY);
        assert_eq!(settings.server.default_port, 8080);
    }

    #[test]
    fn unknown_sections_do_not_clobber_known_ones() {
        let json = r#"{ "server": { "defaultPort": 9090 } }"#;
        let settings: Settings = serde_json::from_str(json).expect("partial document");

        assert_eq!(settings.server.default_port, 9090);
        assert_eq!(settings.server.default_host, "127.0.0.1");
        assert_eq!(settings.appearance.theme, ThemePreference::System);
    }

    #[test]
    fn workspace_directories_fall_back_to_app_defaults() {
        let paths = AppPaths::from_roots(PathBuf::from("/data"), PathBuf::from("/home/ws"));
        let settings = Settings::default();

        assert_eq!(
            settings.sources_directory(&paths),
            PathBuf::from("/home/ws/sources")
        );
    }

    #[test]
    fn explicit_workspace_directories_win() {
        let paths = AppPaths::from_roots(PathBuf::from("/data"), PathBuf::from("/home/ws"));
        let mut settings = Settings::default();
        settings.workspace.sources_directory = Some(PathBuf::from("/elsewhere"));

        assert_eq!(
            settings.sources_directory(&paths),
            PathBuf::from("/elsewhere")
        );
    }
}
