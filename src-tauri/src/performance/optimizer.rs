use std::collections::BTreeMap;

use crate::hardware::GpuInfo;
use crate::llama::capabilities::{LlamaCapabilities, LlamaDevice, LlamaOption};
use crate::profiles::{LaunchProfile, ProfileInput, ProfileOptionSetting};

use super::model::{
    PerformanceCandidate, PerformanceDevice, PerformanceObjective, PerformancePlan,
};

const MINIMUM_VRAM_RESERVE_MIB: u64 = 1_024;
const MANAGED_OPTION_KEYS: &[&str] = &[
    "device",
    "gpuLayers",
    "tensorSplit",
    "splitMode",
    "mainGpu",
    "flashAttention",
    "fit",
    "kvCacheOffload",
    "kvCacheTypeK",
    "kvCacheTypeV",
];

/// Builds every candidate from the same clean placement baseline so a previous row or tensor
/// attempt cannot leak strategy-specific flags into the next measurement.
pub fn apply_candidate(mut input: ProfileInput, candidate: &PerformanceCandidate) -> ProfileInput {
    for key in MANAGED_OPTION_KEYS {
        input.options.remove(*key);
    }
    input.options.extend(candidate.options.clone());
    input
}

pub fn build_plan(
    profile: &LaunchProfile,
    capabilities: &LlamaCapabilities,
    hardware_gpus: &[GpuInfo],
    model_size_bytes: Option<u64>,
) -> PerformancePlan {
    let devices = performance_devices(&capabilities.devices, hardware_gpus);
    let total_free_memory_mib = devices
        .iter()
        .filter_map(|device| device.free_memory_mib)
        .sum();
    let mut warnings = Vec::new();

    if devices.is_empty() {
        warnings.push(
            "The selected runtime reported no accelerator devices. Inspect a CUDA runtime before generating a GPU plan."
                .into(),
        );
    } else if devices.len() == 1 {
        warnings.push(
            "Only one accelerator is available. The plan can tune GPU offload, but cannot compare multi-GPU placement."
                .into(),
        );
    }

    if let Some(model_size_bytes) = model_size_bytes {
        let model_size_mib = model_size_bytes.div_ceil(1024 * 1024);
        let usable = devices
            .iter()
            .map(|device| usable_memory(device.total_memory_mib, device.free_memory_mib))
            .sum::<u64>();
        if usable > 0 && model_size_mib > usable {
            warnings.push(format!(
                "The model is about {model_size_mib} MiB, while the GPUs have about {usable} MiB available after safety reserves. Context and KV cache may require CPU fallback or a smaller quantization."
            ));
        }
    }

    let candidates = candidates(capabilities, &devices);
    if !devices.is_empty() && candidates.is_empty() {
        warnings.push(
            "This runtime does not advertise the placement flags required to build a controlled multi-GPU candidate."
                .into(),
        );
    }
    let recommended_candidate_id = candidates.first().map(|candidate| candidate.id.clone());

    PerformancePlan {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        runtime_id: profile.runtime_id.clone(),
        runtime_label: profile.runtime_label.clone(),
        model_id: profile.model_id.clone(),
        model_name: profile.model_name.clone(),
        model_size_bytes,
        objective: PerformanceObjective::InteractiveCoding,
        devices,
        total_free_memory_mib,
        candidates,
        recommended_candidate_id,
        warnings,
    }
}

fn performance_devices(
    runtime_devices: &[LlamaDevice],
    hardware_gpus: &[GpuInfo],
) -> Vec<PerformanceDevice> {
    let mut devices: Vec<_> = runtime_devices
        .iter()
        .enumerate()
        .map(|(position, device)| {
            let hardware = device_index(&device.id)
                .and_then(|index| hardware_gpus.iter().find(|gpu| gpu.index == index))
                .or_else(|| hardware_gpus.get(position));
            PerformanceDevice {
                id: device.id.clone(),
                name: if device.name.trim().is_empty() {
                    hardware
                        .map(|gpu| gpu.name.clone())
                        .unwrap_or_else(|| device.id.clone())
                } else {
                    device.name.clone()
                },
                total_memory_mib: hardware
                    .map(|gpu| gpu.total_memory_mib)
                    .or(device.memory_total_mib),
                free_memory_mib: hardware
                    .map(|gpu| gpu.free_memory_mib)
                    .or(device.memory_free_mib),
                split_percent: 0,
            }
        })
        .collect();

    let weights: Vec<u64> = devices
        .iter()
        .map(|device| usable_memory(device.total_memory_mib, device.free_memory_mib).max(1))
        .collect();
    let percentages = percentages(&weights);
    for (device, percent) in devices.iter_mut().zip(percentages) {
        device.split_percent = percent;
    }
    devices
}

fn candidates(
    capabilities: &LlamaCapabilities,
    devices: &[PerformanceDevice],
) -> Vec<PerformanceCandidate> {
    if devices.is_empty() || !supports(capabilities, "device") {
        return Vec::new();
    }

    let device_value = devices
        .iter()
        .map(|device| device.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let split_value = devices
        .iter()
        .map(|device| device.split_percent.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mut common = BTreeMap::new();
    common.insert("device".into(), custom(device_value));
    insert_if_supported(capabilities, &mut common, "gpuLayers", "all");
    if devices.len() > 1 {
        insert_if_supported(capabilities, &mut common, "tensorSplit", &split_value);
    }
    insert_if_supported(capabilities, &mut common, "flashAttention", "true");

    let split_modes = split_modes(capabilities);
    if devices.len() == 1 {
        let mut options = common;
        if split_modes.contains(&"none".to_string()) {
            options.insert("splitMode".into(), custom("none"));
        }
        return vec![PerformanceCandidate {
            id: "single-gpu".into(),
            label: "Single GPU baseline".into(),
            description:
                "Offload the model to the detected accelerator and keep a repeatable baseline."
                    .into(),
            split_mode: "none".into(),
            experimental: false,
            options,
            reasons: vec!["The runtime reported one usable accelerator.".into()],
            warnings: Vec::new(),
        }];
    }

    let mut result = Vec::new();
    if split_modes.is_empty() || split_modes.contains(&"layer".to_string()) {
        let mut options = common.clone();
        if supports(capabilities, "splitMode") {
            options.insert("splitMode".into(), custom("layer"));
        }
        result.push(PerformanceCandidate {
            id: "all-gpus-layer".into(),
            label: "All GPUs · layer split".into(),
            description:
                "Distribute model layers across every detected GPU in proportion to usable VRAM."
                    .into(),
            split_mode: "layer".into(),
            experimental: false,
            options,
            reasons: vec![
                "Uses every accelerator reported by this exact runtime.".into(),
                format!("VRAM-weighted placement starts at {split_value}."),
                "Layer split is the safest multi-GPU baseline before measuring row or tensor modes."
                    .into(),
            ],
            warnings: Vec::new(),
        });
    }

    if split_modes.contains(&"row".to_string()) {
        let mut options = common.clone();
        options.insert("splitMode".into(), custom("row"));
        if supports(capabilities, "mainGpu") {
            options.insert(
                "mainGpu".into(),
                custom(main_gpu_position(devices).to_string()),
            );
        }
        result.push(PerformanceCandidate {
            id: "all-gpus-row".into(),
            label: "All GPUs · row split".into(),
            description: "Split tensor rows between GPUs and keep intermediate work on the GPU with the most usable memory."
                .into(),
            split_mode: "row".into(),
            experimental: false,
            options,
            reasons: vec![
                "Can improve parallel work when the PCIe topology and GPU pair are well matched."
                    .into(),
                "Must be benchmarked against layer split on this machine.".into(),
            ],
            warnings: vec![
                "Row split can lose to layer split when inter-GPU traffic is expensive.".into(),
            ],
        });
    }

    if split_modes.contains(&"tensor".to_string()) {
        let mut options = common;
        options.insert("splitMode".into(), custom("tensor"));
        insert_if_supported(capabilities, &mut options, "fit", "false");
        insert_if_supported(capabilities, &mut options, "kvCacheOffload", "true");
        set_non_quantized_cache(capabilities, &mut options, "kvCacheTypeK");
        set_non_quantized_cache(capabilities, &mut options, "kvCacheTypeV");
        result.push(PerformanceCandidate {
            id: "all-gpus-tensor".into(),
            label: "All GPUs · tensor parallel".into(),
            description: "Use the runtime's experimental tensor-parallel path across all GPUs."
                .into(),
            split_mode: "tensor".into(),
            experimental: true,
            options,
            reasons: vec![
                "Offers the strongest cross-GPU parallelism when the model architecture and backend support it."
                    .into(),
                "The runtime explicitly advertises tensor mode.".into(),
            ],
            warnings: vec![
                "Requires Flash Attention and non-quantized KV cache, and may fail for unsupported model architectures."
                    .into(),
                "Automatic memory fitting is disabled for this candidate.".into(),
            ],
        });
    }

    result
}

fn supports(capabilities: &LlamaCapabilities, key: &str) -> bool {
    capabilities
        .options
        .values()
        .any(|option| option.known_key.as_deref() == Some(key))
}

fn option<'a>(capabilities: &'a LlamaCapabilities, key: &str) -> Option<&'a LlamaOption> {
    capabilities
        .options
        .values()
        .find(|option| option.known_key.as_deref() == Some(key))
}

fn split_modes(capabilities: &LlamaCapabilities) -> Vec<String> {
    let Some(option) = option(capabilities, "splitMode") else {
        return Vec::new();
    };
    let mut values = option
        .value_hint
        .as_deref()
        .map(parse_choices)
        .unwrap_or_default();
    for line in option.description.lines() {
        let lower = line.to_ascii_lowercase();
        for marker in ["allowed values:", "one of:"] {
            if let Some((_, value)) = lower.split_once(marker) {
                values.extend(parse_choices(value));
            }
        }
    }
    values.sort();
    values.dedup();
    values
}

fn parse_choices(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_matches(|character| matches!(character, '<' | '>' | '[' | ']' | '{' | '}'))
        .split([',', '|'])
        .filter_map(|choice| {
            choice
                .split_whitespace()
                .next()
                .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
                .filter(|word| !word.is_empty())
                .map(str::to_ascii_lowercase)
        })
        .filter(|choice| matches!(choice.as_str(), "none" | "layer" | "row" | "tensor"))
        .collect()
}

fn insert_if_supported(
    capabilities: &LlamaCapabilities,
    options: &mut BTreeMap<String, ProfileOptionSetting>,
    key: &str,
    value: &str,
) {
    if supports(capabilities, key) {
        options.insert(key.into(), custom(value));
    }
}

fn set_non_quantized_cache(
    capabilities: &LlamaCapabilities,
    options: &mut BTreeMap<String, ProfileOptionSetting>,
    key: &str,
) {
    let Some(option) = option(capabilities, key) else {
        return;
    };
    let combined = format!(
        "{} {}",
        option.value_hint.as_deref().unwrap_or_default(),
        option.description
    )
    .to_ascii_lowercase();
    for value in ["f16", "bf16", "f32"] {
        if combined.contains(value) {
            options.insert(key.into(), custom(value));
            return;
        }
    }
}

fn custom(value: impl Into<String>) -> ProfileOptionSetting {
    ProfileOptionSetting::Custom {
        value: value.into(),
    }
}

fn usable_memory(total: Option<u64>, free: Option<u64>) -> u64 {
    let free = free.or(total).unwrap_or(1);
    let reserve = total
        .map(|total| (total / 12).max(MINIMUM_VRAM_RESERVE_MIB))
        .unwrap_or(MINIMUM_VRAM_RESERVE_MIB);
    free.saturating_sub(reserve).max(1)
}

fn percentages(weights: &[u64]) -> Vec<u32> {
    if weights.is_empty() {
        return Vec::new();
    }
    let total = weights.iter().sum::<u64>().max(1);
    let mut result: Vec<u32> = weights
        .iter()
        .map(|weight| ((*weight as f64 / total as f64) * 100.0).round() as u32)
        .collect();
    let sum = result.iter().sum::<u32>();
    if sum != 100 {
        let largest = weights
            .iter()
            .enumerate()
            .max_by_key(|(_, weight)| *weight)
            .map(|(index, _)| index)
            .unwrap_or(0);
        if sum < 100 {
            result[largest] += 100 - sum;
        } else {
            result[largest] = result[largest].saturating_sub(sum - 100);
        }
    }
    result
}

fn main_gpu_position(devices: &[PerformanceDevice]) -> usize {
    devices
        .iter()
        .enumerate()
        .max_by_key(|(_, device)| device.free_memory_mib.unwrap_or(0))
        .map(|(position, _)| position)
        .unwrap_or(0)
}

fn device_index(id: &str) -> Option<u32> {
    let digits = id
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llama::capabilities::{LlamaOptionCategory, CAPABILITIES_SCHEMA_VERSION};
    use crate::profiles::PROFILE_SCHEMA_VERSION;
    use std::path::PathBuf;

    fn known_option(key: &str, hint: Option<&str>) -> LlamaOption {
        LlamaOption {
            flag: format!("--{key}"),
            aliases: vec![format!("--{key}")],
            value_hint: hint.map(str::to_string),
            description: String::new(),
            section: "gpu".into(),
            category: LlamaOptionCategory::Gpu,
            known_key: Some(key.into()),
            display_name: key.into(),
            summary: None,
        }
    }

    fn capabilities() -> LlamaCapabilities {
        LlamaCapabilities {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            version: "test".into(),
            commit: None,
            options: BTreeMap::from([
                ("--device".into(), known_option("device", None)),
                ("--gpu-layers".into(), known_option("gpuLayers", None)),
                (
                    "--split-mode".into(),
                    known_option("splitMode", Some("{none,layer,row,tensor}")),
                ),
                ("--tensor-split".into(), known_option("tensorSplit", None)),
                ("--main-gpu".into(), known_option("mainGpu", None)),
                ("--flash-attn".into(), known_option("flashAttention", None)),
                (
                    "--cache-type-k".into(),
                    known_option("kvCacheTypeK", Some("{f16,q8_0}")),
                ),
                (
                    "--cache-type-v".into(),
                    known_option("kvCacheTypeV", Some("{f16,q8_0}")),
                ),
            ]),
            speculative_types: Vec::new(),
            devices: vec![
                LlamaDevice {
                    id: "CUDA0".into(),
                    name: "Fast GPU".into(),
                    backend: Some("CUDA".into()),
                    memory_total_mib: Some(24_000),
                    memory_free_mib: Some(20_000),
                    raw: String::new(),
                },
                LlamaDevice {
                    id: "CUDA1".into(),
                    name: "Large GPU".into(),
                    backend: Some("CUDA".into()),
                    memory_total_mib: Some(48_000),
                    memory_free_mib: Some(40_000),
                    raw: String::new(),
                },
            ],
        }
    }

    fn profile() -> LaunchProfile {
        LaunchProfile {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: "profile".into(),
            name: "Coding".into(),
            description: None,
            runtime_id: "runtime".into(),
            runtime_label: "CUDA runtime".into(),
            model_id: "model".into(),
            model_name: "Coder".into(),
            model_path: PathBuf::from("/models/coder.gguf"),
            projector_path: None,
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: false,
            options: BTreeMap::new(),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    #[test]
    fn produces_safe_and_experimental_candidates_for_two_gpus() {
        let hardware = vec![
            GpuInfo {
                index: 0,
                name: "Fast GPU".into(),
                total_memory_mib: 24_000,
                used_memory_mib: 4_000,
                free_memory_mib: 20_000,
                driver_version: None,
            },
            GpuInfo {
                index: 1,
                name: "Large GPU".into(),
                total_memory_mib: 48_000,
                used_memory_mib: 8_000,
                free_memory_mib: 40_000,
                driver_version: None,
            },
        ];

        let plan = build_plan(&profile(), &capabilities(), &hardware, Some(30 << 30));

        assert_eq!(plan.devices.len(), 2);
        assert_eq!(
            plan.devices
                .iter()
                .map(|gpu| gpu.split_percent)
                .sum::<u32>(),
            100
        );
        assert_eq!(
            plan.recommended_candidate_id.as_deref(),
            Some("all-gpus-layer")
        );
        assert_eq!(plan.candidates.len(), 3);
        let tensor = plan
            .candidates
            .iter()
            .find(|candidate| candidate.id == "all-gpus-tensor")
            .expect("tensor candidate");
        assert!(tensor.experimental);
        assert_eq!(tensor.options.get("kvCacheTypeK"), Some(&custom("f16")));
    }

    #[test]
    fn a_runtime_without_devices_returns_an_actionable_warning() {
        let mut capabilities = capabilities();
        capabilities.devices.clear();

        let plan = build_plan(&profile(), &capabilities, &[], None);

        assert!(plan.candidates.is_empty());
        assert!(plan.warnings[0].contains("no accelerator devices"));
    }

    #[test]
    fn percentages_always_sum_to_one_hundred() {
        assert_eq!(percentages(&[1, 1, 1]).iter().sum::<u32>(), 100);
        assert_eq!(percentages(&[1, 2]), vec![33, 67]);
    }

    #[test]
    fn applying_a_candidate_clears_previous_strategy_flags_but_keeps_unrelated_settings() {
        let mut input = profile().input();
        input.options = BTreeMap::from([
            ("contextSize".into(), custom("32768")),
            ("mainGpu".into(), custom("1")),
            ("splitMode".into(), custom("row")),
            ("kvCacheTypeK".into(), custom("f16")),
        ]);
        let candidate = PerformanceCandidate {
            id: "layer".into(),
            label: "Layer".into(),
            description: String::new(),
            split_mode: "layer".into(),
            experimental: false,
            options: BTreeMap::from([("splitMode".into(), custom("layer"))]),
            reasons: Vec::new(),
            warnings: Vec::new(),
        };

        let applied = apply_candidate(input, &candidate);
        assert_eq!(applied.options.get("contextSize"), Some(&custom("32768")));
        assert_eq!(applied.options.get("splitMode"), Some(&custom("layer")));
        assert!(!applied.options.contains_key("mainGpu"));
        assert!(!applied.options.contains_key("kvCacheTypeK"));
    }
}
