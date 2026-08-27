use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::profiles::ProfileOptionSetting;

pub const PERFORMANCE_SCHEMA_VERSION: u32 = 1;
const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PerformanceObjective {
    InteractiveCoding,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceDevice {
    pub id: String,
    pub name: String,
    pub total_memory_mib: Option<u64>,
    pub free_memory_mib: Option<u64>,
    pub split_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceCandidate {
    pub id: String,
    pub label: String,
    pub description: String,
    pub split_mode: String,
    pub experimental: bool,
    pub options: BTreeMap<String, ProfileOptionSetting>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePlan {
    pub profile_id: String,
    pub profile_name: String,
    pub runtime_id: String,
    pub runtime_label: String,
    pub model_id: String,
    pub model_name: String,
    pub model_size_bytes: Option<u64>,
    pub objective: PerformanceObjective,
    pub devices: Vec<PerformanceDevice>,
    pub total_free_memory_mib: u64,
    pub candidates: Vec<PerformanceCandidate>,
    pub recommended_candidate_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkGpu {
    pub index: u32,
    pub name: String,
    pub total_memory_mib: u64,
    pub free_memory_mib: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkPlacement {
    pub devices: Option<String>,
    pub split_mode: Option<String>,
    pub tensor_split: Option<String>,
    pub gpu_layers: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceBenchmark {
    pub schema_version: u32,
    pub id: String,
    pub created_at: String,
    pub profile_id: String,
    pub profile_name: String,
    pub runtime_id: String,
    pub runtime_label: String,
    pub model_name: String,
    pub placement: BenchmarkPlacement,
    pub gpus: Vec<BenchmarkGpu>,
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
    #[serde(default)]
    pub sweep_id: Option<String>,
    #[serde(default)]
    pub candidate_id: Option<String>,
    #[serde(default)]
    pub candidate_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSweepCandidateResult {
    pub candidate_id: String,
    pub candidate_label: String,
    pub success: bool,
    pub benchmark: Option<PerformanceBenchmark>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSweep {
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub started_at: String,
    pub finished_at: String,
    pub results: Vec<PerformanceSweepCandidateResult>,
    pub winner_candidate_id: Option<String>,
    pub winner_benchmark_id: Option<String>,
    pub applied_winner: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PerformanceSweepEvent {
    Started {
        sweep_id: String,
        total_candidates: usize,
    },
    CandidateStarted {
        candidate_id: String,
        candidate_label: String,
        index: usize,
        total_candidates: usize,
    },
    CandidateFinished {
        result: Box<PerformanceSweepCandidateResult>,
        completed_candidates: usize,
        total_candidates: usize,
    },
    ApplyingWinner {
        candidate_id: String,
        candidate_label: String,
    },
    Finished {
        success: bool,
        winner_candidate_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PerformanceHistory {
    pub schema_version: u32,
    pub results: Vec<PerformanceBenchmark>,
}

impl Default for PerformanceHistory {
    fn default() -> Self {
        Self {
            schema_version: PERFORMANCE_SCHEMA_VERSION,
            results: Vec::new(),
        }
    }
}

impl PerformanceHistory {
    pub fn record(&mut self, benchmark: PerformanceBenchmark) {
        self.results.insert(0, benchmark);
        self.results.truncate(MAX_HISTORY_ENTRIES);
    }

    pub fn for_profile(&self, profile_id: Option<&str>) -> Vec<PerformanceBenchmark> {
        self.results
            .iter()
            .filter(|result| profile_id.is_none_or(|id| result.profile_id == id))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn benchmark(index: usize) -> PerformanceBenchmark {
        PerformanceBenchmark {
            schema_version: PERFORMANCE_SCHEMA_VERSION,
            id: format!("benchmark-{index}"),
            created_at: format!("2026-08-27T00:00:{index:02}Z"),
            profile_id: if index % 2 == 0 { "a" } else { "b" }.into(),
            profile_name: "Coding".into(),
            runtime_id: "runtime".into(),
            runtime_label: "CUDA runtime".into(),
            model_name: "Coder".into(),
            placement: BenchmarkPlacement::default(),
            gpus: Vec::new(),
            latency_ms: 1.0,
            prompt_tokens: 1,
            predicted_tokens: 1,
            prompt_ms: 1.0,
            predicted_ms: 1.0,
            prompt_tokens_per_second: 1.0,
            predicted_tokens_per_second: 1.0,
            truncated: false,
            stop_type: None,
            draft_tokens: None,
            accepted_draft_tokens: None,
            sweep_id: None,
            candidate_id: None,
            candidate_label: None,
        }
    }

    #[test]
    fn history_keeps_newest_results_and_filters_by_profile() {
        let mut history = PerformanceHistory::default();
        for index in 0..105 {
            history.record(benchmark(index));
        }

        assert_eq!(history.results.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(history.results[0].id, "benchmark-104");
        assert!(history
            .for_profile(Some("a"))
            .iter()
            .all(|result| result.profile_id == "a"));
    }
}
