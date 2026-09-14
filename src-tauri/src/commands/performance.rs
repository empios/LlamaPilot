use std::time::Duration;

use tauri::ipc::Channel;
use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::hardware;
use crate::performance::{
    self, BenchmarkGpu, BenchmarkPlacement, PerformanceBenchmark, PerformanceCandidate,
    PerformancePlan, PerformanceSweep, PerformanceSweepCandidateResult, PerformanceSweepEvent,
    PerformanceSweepPermit, PERFORMANCE_SCHEMA_VERSION,
};
use crate::profiles::{LaunchProfile, ProfileOptionSetting};
use crate::server::ServerLifecycleState;
use crate::state::AppState;

use super::profiles::{prepare_profile, update_profile_record};
use super::server::launch_profile;

const SERVER_READY_TIMEOUT: Duration = Duration::from_secs(300);
const SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(15);

#[tauri::command]
pub async fn get_performance_plan(
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<PerformancePlan> {
    let _work = state.work.enter()?;
    performance_plan(&state, &profile_id).await
}

async fn performance_plan(state: &AppState, profile_id: &str) -> AppResult<PerformancePlan> {
    let profile = state.profiles.find(profile_id)?;
    let hardware = hardware::snapshot().await;
    let models = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let runtimes = state.runtimes.get();
    tauri::async_runtime::spawn_blocking(move || {
        let catalog = models.scan(&roots)?;
        let prepared = prepare_profile(profile.input(), Some(&profile), &runtimes, &catalog)?;
        let model_size_bytes = catalog
            .models
            .iter()
            .find(|model| model.id == profile.model_id)
            .map(|model| model.total_size_bytes);
        Ok(performance::build_plan(
            &profile,
            &prepared.capabilities,
            &hardware.gpus,
            model_size_bytes,
        ))
    })
    .await
    .map_err(|error| {
        AppError::internal("Preparing the performance plan stopped unexpectedly.")
            .with_details(error.to_string())
    })?
}

#[tauri::command]
pub fn list_performance_benchmarks(
    state: State<'_, AppState>,
    profile_id: Option<String>,
) -> Vec<PerformanceBenchmark> {
    state.performance.get().for_profile(profile_id.as_deref())
}

#[tauri::command]
pub async fn run_performance_benchmark(
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<PerformanceBenchmark> {
    let _work = state.work.enter()?;
    if state.performance_sweeps.is_running() {
        return Err(performance::sweep_in_progress_error());
    }
    run_benchmark_for_profile(&state, &profile_id, None).await
}

#[tauri::command]
pub fn cancel_performance_sweep(state: State<'_, AppState>) -> bool {
    state.performance_sweeps.cancel()
}

#[tauri::command]
pub fn is_performance_sweep_running(state: State<'_, AppState>) -> bool {
    state.performance_sweeps.is_running()
}

#[tauri::command]
pub async fn run_performance_sweep(
    state: State<'_, AppState>,
    profile_id: String,
    on_event: Channel<PerformanceSweepEvent>,
) -> AppResult<PerformanceSweep> {
    let _work = state.work.enter()?;
    let permit = state.performance_sweeps.begin()?;
    let result = run_sweep(&state, &profile_id, &on_event, &permit).await;
    if result.is_err() {
        send_event(&on_event, performance::event_finished(false, None));
    }
    result
}

async fn run_sweep(
    state: &AppState,
    profile_id: &str,
    on_event: &Channel<PerformanceSweepEvent>,
    permit: &PerformanceSweepPermit<'_>,
) -> AppResult<PerformanceSweep> {
    let snapshot = state.server.snapshot().await;
    if snapshot.state.is_active() {
        return Err(AppError::new(
            ErrorCode::ServerAlreadyRunning,
            "Stop the active server before starting an automatic performance sweep.",
        )
        .with_hint(
            "The sweep controls every start and stop so its measurements remain comparable.",
        ));
    }

    let original = state.profiles.find(profile_id)?;
    let plan = performance_plan(state, profile_id).await?;
    if plan.candidates.is_empty() {
        return Err(AppError::new(
            ErrorCode::BenchmarkFailed,
            "The selected runtime produced no benchmarkable placement candidates.",
        )
        .with_hint("Inspect a GPU runtime and verify that it advertises device placement flags."));
    }

    let sweep_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();
    let total_candidates = plan.candidates.len();
    send_event(
        on_event,
        PerformanceSweepEvent::Started {
            sweep_id: sweep_id.clone(),
            total_candidates,
        },
    );

    let mut results = Vec::with_capacity(total_candidates);
    for (position, candidate) in plan.candidates.iter().enumerate() {
        if let Err(error) = permit.ensure_not_cancelled() {
            restore_original(state, &original).await?;
            return Err(error);
        }
        send_event(
            on_event,
            PerformanceSweepEvent::CandidateStarted {
                candidate_id: candidate.id.clone(),
                candidate_label: candidate.label.clone(),
                index: position + 1,
                total_candidates,
            },
        );

        let attempt = run_candidate(state, &original, candidate, &sweep_id, permit).await;
        let stopped = ensure_server_stopped(state).await;
        let result = match (attempt, stopped) {
            (Ok(benchmark), Ok(())) => PerformanceSweepCandidateResult {
                candidate_id: candidate.id.clone(),
                candidate_label: candidate.label.clone(),
                success: true,
                benchmark: Some(benchmark),
                error: None,
            },
            (Err(error), Ok(())) => PerformanceSweepCandidateResult {
                candidate_id: candidate.id.clone(),
                candidate_label: candidate.label.clone(),
                success: false,
                benchmark: None,
                error: Some(error.message),
            },
            (attempt, Err(stop_error)) => {
                let attempt_error = attempt.err().map(|error| error.message);
                let combined = attempt_error
                    .map(|error| {
                        format!("{error} Server cleanup also failed: {}", stop_error.message)
                    })
                    .unwrap_or_else(|| stop_error.message.clone());
                let result = PerformanceSweepCandidateResult {
                    candidate_id: candidate.id.clone(),
                    candidate_label: candidate.label.clone(),
                    success: false,
                    benchmark: None,
                    error: Some(combined),
                };
                send_event(
                    on_event,
                    PerformanceSweepEvent::CandidateFinished {
                        result: Box::new(result.clone()),
                        completed_candidates: position + 1,
                        total_candidates,
                    },
                );
                results.push(result);
                let _ = update_profile_record(state, original.id.clone(), original.input()).await;
                return Err(stop_error);
            }
        };

        send_event(
            on_event,
            PerformanceSweepEvent::CandidateFinished {
                result: Box::new(result.clone()),
                completed_candidates: position + 1,
                total_candidates,
            },
        );
        results.push(result);

        if let Err(error) = permit.ensure_not_cancelled() {
            restore_original(state, &original).await?;
            return Err(error);
        }
    }

    let winner = performance::winning_result(&results);
    let winner_candidate_id = winner.map(|result| result.candidate_id.clone());
    let winner_benchmark_id = winner
        .and_then(|result| result.benchmark.as_ref())
        .map(|benchmark| benchmark.id.clone());
    let applied_winner = if let Some(candidate_id) = winner_candidate_id.as_deref() {
        let candidate = plan
            .candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)
            .expect("winner belongs to the generated plan");
        send_event(
            on_event,
            PerformanceSweepEvent::ApplyingWinner {
                candidate_id: candidate.id.clone(),
                candidate_label: candidate.label.clone(),
            },
        );
        let applied = update_profile_record(
            state,
            original.id.clone(),
            performance::apply_candidate(original.input(), candidate),
        )
        .await;
        if let Err(error) = applied {
            let _ = restore_original(state, &original).await;
            return Err(error);
        }
        true
    } else {
        restore_original(state, &original).await?;
        false
    };

    let sweep = PerformanceSweep {
        id: sweep_id,
        profile_id: original.id,
        profile_name: original.name,
        started_at,
        finished_at: chrono::Utc::now().to_rfc3339(),
        results,
        winner_candidate_id: winner_candidate_id.clone(),
        winner_benchmark_id,
        applied_winner,
    };
    send_event(
        on_event,
        performance::event_finished(applied_winner, winner_candidate_id),
    );
    Ok(sweep)
}

async fn run_candidate(
    state: &AppState,
    original: &LaunchProfile,
    candidate: &PerformanceCandidate,
    sweep_id: &str,
    permit: &PerformanceSweepPermit<'_>,
) -> AppResult<PerformanceBenchmark> {
    let input = performance::apply_candidate(original.input(), candidate);
    update_profile_record(state, original.id.clone(), input).await?;
    permit.ensure_not_cancelled()?;
    launch_profile(state, original.id.clone()).await?;
    wait_until_ready(state, &original.id, permit).await?;
    permit.ensure_not_cancelled()?;
    run_benchmark_for_profile(
        state,
        &original.id,
        Some(BenchmarkContext {
            sweep_id,
            candidate,
        }),
    )
    .await
}

async fn wait_until_ready(
    state: &AppState,
    profile_id: &str,
    permit: &PerformanceSweepPermit<'_>,
) -> AppResult<()> {
    let deadline = tokio::time::Instant::now() + SERVER_READY_TIMEOUT;
    loop {
        permit.ensure_not_cancelled()?;
        let snapshot = state.server.snapshot().await;
        if snapshot.profile_id.as_deref() != Some(profile_id) {
            return Err(AppError::new(
                ErrorCode::BenchmarkFailed,
                "The server switched to a different profile during the performance sweep.",
            ));
        }
        match snapshot.state {
            ServerLifecycleState::Ready => return Ok(()),
            ServerLifecycleState::Crashed | ServerLifecycleState::Stopped => {
                return Err(AppError::new(
                    ErrorCode::BenchmarkFailed,
                    "llama-server did not become ready for this candidate.",
                )
                .with_details(snapshot.last_error.unwrap_or_default()));
            }
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::BenchmarkFailed,
                "Timed out while waiting for llama-server to load the candidate.",
            )
            .with_hint(
                "Review the server log for memory pressure or unsupported placement options.",
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn ensure_server_stopped(state: &AppState) -> AppResult<()> {
    if state.server.snapshot().await.state.is_active() {
        state.server.stop().await?;
        state.server.wait_until_stopped(SERVER_STOP_TIMEOUT).await?;
    }
    Ok(())
}

async fn restore_original(state: &AppState, original: &LaunchProfile) -> AppResult<()> {
    ensure_server_stopped(state).await?;
    update_profile_record(state, original.id.clone(), original.input()).await?;
    Ok(())
}

fn send_event(channel: &Channel<PerformanceSweepEvent>, event: PerformanceSweepEvent) {
    let _ = channel.send(event);
}

struct BenchmarkContext<'a> {
    sweep_id: &'a str,
    candidate: &'a PerformanceCandidate,
}

async fn run_benchmark_for_profile(
    state: &AppState,
    profile_id: &str,
    context: Option<BenchmarkContext<'_>>,
) -> AppResult<PerformanceBenchmark> {
    let profile = state.profiles.find(profile_id)?;
    let snapshot = state.server.snapshot().await;
    if snapshot.state != ServerLifecycleState::Ready {
        return Err(AppError::new(
            ErrorCode::BenchmarkFailed,
            "The selected server must be idle and Ready before benchmarking.",
        )
        .with_hint("Start this profile, wait for Ready, and stop other client requests."));
    }
    if snapshot.profile_id.as_deref() != Some(profile_id) {
        return Err(AppError::new(
            ErrorCode::BenchmarkFailed,
            "The active server is using a different profile.",
        )
        .with_hint("Stop the active server, then start the profile selected in Performance Lab."));
    }
    let host = snapshot.host.as_deref().ok_or_else(|| {
        AppError::internal("The active server has no host for the benchmark request.")
    })?;
    let port = snapshot.port.ok_or_else(|| {
        AppError::internal("The active server has no port for the benchmark request.")
    })?;
    let base_url = performance::local_base_url(host, port);
    let measurement = performance::run_benchmark(&base_url).await?;
    let hardware = hardware::snapshot().await;

    let benchmark = PerformanceBenchmark {
        schema_version: PERFORMANCE_SCHEMA_VERSION,
        id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        runtime_id: profile.runtime_id.clone(),
        runtime_label: profile.runtime_label.clone(),
        model_name: profile.model_name.clone(),
        placement: placement(&profile),
        gpus: hardware
            .gpus
            .into_iter()
            .map(|gpu| BenchmarkGpu {
                index: gpu.index,
                name: gpu.name,
                total_memory_mib: gpu.total_memory_mib,
                free_memory_mib: gpu.free_memory_mib,
            })
            .collect(),
        latency_ms: measurement.latency_ms,
        prompt_tokens: measurement.prompt_tokens,
        predicted_tokens: measurement.predicted_tokens,
        prompt_ms: measurement.prompt_ms,
        predicted_ms: measurement.predicted_ms,
        prompt_tokens_per_second: measurement.prompt_tokens_per_second,
        predicted_tokens_per_second: measurement.predicted_tokens_per_second,
        truncated: measurement.truncated,
        stop_type: measurement.stop_type,
        draft_tokens: measurement.draft_tokens,
        accepted_draft_tokens: measurement.accepted_draft_tokens,
        sweep_id: context.as_ref().map(|context| context.sweep_id.to_string()),
        candidate_id: context.as_ref().map(|context| context.candidate.id.clone()),
        candidate_label: context.map(|context| context.candidate.label.clone()),
    };
    state
        .performance
        .update(|history| history.record(benchmark.clone()))?;
    Ok(benchmark)
}

fn placement(profile: &LaunchProfile) -> BenchmarkPlacement {
    BenchmarkPlacement {
        devices: custom_value(profile, "device"),
        split_mode: custom_value(profile, "splitMode"),
        tensor_split: custom_value(profile, "tensorSplit"),
        gpu_layers: custom_value(profile, "gpuLayers"),
    }
}

fn custom_value(profile: &LaunchProfile, key: &str) -> Option<String> {
    match profile.options.get(key) {
        Some(ProfileOptionSetting::Custom { value }) => Some(value.clone()),
        Some(ProfileOptionSetting::Auto) => Some("auto".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::PROFILE_SCHEMA_VERSION;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn captures_the_profile_placement_used_by_a_benchmark() {
        let profile = LaunchProfile {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: "profile".into(),
            name: "Coding".into(),
            description: None,
            runtime_id: "runtime".into(),
            runtime_label: "CUDA".into(),
            model_id: "model".into(),
            model_name: "Coder".into(),
            model_path: PathBuf::from("/models/coder.gguf"),
            projector_path: None,
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: false,
            options: BTreeMap::from([
                (
                    "device".into(),
                    ProfileOptionSetting::Custom {
                        value: "CUDA0,CUDA1".into(),
                    },
                ),
                (
                    "splitMode".into(),
                    ProfileOptionSetting::Custom {
                        value: "layer".into(),
                    },
                ),
                ("gpuLayers".into(), ProfileOptionSetting::Auto),
            ]),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
            created_at: "now".into(),
            updated_at: "now".into(),
        };

        let placement = placement(&profile);
        assert_eq!(placement.devices.as_deref(), Some("CUDA0,CUDA1"));
        assert_eq!(placement.split_mode.as_deref(), Some("layer"));
        assert_eq!(placement.gpu_layers.as_deref(), Some("auto"));
    }
}
