use serde_json::Value;

use super::model::ServerTelemetry;

#[derive(Debug)]
pub struct ProbeResult {
    pub health_status: Option<u16>,
    pub health_message: Option<String>,
    pub health_ready: bool,
    pub health_loading: bool,
    pub telemetry: ServerTelemetry,
}

pub async fn probe(client: &reqwest::Client, base_url: &str, include_details: bool) -> ProbeResult {
    let mut result = ProbeResult {
        health_status: None,
        health_message: None,
        health_ready: false,
        health_loading: false,
        telemetry: ServerTelemetry::default(),
    };

    match client.get(format!("{base_url}/health")).send().await {
        Ok(response) => {
            result.health_status = Some(response.status().as_u16());
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            result.health_ready = status.is_success()
                && serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|value| value.get("status")?.as_str().map(str::to_string))
                    .is_some_and(|value| value.eq_ignore_ascii_case("ok"));
            result.health_loading =
                status.as_u16() == 503 && body.to_ascii_lowercase().contains("loading model");
            if !result.health_ready && !result.health_loading && !body.trim().is_empty() {
                result.health_message = extract_error_message(&body).or(Some(body));
            }
        }
        Err(error) => {
            result.health_message = Some(error.to_string());
            return result;
        }
    }

    if !include_details || !result.health_ready {
        return result;
    }

    probe_props(client, base_url, &mut result.telemetry).await;
    probe_slots(client, base_url, &mut result.telemetry).await;
    probe_metrics(client, base_url, &mut result.telemetry).await;
    result
}

async fn probe_props(client: &reqwest::Client, base_url: &str, telemetry: &mut ServerTelemetry) {
    let Ok(response) = client.get(format!("{base_url}/props")).send().await else {
        return;
    };
    if response.status().as_u16() == 501 || response.status().as_u16() == 401 {
        telemetry.props_available = Some(false);
        return;
    }
    if !response.status().is_success() {
        return;
    }
    telemetry.props_available = Some(true);
    if let Ok(value) = response.json::<Value>().await {
        telemetry.total_slots = value.get("total_slots").and_then(Value::as_u64);
        telemetry.build_info = value
            .get("build_info")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
}

async fn probe_slots(client: &reqwest::Client, base_url: &str, telemetry: &mut ServerTelemetry) {
    let Ok(response) = client.get(format!("{base_url}/slots")).send().await else {
        return;
    };
    if response.status().as_u16() == 501 || response.status().as_u16() == 401 {
        telemetry.slots_available = Some(false);
        return;
    }
    if !response.status().is_success() {
        return;
    }
    telemetry.slots_available = Some(true);
    if let Ok(slots) = response.json::<Vec<Value>>().await {
        telemetry.total_slots = Some(slots.len() as u64);
        telemetry.busy_slots = Some(
            slots
                .iter()
                .filter(|slot| {
                    slot.get("is_processing")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .count() as u64,
        );
    }
}

async fn probe_metrics(client: &reqwest::Client, base_url: &str, telemetry: &mut ServerTelemetry) {
    let Ok(response) = client.get(format!("{base_url}/metrics")).send().await else {
        return;
    };
    if response.status().as_u16() == 501 || response.status().as_u16() == 401 {
        telemetry.metrics_available = Some(false);
        return;
    }
    if !response.status().is_success() {
        return;
    }
    telemetry.metrics_available = Some(true);
    let body = response.text().await.unwrap_or_default();
    telemetry.requests_processing = metric(&body, "llamacpp:requests_processing");
    telemetry.requests_deferred = metric(&body, "llamacpp:requests_deferred");
    telemetry.prompt_tokens_per_second = metric(&body, "llamacpp:prompt_tokens_seconds");
    telemetry.predicted_tokens_per_second = metric(&body, "llamacpp:predicted_tokens_seconds");
}

fn metric(body: &str, name: &str) -> Option<f64> {
    body.lines().find_map(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return None;
        }
        let (metric_name, value) = line.split_once(char::is_whitespace)?;
        (metric_name == name)
            .then(|| value.split_whitespace().next()?.parse::<f64>().ok())
            .flatten()
    })
}

fn extract_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("error")?
        .get("message")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_upstream_metric_names_and_skips_comments() {
        let body = "# HELP llamacpp:requests_processing requests\n\
                    llamacpp:requests_processing 2\n\
                    llamacpp:predicted_tokens_seconds 31.25\n";
        assert_eq!(metric(body, "llamacpp:requests_processing"), Some(2.0));
        assert_eq!(
            metric(body, "llamacpp:predicted_tokens_seconds"),
            Some(31.25)
        );
    }

    #[test]
    fn extracts_documented_error_envelope() {
        assert_eq!(
            extract_error_message(
                r#"{"error":{"code":503,"message":"Loading model","type":"unavailable_error"}}"#
            )
            .as_deref(),
            Some("Loading model")
        );
    }
}
