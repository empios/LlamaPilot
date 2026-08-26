use std::path::PathBuf;

use tauri::ipc::Channel;
use tauri::State;

use crate::error::AppResult;
use crate::git::{GitRemote, UpdateOutcome};
use crate::sources::service::{self, CloneRequest, SwitchRefOutcome};
use crate::sources::{LlamaSource, SourceRefs, SourceStatus};
use crate::state::AppState;

use super::progress::{self, ProgressEvent};

#[tauri::command]
pub fn list_sources(state: State<'_, AppState>) -> Vec<LlamaSource> {
    service::list(&state)
}

#[tauri::command]
pub async fn add_existing_source(
    state: State<'_, AppState>,
    directory: PathBuf,
    name: Option<String>,
) -> AppResult<LlamaSource> {
    service::add_existing(&state, &directory, name).await
}

#[tauri::command]
pub async fn clone_source(
    state: State<'_, AppState>,
    request: CloneRequest,
    on_progress: Channel<ProgressEvent>,
) -> AppResult<LlamaSource> {
    let sink = progress::forward_to_channel(on_progress.clone(), "Cloning repository");
    let outcome = service::clone(&state, request, sink).await;

    progress::finish(&on_progress, outcome.is_ok());
    outcome
}

#[tauri::command]
pub fn remove_source(
    state: State<'_, AppState>,
    id: String,
    delete_directory: bool,
) -> AppResult<()> {
    service::remove(&state, &id, delete_directory)
}

#[tauri::command]
pub async fn get_source_status(state: State<'_, AppState>, id: String) -> AppResult<SourceStatus> {
    service::status(&state, &id).await
}

#[tauri::command]
pub async fn fetch_source(
    state: State<'_, AppState>,
    id: String,
    remote: Option<String>,
    on_progress: Channel<ProgressEvent>,
) -> AppResult<()> {
    let sink = progress::forward_to_channel(on_progress.clone(), "Fetching from remote");
    let outcome = service::fetch(&state, &id, remote.as_deref(), sink).await;

    progress::finish(&on_progress, outcome.is_ok());
    outcome
}

#[tauri::command]
pub async fn update_source(state: State<'_, AppState>, id: String) -> AppResult<UpdateOutcome> {
    service::update(&state, &id).await
}

#[tauri::command]
pub async fn list_source_refs(state: State<'_, AppState>, id: String) -> AppResult<SourceRefs> {
    service::refs(&state, &id).await
}

#[tauri::command]
pub async fn switch_source_ref(
    state: State<'_, AppState>,
    id: String,
    target: String,
) -> AppResult<SwitchRefOutcome> {
    service::switch_ref(&state, &id, &target).await
}

#[tauri::command]
pub async fn list_source_remotes(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<GitRemote>> {
    service::remotes(&state, &id).await
}

#[tauri::command]
pub async fn add_source_remote(
    state: State<'_, AppState>,
    id: String,
    name: String,
    url: String,
) -> AppResult<Vec<GitRemote>> {
    service::add_remote(&state, &id, &name, &url).await
}

#[tauri::command]
pub async fn remove_source_remote(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<Vec<GitRemote>> {
    service::remove_remote(&state, &id, &name).await
}

#[tauri::command]
pub async fn discover_default_branch(
    state: State<'_, AppState>,
    repository: String,
) -> AppResult<Option<String>> {
    crate::git::discover_default_branch(&state.git(), &repository).await
}
