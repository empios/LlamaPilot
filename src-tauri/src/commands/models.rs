use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::models::{
    download_selection, inspect_hugging_face_repository as inspect_repository,
    inspect_repository_revision, validate_destination, HuggingFaceRepository, ModelCatalog,
    ModelDownloadEvent, ModelDownloadRequest, ProjectorSelection,
};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadOutcome {
    files: Vec<PathBuf>,
    catalog: ModelCatalog,
}

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

#[tauri::command]
pub async fn inspect_hugging_face_repository(
    repository: String,
) -> AppResult<HuggingFaceRepository> {
    inspect_repository(&repository).await
}

#[tauri::command]
pub async fn download_hugging_face_model(
    state: State<'_, AppState>,
    request: ModelDownloadRequest,
    on_event: Channel<ModelDownloadEvent>,
) -> AppResult<ModelDownloadOutcome> {
    let permit = state.model_downloads.begin()?;
    let roots = state.settings.get().workspace.model_directories;
    let requested_destination = request.destination_directory.clone();
    let destination = tauri::async_runtime::spawn_blocking(move || {
        validate_destination(&requested_destination, &roots)
    })
    .await
    .map_err(|error| {
        AppError::internal("Validating the model destination stopped unexpectedly.")
            .with_details(error.to_string())
    })??;
    let repository = inspect_repository_revision(&request.repository_id, &request.revision).await?;
    let files = download_selection(
        &repository,
        &request.selection_id,
        &destination,
        &permit,
        |event| {
            let _ = on_event.send(event);
        },
    )
    .await?;

    let service = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let catalog = tauri::async_runtime::spawn_blocking(move || service.scan(&roots))
        .await
        .map_err(|error| {
            AppError::new(
                ErrorCode::ModelScanFailed,
                "The model was downloaded, but the catalog refresh stopped unexpectedly.",
            )
            .with_details(error.to_string())
        })??;
    Ok(ModelDownloadOutcome { files, catalog })
}

#[tauri::command]
pub fn cancel_model_download(state: State<'_, AppState>) -> bool {
    state.model_downloads.cancel()
}

#[tauri::command]
pub fn is_model_download_running(state: State<'_, AppState>) -> bool {
    state.model_downloads.is_running()
}
