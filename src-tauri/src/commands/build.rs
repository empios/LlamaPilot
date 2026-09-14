use tauri::ipc::Channel;
use tauri::State;

use crate::build::service::{self, BuildOutcome, BuildRequest};
use crate::build::{detect, Toolchain};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::RuntimeInspection;
use crate::runtime::{inspection, RuntimeRecord};
use crate::state::AppState;

use super::progress::{self, ProgressEvent};

#[tauri::command]
pub async fn detect_toolchain(state: State<'_, AppState>) -> AppResult<Toolchain> {
    let _work = state.work.enter()?;
    Ok(detect::detect(&state.settings.get()).await)
}

#[tauri::command]
pub async fn build_runtime(
    state: State<'_, AppState>,
    request: BuildRequest,
    on_progress: Channel<ProgressEvent>,
) -> AppResult<BuildOutcome> {
    let _work = state.work.enter()?;
    let sink = progress::forward_to_channel(on_progress.clone(), "Building llama-server");
    let outcome = service::build(&state, request, sink).await;

    progress::finish(&on_progress, outcome.is_ok());
    outcome
}

#[tauri::command]
pub fn cancel_build(state: State<'_, AppState>) -> bool {
    state.builds.cancel()
}

#[tauri::command]
pub fn is_build_running(state: State<'_, AppState>) -> bool {
    state.builds.is_running()
}

#[tauri::command]
pub fn list_runtimes(state: State<'_, AppState>) -> Vec<RuntimeRecord> {
    state.runtimes.get().sorted()
}

#[tauri::command]
pub fn get_runtime_capabilities(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<RuntimeInspection> {
    let _work = state.work.enter()?;
    inspection::load(&state, &id)
}

#[tauri::command]
pub async fn inspect_runtime_capabilities(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<RuntimeInspection> {
    let _work = state.work.enter()?;
    inspection::refresh(&state, &id).await
}

/// Removes a runtime snapshot and its files.
///
/// This is the only operation that deletes a runtime, and it is always explicit — a build never
/// removes one.
#[tauri::command]
pub async fn delete_runtime(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let _work = state.work.enter()?;
    let snapshot = state.server.snapshot().await;
    if snapshot.state.is_active() && snapshot.runtime_id.as_deref() == Some(&id) {
        return Err(AppError::new(
            ErrorCode::ServerAlreadyRunning,
            "The runtime used by the active server cannot be deleted.",
        )
        .with_hint("Stop llama-server, then delete the runtime."));
    }
    let runtime = state.runtimes.get().find(&id).cloned().ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeNotFound,
            "That runtime is no longer registered.",
        )
        .with_details(id.clone())
    })?;

    // Delete the files first. If Windows refuses because llama-server is still running, the
    // registry entry remains visible and the user can stop it and retry. JsonStore likewise
    // keeps the record if the later metadata write fails.
    if runtime.directory.is_dir() {
        std::fs::remove_dir_all(&runtime.directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not delete {}.", runtime.directory.display()),
            )
            .with_hint("It may still be running. Stop it and try again.")
            .with_details(error.to_string())
        })?;
    }

    let (_, removed) = state.runtimes.update(|registry| registry.remove(&id))?;
    removed?;

    prune_empty_runtime_parents(&runtime.directory, &state.paths.runtimes_dir);
    Ok(())
}

fn prune_empty_runtime_parents(directory: &std::path::Path, runtimes_root: &std::path::Path) {
    let mut current = directory.parent();
    while let Some(parent) = current {
        if parent == runtimes_root || !parent.starts_with(runtimes_root) {
            break;
        }

        if std::fs::remove_dir(parent).is_err() {
            break;
        }
        current = parent.parent();
    }
}
