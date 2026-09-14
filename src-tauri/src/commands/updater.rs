use crate::{
    error::AppResult,
    state::AppState,
    updater::{persistent_work_active, UpdateStatus},
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_app_update_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<UpdateStatus> {
    let mut status = state.updates.snapshot(&app);
    status.busy = state.work.busy() || persistent_work_active(&state).await;
    Ok(status)
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.updates.check(&app).await
}

#[tauri::command]
pub async fn download_app_update(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.updates.download(&app).await
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
    automatic: bool,
) -> AppResult<()> {
    state.updates.install(&app, &state, automatic).await
}

#[tauri::command]
pub fn defer_app_update(app: AppHandle, state: State<'_, AppState>) {
    state.updates.defer(&app);
}
