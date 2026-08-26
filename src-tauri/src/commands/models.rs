use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::models::{ModelCatalog, ProjectorSelection};
use crate::state::AppState;

#[tauri::command]
pub async fn scan_models(state: State<'_, AppState>) -> AppResult<ModelCatalog> {
    let service = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    tauri::async_runtime::spawn_blocking(move || service.scan(&roots))
        .await
        .map_err(|error| {
            AppError::new(
                ErrorCode::ModelScanFailed,
                "The model scan stopped unexpectedly.",
            )
            .with_details(error.to_string())
        })?
}

#[tauri::command]
pub async fn set_model_projector(
    state: State<'_, AppState>,
    model_id: String,
    selection: ProjectorSelection,
) -> AppResult<()> {
    let service = state.models.clone();
    tauri::async_runtime::spawn_blocking(move || service.set_projector(model_id, selection))
        .await
        .map_err(|error| {
            AppError::new(
                ErrorCode::ModelScanFailed,
                "The projector choice could not be saved.",
            )
            .with_details(error.to_string())
        })?
}
