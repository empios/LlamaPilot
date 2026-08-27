use std::time::{Duration, Instant};

use reqwest::{RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConnectionTestRequest {
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConnectionTest {
    pub api_base_url: String,
    pub model_ids: Vec<String>,
    pub selected_model: String,
    pub requested_model_matched: bool,
    pub response_text: String,
    pub models_latency_ms: f64,
    pub chat_latency_ms: f64,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelObject>,
}

#[derive(Debug, Deserialize)]
struct ModelObject {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

pub async fn test_openai_connection(
    server_base_url: &str,
    request: AgentConnectionTestRequest,
) -> AppResult<AgentConnectionTest> {
    let api_key = normalized_api_key(request.api_key)?;
    let requested_model = normalized_model(request.model)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| {
            AppError::internal("Could not initialize the agent connection client.")
                .with_details(error.to_string())
        })?;
    let api_base_url = format!("{}/v1", server_base_url.trim_end_matches('/'));

    let models_started = Instant::now();
    let response = authorize(
        client.get(format!("{api_base_url}/models")),
        api_key.as_deref(),
    )
    .send()
    .await
    .map_err(|error| connection_error("The model-list request did not complete.", error))?;
    let models_latency_ms = models_started.elapsed().as_secs_f64() * 1_000.0;
    let models = parse_success_response::<ModelsResponse>(response, "list models").await?;
    let model_ids: Vec<_> = models
        .data
        .into_iter()
        .map(|model| model.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let first_model = model_ids.first().cloned().ok_or_else(|| {
        AppError::new(
            ErrorCode::AgentConnectionFailed,
            "llama-server returned no models from /v1/models.",
        )
        .with_hint("Wait for the selected profile to become Ready, then retry.")
    })?;
    let requested_model_matched = requested_model
        .as_ref()
        .is_some_and(|requested| model_ids.iter().any(|id| id == requested));
    let selected_model = requested_model
        .filter(|requested| model_ids.iter().any(|id| id == requested))
        .unwrap_or(first_model);

    let chat_started = Instant::now();
    let response = authorize(
        client.post(format!("{api_base_url}/chat/completions")),
        api_key.as_deref(),
    )
    .json(&json!({
        "model": selected_model,
        "messages": [
            {
                "role": "user",
                "content": "Reply with only the word: connected"
            }
        ],
        "temperature": 0.0,
        "max_tokens": 16,
        "stream": false
    }))
    .send()
    .await
    .map_err(|error| connection_error("The chat-completions request did not complete.", error))?;
    let chat_latency_ms = chat_started.elapsed().as_secs_f64() * 1_000.0;
    let chat = parse_success_response::<ChatResponse>(response, "create a chat completion").await?;
    let response_text = chat
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::AgentConnectionFailed,
                "The OpenAI-compatible response contained no assistant message.",
            )
            .with_hint("Check that the model has a compatible chat template and retry the test.")
        })?;

    Ok(AgentConnectionTest {
        api_base_url,
        model_ids,
        selected_model,
        requested_model_matched,
        response_text,
        models_latency_ms,
        chat_latency_ms,
    })
}

fn authorize(builder: RequestBuilder, api_key: Option<&str>) -> RequestBuilder {
    match api_key {
        Some(api_key) => builder.bearer_auth(api_key),
        None => builder,
    }
}

async fn parse_success_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    action: &str,
) -> AppResult<T> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| connection_error("Could not read the llama-server response.", error))?;
    if !status.is_success() {
        let mut error = AppError::new(
            ErrorCode::AgentConnectionFailed,
            format!(
                "llama-server could not {action} (HTTP {}).",
                status.as_u16()
            ),
        )
        .with_details(body);
        if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            error = error.with_hint("Enter the API key configured for this server profile.");
        }
        return Err(error);
    }
    serde_json::from_str(&body).map_err(|error| {
        AppError::new(
            ErrorCode::AgentConnectionFailed,
            "llama-server returned a response that was not OpenAI-compatible.",
        )
        .with_details(format!("{error}\n\n{body}"))
    })
}

fn normalized_api_key(api_key: Option<String>) -> AppResult<Option<String>> {
    let api_key = api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if api_key.as_ref().is_some_and(|value| value.len() > 4_096) {
        return Err(AppError::new(
            ErrorCode::AgentConnectionFailed,
            "The API key is too long.",
        ));
    }
    Ok(api_key)
}

fn normalized_model(model: Option<String>) -> AppResult<Option<String>> {
    let model = model
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if model.as_ref().is_some_and(|value| value.len() > 1_024) {
        return Err(AppError::new(
            ErrorCode::AgentConnectionFailed,
            "The requested model identifier is too long.",
        ));
    }
    Ok(model)
}

fn connection_error(message: &str, error: reqwest::Error) -> AppError {
    AppError::new(ErrorCode::AgentConnectionFailed, message)
        .with_hint("Keep the selected profile Ready and verify its API key.")
        .with_details(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_optional_credentials_and_model_names() {
        assert_eq!(
            normalized_api_key(Some("  secret  ".into())).unwrap(),
            Some("secret".into())
        );
        assert_eq!(normalized_api_key(Some("  ".into())).unwrap(), None);
        assert_eq!(
            normalized_model(Some("  coder  ".into())).unwrap(),
            Some("coder".into())
        );
    }

    #[test]
    fn rejects_unbounded_agent_input() {
        assert!(normalized_api_key(Some("x".repeat(4_097))).is_err());
        assert!(normalized_model(Some("x".repeat(1_025))).is_err());
    }
}
