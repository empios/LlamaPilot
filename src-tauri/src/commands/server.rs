use std::time::Duration;

use tauri::ipc::Channel;
use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::profiles::build_profile_command;
use crate::server::{ServerEvent, ServerLaunch, ServerLogsSnapshot, ServerSnapshot};
use crate::state::AppState;

use super::profiles::prepare_profile;

#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> AppResult<ServerSnapshot> {
    Ok(state.server.snapshot().await)
}

#[tauri::command]
pub fn get_server_logs(state: State<'_, AppState>) -> ServerLogsSnapshot {
    state.server.logs_snapshot()
}

#[tauri::command]
pub async fn subscribe_server_events(
    state: State<'_, AppState>,
    on_event: Channel<ServerEvent>,
) -> AppResult<String> {
    Ok(state.server.subscribe(on_event).await)
}

#[tauri::command]
pub fn unsubscribe_server_events(state: State<'_, AppState>, id: String) -> bool {
    state.server.unsubscribe(&id)
}

#[tauri::command]
pub fn clear_server_logs(state: State<'_, AppState>) {
    state.server.clear_logs();
}

#[tauri::command]
pub async fn start_server(
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<ServerSnapshot> {
    if state.performance_sweeps.is_running() {
        return Err(crate::performance::sweep_in_progress_error());
    }
    launch_profile(&state, profile_id).await
}

#[tauri::command]
pub async fn stop_server(state: State<'_, AppState>) -> AppResult<ServerSnapshot> {
    state.server.stop().await
}

#[tauri::command]
pub async fn restart_server(state: State<'_, AppState>) -> AppResult<ServerSnapshot> {
    if state.performance_sweeps.is_running() {
        return Err(crate::performance::sweep_in_progress_error());
    }
    let snapshot = state.server.snapshot().await;
    let profile_id = snapshot.profile_id.ok_or_else(|| {
        AppError::new(
            ErrorCode::ServerNotRunning,
            "There is no previously selected profile to restart.",
        )
    })?;
    if snapshot.state.is_active() {
        state.server.stop().await?;
        state
            .server
            .wait_until_stopped(Duration::from_secs(5))
            .await?;
    }
    launch_profile(&state, profile_id).await
}

pub(crate) async fn launch_profile(
    state: &AppState,
    profile_id: String,
) -> AppResult<ServerSnapshot> {
    let profile = state.profiles.find(&profile_id)?;
    let port = state
        .server
        .select_port(&profile.host, profile.port, profile.auto_select_port)
        .await?;

    let mut input = profile.input();
    input.port = port;
    // Port selection has already happened. Suppress the preview-only warning and execute this exact
    // resolved port through the structured argument array.
    input.auto_select_port = false;
    let existing = profile.clone();
    let models = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let runtimes = state.runtimes.get();
    let generated = tauri::async_runtime::spawn_blocking(move || {
        let prepared = prepare_profile(input, Some(&existing), &runtimes, &models.scan(&roots)?)?;
        let generated = build_profile_command(
            prepared.input,
            &prepared.runtime,
            &prepared.capabilities,
            &prepared.target,
        )?;
        Ok::<_, AppError>((prepared.runtime, prepared.target, generated))
    })
    .await
    .map_err(|error| {
        AppError::internal("Preparing the server process stopped unexpectedly.")
            .with_details(error.to_string())
    })??;
    let (runtime, target, generated) = generated;

    state
        .server
        .start(ServerLaunch {
            profile_id: profile.id,
            profile_name: profile.name,
            runtime_id: runtime.id,
            runtime_label: target.runtime_label,
            model_name: target.model_name,
            host: profile.host,
            port,
            command: generated.spec,
            warnings: generated.preview.warnings,
        })
        .await
}
