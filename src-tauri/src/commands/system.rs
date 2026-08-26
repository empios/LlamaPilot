use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::config::paths::validate_absolute_path;
use crate::config::AppPaths;
use crate::error::AppResult;
use crate::hardware::HardwareSnapshot;
use crate::platform;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub paths: AppPaths,
    pub platform: String,
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        paths: state.paths.clone(),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
pub async fn get_hardware_snapshot() -> HardwareSnapshot {
    crate::hardware::snapshot().await
}

#[tauri::command]
pub async fn get_git_version(state: State<'_, AppState>) -> AppResult<String> {
    state.git().version().await
}

#[tauri::command]
pub fn reveal_path(path: PathBuf) -> AppResult<()> {
    let path = validate_absolute_path(&path, "The path")?;
    platform::reveal_in_file_manager(&path)
}
