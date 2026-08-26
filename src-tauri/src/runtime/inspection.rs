use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::{artifacts, discovery, RuntimeInspection};
use crate::state::AppState;

use super::{snapshot, RuntimeRecord};

pub fn load(state: &AppState, id: &str) -> AppResult<RuntimeInspection> {
    let runtime = require_runtime(state, id)?;
    let summary = runtime.capabilities.as_ref().ok_or_else(|| {
        AppError::new(
            ErrorCode::CapabilityDiscoveryFailed,
            "This runtime has not been inspected yet.",
        )
        .with_hint("Run capability inspection for this runtime first.")
    })?;
    artifacts::load(&runtime.directory, &summary.inspection_id)
}

pub async fn refresh(state: &AppState, id: &str) -> AppResult<RuntimeInspection> {
    let runtime = require_runtime(state, id)?;
    let inspection = discovery::inspect(&runtime.executable, None).await?;
    let summary = artifacts::persist(&runtime.directory, &inspection)?;
    let new_inspection_id = summary.inspection_id.clone();
    let previous_inspection_id = runtime
        .capabilities
        .as_ref()
        .map(|previous| previous.inspection_id.clone());

    let mut updated = runtime;
    updated.capabilities = Some(summary);
    if let Err(error) = snapshot::write_runtime_metadata(&updated) {
        artifacts::remove(&updated.directory, &new_inspection_id);
        return Err(error);
    }

    // Snapshot metadata is written before the central registry. If the registry write is
    // interrupted, startup reconciliation repairs it from metadata.json.
    let (_, replaced) = state
        .runtimes
        .update(|registry| registry.replace(updated.clone()))?;
    replaced?;

    if let Some(previous) = previous_inspection_id {
        if previous != new_inspection_id {
            artifacts::remove(&updated.directory, &previous);
        }
    }

    Ok(inspection)
}

fn require_runtime(state: &AppState, id: &str) -> AppResult<RuntimeRecord> {
    state.runtimes.get().find(id).cloned().ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeNotFound,
            "That runtime is no longer registered.",
        )
        .with_details(id.to_string())
    })
}
