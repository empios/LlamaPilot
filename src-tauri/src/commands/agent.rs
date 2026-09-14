use tauri::State;

use crate::agent::{self, AgentConnectionTest, AgentConnectionTestRequest};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::performance::local_base_url;
use crate::server::ServerLifecycleState;
use crate::state::AppState;

#[tauri::command]
pub async fn test_agent_connection(
    state: State<'_, AppState>,
    request: AgentConnectionTestRequest,
) -> AppResult<AgentConnectionTest> {
    let _work = state.work.enter()?;
    let snapshot = state.server.snapshot().await;
    if snapshot.state != ServerLifecycleState::Ready {
        return Err(AppError::new(
            ErrorCode::AgentConnectionFailed,
            "The selected llama-server profile must be Ready before testing an agent connection.",
        )
        .with_hint("Start the profile and wait until model loading finishes."));
    }
    let host = snapshot.host.as_deref().ok_or_else(|| {
        AppError::internal("The active server has no host for the agent connection test.")
    })?;
    let port = snapshot.port.ok_or_else(|| {
        AppError::internal("The active server has no port for the agent connection test.")
    })?;
    agent::test_openai_connection(&local_base_url(host, port), request).await
}
