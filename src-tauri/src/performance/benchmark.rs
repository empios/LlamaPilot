use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkMeasurement {
    pub latency_ms: f64,
    pub prompt_tokens: u64,
    pub predicted_tokens: u64,
    pub prompt_ms: f64,
    pub predicted_ms: f64,
    pub prompt_tokens_per_second: f64,
    pub predicted_tokens_per_second: f64,
    pub truncated: bool,
    pub stop_type: Option<String>,
    pub draft_tokens: Option<u64>,
    pub accepted_draft_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    timings: Option<CompletionTimings>,
    #[serde(default)]
    truncated: bool,
    stop_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompletionTimings {
    #[serde(default)]
    prompt_n: u64,
    #[serde(default)]
    predicted_n: u64,
    #[serde(default)]
    prompt_ms: f64,
    #[serde(default)]
    predicted_ms: f64,
    #[serde(default)]
    prompt_per_second: f64,
    #[serde(default)]
    predicted_per_second: f64,
    draft_n: Option<u64>,
    draft_n_accepted: Option<u64>,
}

pub async fn run(base_url: &str) -> AppResult<BenchmarkMeasurement> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| {
            AppError::internal("Could not initialize the performance benchmark client.")
                .with_details(error.to_string())
        })?;
    let started = Instant::now();
    let response = client
        .post(format!("{base_url}/completion"))
        .json(&json!({
            "prompt": coding_prompt(),
            "n_predict": 128,
            "temperature": 0.0,
            "seed": 42,
            "cache_prompt": false,
            "stream": false,
        }))
        .send()
        .await
        .map_err(|error| benchmark_error("The benchmark request did not complete.", error))?;
    let latency_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| benchmark_error("Could not read the benchmark response.", error))?;
    if !status.is_success() {
        return Err(AppError::new(
            ErrorCode::BenchmarkFailed,
            format!(
                "llama-server rejected the benchmark with HTTP {}.",
                status.as_u16()
            ),
        )
        .with_hint(
            "Keep the selected profile ready and make sure no API key blocks local requests.",
        )
        .with_details(body));
    }

    parse_response(&body, latency_ms)
}

pub fn local_base_url(host: &str, port: u16) -> String {
    let host = match host {
        "0.0.0.0" | "*" => "127.0.0.1".to_string(),
        "::" | "[::]" => "[::1]".to_string(),
        value if value.contains(':') && !value.starts_with('[') => format!("[{value}]"),
        value => value.to_string(),
    };
    format!("http://{host}:{port}")
}

fn parse_response(body: &str, latency_ms: f64) -> AppResult<BenchmarkMeasurement> {
    let response: CompletionResponse = serde_json::from_str(body).map_err(|error| {
        AppError::new(
            ErrorCode::BenchmarkFailed,
            "llama-server returned an unreadable benchmark response.",
        )
        .with_details(format!("{error}\n\n{body}"))
    })?;
    let timings = response.timings.ok_or_else(|| {
        AppError::new(
            ErrorCode::BenchmarkFailed,
            "This llama-server response did not include performance timings.",
        )
        .with_hint("Use a current llama.cpp runtime and inspect it again before benchmarking.")
        .with_details(body.to_string())
    })?;
    if timings.prompt_n == 0 || timings.predicted_n == 0 {
        return Err(AppError::new(
            ErrorCode::BenchmarkFailed,
            "The benchmark completed without processing both prompt and generated tokens.",
        )
        .with_details(body.to_string()));
    }

    let prompt_tokens_per_second = positive_rate(
        timings.prompt_per_second,
        timings.prompt_n,
        timings.prompt_ms,
    );
    let predicted_tokens_per_second = positive_rate(
        timings.predicted_per_second,
        timings.predicted_n,
        timings.predicted_ms,
    );

    Ok(BenchmarkMeasurement {
        latency_ms,
        prompt_tokens: timings.prompt_n,
        predicted_tokens: timings.predicted_n,
        prompt_ms: timings.prompt_ms,
        predicted_ms: timings.predicted_ms,
        prompt_tokens_per_second,
        predicted_tokens_per_second,
        truncated: response.truncated,
        stop_type: response.stop_type,
        draft_tokens: timings.draft_n,
        accepted_draft_tokens: timings.draft_n_accepted,
    })
}

fn positive_rate(reported: f64, tokens: u64, milliseconds: f64) -> f64 {
    if reported.is_finite() && reported > 0.0 {
        reported
    } else if milliseconds.is_finite() && milliseconds > 0.0 {
        tokens as f64 / (milliseconds / 1_000.0)
    } else {
        0.0
    }
}

fn benchmark_error(message: &str, error: reqwest::Error) -> AppError {
    AppError::new(ErrorCode::BenchmarkFailed, message)
        .with_hint("Keep the selected profile running and wait until it is Ready.")
        .with_details(error.to_string())
}

fn coding_prompt() -> String {
    const SAMPLE: &str = r#"
fn aggregate_requests(requests: &[Request]) -> Summary {
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut elapsed_ms = 0u64;
    for request in requests {
        match request.status {
            Status::Completed { duration_ms } => {
                completed += 1;
                elapsed_ms += duration_ms;
            }
            Status::Failed => failed += 1,
            Status::Pending => {}
        }
    }
    Summary { completed, failed, elapsed_ms }
}
"#;
    let mut prompt = String::from(
        "You are reviewing a Rust service. Analyze the repeated sample below, identify performance and correctness risks, then propose a concise refactor.\n",
    );
    for _ in 0..24 {
        prompt.push_str(SAMPLE);
    }
    prompt
        .push_str("\nReturn the review as five numbered findings followed by revised Rust code.\n");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_llama_server_timings() {
        let measurement = parse_response(
            r#"{
                "truncated": false,
                "stop_type": "limit",
                "timings": {
                    "prompt_n": 1024,
                    "prompt_ms": 2048.0,
                    "prompt_per_second": 500.0,
                    "predicted_n": 128,
                    "predicted_ms": 3200.0,
                    "predicted_per_second": 40.0,
                    "draft_n": 200,
                    "draft_n_accepted": 120
                }
            }"#,
            5_500.0,
        )
        .expect("valid response");

        assert_eq!(measurement.prompt_tokens, 1024);
        assert_eq!(measurement.predicted_tokens_per_second, 40.0);
        assert_eq!(measurement.accepted_draft_tokens, Some(120));
        assert_eq!(measurement.stop_type.as_deref(), Some("limit"));
    }

    #[test]
    fn derives_rates_when_the_server_omits_precomputed_values() {
        let measurement = parse_response(
            r#"{
                "timings": {
                    "prompt_n": 100,
                    "prompt_ms": 500.0,
                    "predicted_n": 20,
                    "predicted_ms": 1000.0
                }
            }"#,
            1_500.0,
        )
        .expect("valid response");

        assert_eq!(measurement.prompt_tokens_per_second, 200.0);
        assert_eq!(measurement.predicted_tokens_per_second, 20.0);
    }

    #[test]
    fn requires_timing_data_and_non_empty_work() {
        assert_eq!(
            parse_response(r#"{"content":"result"}"#, 1.0)
                .expect_err("missing timings")
                .code,
            ErrorCode::BenchmarkFailed
        );
        assert!(coding_prompt().len() > 4_000);
    }

    #[test]
    fn normalizes_wildcard_and_ipv6_hosts_for_local_requests() {
        assert_eq!(local_base_url("0.0.0.0", 8080), "http://127.0.0.1:8080");
        assert_eq!(local_base_url("::", 8080), "http://[::1]:8080");
    }
}
