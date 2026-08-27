use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::{AppError, AppResult, ErrorCode};

use super::{PerformanceSweepCandidateResult, PerformanceSweepEvent};

#[derive(Default)]
pub struct PerformanceSweepSupervisor {
    active: std::sync::Mutex<Option<ActiveSweep>>,
}

struct ActiveSweep {
    id: uuid::Uuid,
    cancelled: Arc<AtomicBool>,
}

pub struct PerformanceSweepPermit<'a> {
    supervisor: &'a PerformanceSweepSupervisor,
    id: uuid::Uuid,
    cancelled: Arc<AtomicBool>,
}

impl PerformanceSweepSupervisor {
    pub fn begin(&self) -> AppResult<PerformanceSweepPermit<'_>> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err(sweep_in_progress_error());
        }

        let id = uuid::Uuid::new_v4();
        let cancelled = Arc::new(AtomicBool::new(false));
        *slot = Some(ActiveSweep {
            id,
            cancelled: Arc::clone(&cancelled),
        });
        Ok(PerformanceSweepPermit {
            supervisor: self,
            id,
            cancelled,
        })
    }

    pub fn cancel(&self) -> bool {
        let cancelled = self
            .lock()
            .as_ref()
            .map(|active| Arc::clone(&active.cancelled));
        let Some(cancelled) = cancelled else {
            return false;
        };
        cancelled.store(true, Ordering::Release);
        true
    }

    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    fn finish(&self, id: uuid::Uuid) {
        let mut slot = self.lock();
        if slot.as_ref().is_some_and(|active| active.id == id) {
            *slot = None;
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ActiveSweep>> {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn sweep_in_progress_error() -> AppError {
    AppError::new(
        ErrorCode::BenchmarkInProgress,
        "A Performance Lab sweep is already running.",
    )
    .with_hint("Wait for it to finish, or cancel it first.")
}

impl PerformanceSweepPermit<'_> {
    pub fn ensure_not_cancelled(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::BenchmarkCancelled,
                "The automatic performance sweep was cancelled.",
            ));
        }
        Ok(())
    }
}

impl Drop for PerformanceSweepPermit<'_> {
    fn drop(&mut self) {
        self.supervisor.finish(self.id);
    }
}

pub fn winning_result(
    results: &[PerformanceSweepCandidateResult],
) -> Option<&PerformanceSweepCandidateResult> {
    results
        .iter()
        .filter(|result| result.success && result.benchmark.is_some())
        .max_by(|left, right| {
            let left = left.benchmark.as_ref().expect("filtered benchmark");
            let right = right.benchmark.as_ref().expect("filtered benchmark");
            left.predicted_tokens_per_second
                .total_cmp(&right.predicted_tokens_per_second)
                .then_with(|| {
                    left.prompt_tokens_per_second
                        .total_cmp(&right.prompt_tokens_per_second)
                })
                .then_with(|| right.latency_ms.total_cmp(&left.latency_ms))
        })
}

pub fn event_finished(success: bool, winner_candidate_id: Option<String>) -> PerformanceSweepEvent {
    PerformanceSweepEvent::Finished {
        success,
        winner_candidate_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::{
        BenchmarkPlacement, PerformanceBenchmark, PERFORMANCE_SCHEMA_VERSION,
    };

    fn result(
        id: &str,
        prompt: f64,
        generation: f64,
        latency: f64,
    ) -> PerformanceSweepCandidateResult {
        PerformanceSweepCandidateResult {
            candidate_id: id.into(),
            candidate_label: id.into(),
            success: true,
            benchmark: Some(PerformanceBenchmark {
                schema_version: PERFORMANCE_SCHEMA_VERSION,
                id: format!("benchmark-{id}"),
                created_at: "now".into(),
                profile_id: "profile".into(),
                profile_name: "Coding".into(),
                runtime_id: "runtime".into(),
                runtime_label: "CUDA".into(),
                model_name: "Coder".into(),
                placement: BenchmarkPlacement::default(),
                gpus: Vec::new(),
                latency_ms: latency,
                prompt_tokens: 1,
                predicted_tokens: 1,
                prompt_ms: 1.0,
                predicted_ms: 1.0,
                prompt_tokens_per_second: prompt,
                predicted_tokens_per_second: generation,
                truncated: false,
                stop_type: None,
                draft_tokens: None,
                accepted_draft_tokens: None,
                sweep_id: Some("sweep".into()),
                candidate_id: Some(id.into()),
                candidate_label: Some(id.into()),
            }),
            error: None,
        }
    }

    #[test]
    fn ranks_generation_before_prompt_and_latency() {
        let results = vec![
            result("prompt-heavy", 900.0, 39.0, 100.0),
            result("winner", 400.0, 40.0, 200.0),
            result("same-speed-slower", 300.0, 40.0, 300.0),
        ];
        assert_eq!(winning_result(&results).unwrap().candidate_id, "winner");
    }

    #[test]
    fn cancellation_keeps_the_slot_until_the_permit_is_dropped() {
        let supervisor = PerformanceSweepSupervisor::default();
        let permit = supervisor.begin().expect("first sweep");
        assert!(supervisor.is_running());
        assert!(supervisor.cancel());
        assert_eq!(
            permit.ensure_not_cancelled().unwrap_err().code,
            ErrorCode::BenchmarkCancelled
        );
        assert!(supervisor.begin().is_err());
        drop(permit);
        assert!(!supervisor.is_running());
    }
}
