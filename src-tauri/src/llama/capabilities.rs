use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const CAPABILITIES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilitySummary {
    pub inspection_id: String,
    pub inspected_at: String,
    pub version: String,
    pub commit: Option<String>,
    pub option_count: usize,
    pub known_option_count: usize,
    pub device_count: usize,
    pub speculative_type_count: usize,
}

impl RuntimeCapabilitySummary {
    pub fn from_capabilities(
        inspection_id: String,
        inspected_at: String,
        capabilities: &LlamaCapabilities,
    ) -> Self {
        Self {
            inspection_id,
            inspected_at,
            version: capabilities.version.clone(),
            commit: capabilities.commit.clone(),
            option_count: capabilities.options.len(),
            known_option_count: capabilities.known_option_count(),
            device_count: capabilities.devices.len(),
            speculative_type_count: capabilities.speculative_types.len(),
        }
    }
}

/// Parsed, runtime-specific description of the command-line surface exposed by llama-server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCapabilities {
    pub schema_version: u32,
    pub version: String,
    pub commit: Option<String>,
    pub options: BTreeMap<String, LlamaOption>,
    pub speculative_types: Vec<String>,
    pub devices: Vec<LlamaDevice>,
}

impl LlamaCapabilities {
    pub fn known_option_count(&self) -> usize {
        self.options
            .values()
            .filter(|option| option.known_key.is_some())
            .count()
    }

    /// Re-applies the current presentation registry to an immutable inspection. The advertised
    /// flags remain the source of truth; this only lets newer app versions recognise concepts in
    /// an older persisted manifest without changing that runtime snapshot.
    pub fn refresh_known_registry(&mut self) {
        for option in self.options.values_mut() {
            let Some(definition) = known_option(&option.aliases) else {
                continue;
            };
            option.category = definition.category;
            option.known_key = Some(definition.key.to_string());
            option.display_name = definition.label.to_string();
            option.summary = Some(definition.summary.to_string());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaOption {
    /// The preferred long flag, or the first advertised flag when no long spelling exists.
    pub flag: String,
    /// Every spelling advertised by this exact runtime, including the canonical one.
    pub aliases: Vec<String>,
    pub value_hint: Option<String>,
    /// The untouched description reconstructed from the runtime's help text.
    pub description: String,
    /// The upstream help section, retained even when the option is not in our known registry.
    pub section: String,
    pub category: LlamaOptionCategory,
    pub known_key: Option<String>,
    pub display_name: String,
    /// Stable explanatory copy for important concepts; unknown flags intentionally have none.
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LlamaOptionCategory {
    Context,
    Batching,
    Gpu,
    Memory,
    Parallelism,
    Templates,
    Reasoning,
    Speculative,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaDevice {
    pub id: String,
    pub name: String,
    pub backend: Option<String>,
    pub memory_total_mib: Option<u64>,
    pub memory_free_mib: Option<u64>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

impl RawCommandOutput {
    pub fn combined(&self) -> String {
        match (self.stdout.trim().is_empty(), self.stderr.trim().is_empty()) {
            (false, false) => format!("{}\n{}", self.stdout.trim_end(), self.stderr.trim_end()),
            (false, true) => self.stdout.trim_end().to_string(),
            (true, false) => self.stderr.trim_end().to_string(),
            (true, true) => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaRawOutputs {
    pub version: RawCommandOutput,
    pub help: RawCommandOutput,
    pub devices: RawCommandOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInspection {
    pub capabilities: LlamaCapabilities,
    pub raw: LlamaRawOutputs,
}

#[derive(Debug, Clone, Copy)]
pub struct KnownOptionDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub category: LlamaOptionCategory,
    pub flags: &'static [&'static str],
}

/// Stable concepts used by later profile editors. A definition only becomes active when one of
/// its flags is actually present in the selected binary's `--help` output.
pub const KNOWN_OPTIONS: &[KnownOptionDefinition] = &[
    KnownOptionDefinition {
        key: "contextSize",
        label: "Context size",
        summary: "Maximum context window used by the model.",
        category: LlamaOptionCategory::Context,
        flags: &["-c", "--ctx-size"],
    },
    KnownOptionDefinition {
        key: "batchSize",
        label: "Logical batch size",
        summary: "Maximum number of tokens processed in one logical batch.",
        category: LlamaOptionCategory::Batching,
        flags: &["-b", "--batch-size"],
    },
    KnownOptionDefinition {
        key: "microBatchSize",
        label: "Physical batch size",
        summary: "Maximum number of tokens processed in one physical micro-batch.",
        category: LlamaOptionCategory::Batching,
        flags: &["-ub", "--ubatch-size"],
    },
    KnownOptionDefinition {
        key: "gpuLayers",
        label: "GPU layers",
        summary: "Number of model layers offloaded to accelerator devices.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-ngl", "--gpu-layers", "--n-gpu-layers"],
    },
    KnownOptionDefinition {
        key: "device",
        label: "Devices",
        summary: "Ordered accelerator devices used for model offload.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-dev", "--device"],
    },
    KnownOptionDefinition {
        key: "splitMode",
        label: "Split mode",
        summary: "How model tensors are distributed across multiple devices.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-sm", "--split-mode"],
    },
    KnownOptionDefinition {
        key: "tensorSplit",
        label: "Tensor split",
        summary: "Explicit per-device proportions for tensor placement.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-ts", "--tensor-split"],
    },
    KnownOptionDefinition {
        key: "mainGpu",
        label: "Main GPU",
        summary: "Primary device used by split modes that require one.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-mg", "--main-gpu"],
    },
    KnownOptionDefinition {
        key: "flashAttention",
        label: "Flash attention",
        summary: "Use the runtime's optimized flash-attention implementation.",
        category: LlamaOptionCategory::Gpu,
        flags: &["-fa", "--flash-attn", "--no-flash-attn"],
    },
    KnownOptionDefinition {
        key: "kvCacheTypeK",
        label: "K cache type",
        summary: "Data type used for key tensors in the KV cache.",
        category: LlamaOptionCategory::Memory,
        flags: &["-ctk", "--cache-type-k"],
    },
    KnownOptionDefinition {
        key: "kvCacheTypeV",
        label: "V cache type",
        summary: "Data type used for value tensors in the KV cache.",
        category: LlamaOptionCategory::Memory,
        flags: &["-ctv", "--cache-type-v"],
    },
    KnownOptionDefinition {
        key: "kvCacheOffload",
        label: "KV cache offload",
        summary: "Keep the KV cache on accelerator devices when supported.",
        category: LlamaOptionCategory::Memory,
        flags: &["-kvo", "--kv-offload", "-nkvo", "--no-kv-offload"],
    },
    KnownOptionDefinition {
        key: "kvUnified",
        label: "Unified KV buffer",
        summary: "Share one KV buffer across all server slots.",
        category: LlamaOptionCategory::Memory,
        flags: &["-kvu", "--kv-unified", "-no-kvu", "--no-kv-unified"],
    },
    KnownOptionDefinition {
        key: "swaFull",
        label: "Full SWA cache",
        summary: "Use a full-size cache for sliding-window attention layers.",
        category: LlamaOptionCategory::Memory,
        flags: &["--swa-full"],
    },
    KnownOptionDefinition {
        key: "fit",
        label: "Fit to memory",
        summary: "Ask llama.cpp to adjust parameters to available device memory.",
        category: LlamaOptionCategory::Memory,
        flags: &[
            "-fit",
            "--fit",
            "-fitp",
            "--fit-print",
            "-fitt",
            "--fit-target",
            "-fitc",
            "--fit-ctx",
        ],
    },
    KnownOptionDefinition {
        key: "parallel",
        label: "Parallel slots",
        summary: "Number of requests the server may process in parallel.",
        category: LlamaOptionCategory::Parallelism,
        flags: &["-np", "--parallel"],
    },
    KnownOptionDefinition {
        key: "jinja",
        label: "Jinja templates",
        summary: "Enable Jinja chat templates and tool-aware template features.",
        category: LlamaOptionCategory::Templates,
        flags: &["--jinja", "--no-jinja"],
    },
    KnownOptionDefinition {
        key: "reasoning",
        label: "Reasoning",
        summary: "Control extraction and presentation of model reasoning content.",
        category: LlamaOptionCategory::Reasoning,
        flags: &[
            "-rea",
            "--reasoning",
            "--reasoning-format",
            "--reasoning-effort",
            "--reasoning-budget",
            "--reasoning-budget-message",
            "--reasoning-preserve",
        ],
    },
    KnownOptionDefinition {
        key: "speculativeType",
        label: "Speculative decoding",
        summary: "Select one or more speculative decoding strategies advertised by this runtime.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-type"],
    },
    KnownOptionDefinition {
        key: "draftModel",
        label: "Draft model",
        summary: "Model used to draft candidate tokens for speculative decoding.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-model", "-md", "--model-draft"],
    },
    KnownOptionDefinition {
        key: "draftDevice",
        label: "Draft devices",
        summary: "Devices used to offload the speculative draft model.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-device", "-devd", "--device-draft"],
    },
    KnownOptionDefinition {
        key: "draftGpuLayers",
        label: "Draft GPU layers",
        summary: "Number of draft-model layers offloaded to accelerator devices.",
        category: LlamaOptionCategory::Speculative,
        flags: &[
            "--spec-draft-gpu-layers",
            "--spec-draft-ngl",
            "-ngld",
            "--gpu-layers-draft",
            "--n-gpu-layers-draft",
        ],
    },
    KnownOptionDefinition {
        key: "draftKvCacheTypeK",
        label: "Draft K cache type",
        summary: "Data type used for key tensors in the draft model's KV cache.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-type-k", "-ctkd", "--cache-type-k-draft"],
    },
    KnownOptionDefinition {
        key: "draftKvCacheTypeV",
        label: "Draft V cache type",
        summary: "Data type used for value tensors in the draft model's KV cache.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-type-v", "-ctvd", "--cache-type-v-draft"],
    },
    KnownOptionDefinition {
        key: "specDraftMaxTokens",
        label: "Maximum draft tokens",
        summary: "Maximum number of candidate tokens produced in one speculative step.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-n-max"],
    },
    KnownOptionDefinition {
        key: "specDraftMinTokens",
        label: "Minimum draft tokens",
        summary: "Minimum accepted draft length before speculative verification is used.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-n-min"],
    },
    KnownOptionDefinition {
        key: "specDraftSplitProbability",
        label: "Draft split probability",
        summary: "Probability threshold used when splitting speculative draft branches.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-p-split", "--draft-p-split"],
    },
    KnownOptionDefinition {
        key: "specDraftMinProbability",
        label: "Minimum draft probability",
        summary: "Minimum token probability retained by greedy draft decoding.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-draft-p-min", "--draft-p-min"],
    },
    KnownOptionDefinition {
        key: "specDraftBackendSampling",
        label: "Draft backend sampling",
        summary: "Run supported draft-model samplers on the accelerator backend.",
        category: LlamaOptionCategory::Speculative,
        flags: &[
            "--spec-draft-backend-sampling",
            "--no-spec-draft-backend-sampling",
        ],
    },
    KnownOptionDefinition {
        key: "ngramModMinTokens",
        label: "ngram-mod minimum tokens",
        summary: "Minimum draft length produced by the ngram-mod strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-mod-n-min"],
    },
    KnownOptionDefinition {
        key: "ngramModMaxTokens",
        label: "ngram-mod maximum tokens",
        summary: "Maximum draft length produced by the ngram-mod strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-mod-n-max"],
    },
    KnownOptionDefinition {
        key: "ngramModMatchTokens",
        label: "ngram-mod match length",
        summary: "Lookup length used by the ngram-mod strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-mod-n-match"],
    },
    KnownOptionDefinition {
        key: "ngramSimpleSizeN",
        label: "ngram-simple lookup length",
        summary: "Lookup n-gram length used by the ngram-simple strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-simple-size-n"],
    },
    KnownOptionDefinition {
        key: "ngramSimpleSizeM",
        label: "ngram-simple draft length",
        summary: "Draft m-gram length used by the ngram-simple strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-simple-size-m"],
    },
    KnownOptionDefinition {
        key: "ngramSimpleMinHits",
        label: "ngram-simple minimum hits",
        summary: "Minimum number of matching history entries required by ngram-simple.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-simple-min-hits"],
    },
    KnownOptionDefinition {
        key: "ngramMapKSizeN",
        label: "ngram-map-k lookup length",
        summary: "Lookup n-gram length used by the ngram-map-k strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k-size-n"],
    },
    KnownOptionDefinition {
        key: "ngramMapKSizeM",
        label: "ngram-map-k draft length",
        summary: "Draft m-gram length used by the ngram-map-k strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k-size-m"],
    },
    KnownOptionDefinition {
        key: "ngramMapKMinHits",
        label: "ngram-map-k minimum hits",
        summary: "Minimum number of matching history entries required by ngram-map-k.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k-min-hits"],
    },
    KnownOptionDefinition {
        key: "ngramMapK4vSizeN",
        label: "ngram-map-k4v lookup length",
        summary: "Lookup n-gram length used by the ngram-map-k4v strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k4v-size-n"],
    },
    KnownOptionDefinition {
        key: "ngramMapK4vSizeM",
        label: "ngram-map-k4v draft length",
        summary: "Draft m-gram length used by the ngram-map-k4v strategy.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k4v-size-m"],
    },
    KnownOptionDefinition {
        key: "ngramMapK4vMinHits",
        label: "ngram-map-k4v minimum hits",
        summary: "Minimum number of matching history entries required by ngram-map-k4v.",
        category: LlamaOptionCategory::Speculative,
        flags: &["--spec-ngram-map-k4v-min-hits"],
    },
];

pub fn known_option(flags: &[String]) -> Option<&'static KnownOptionDefinition> {
    KNOWN_OPTIONS.iter().find(|definition| {
        definition
            .flags
            .iter()
            .any(|known| flags.iter().any(|actual| actual == known))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_every_specialized_profile_concept() {
        let keys: Vec<_> = KNOWN_OPTIONS.iter().map(|option| option.key).collect();
        for required in [
            "contextSize",
            "batchSize",
            "microBatchSize",
            "gpuLayers",
            "device",
            "splitMode",
            "tensorSplit",
            "mainGpu",
            "flashAttention",
            "kvCacheTypeK",
            "kvCacheTypeV",
            "kvCacheOffload",
            "kvUnified",
            "swaFull",
            "fit",
            "parallel",
            "jinja",
            "reasoning",
            "speculativeType",
            "draftModel",
            "draftDevice",
            "draftGpuLayers",
            "draftKvCacheTypeK",
            "draftKvCacheTypeV",
            "specDraftMaxTokens",
            "specDraftMinTokens",
            "specDraftSplitProbability",
            "specDraftMinProbability",
            "specDraftBackendSampling",
            "ngramModMinTokens",
            "ngramModMaxTokens",
            "ngramModMatchTokens",
            "ngramSimpleSizeN",
            "ngramSimpleSizeM",
            "ngramSimpleMinHits",
            "ngramMapKSizeN",
            "ngramMapKSizeM",
            "ngramMapKMinHits",
            "ngramMapK4vSizeN",
            "ngramMapK4vSizeM",
            "ngramMapK4vMinHits",
        ] {
            assert!(keys.contains(&required), "missing known concept {required}");
        }
    }

    #[test]
    fn current_registry_enriches_options_loaded_from_older_manifests() {
        let option = LlamaOption {
            flag: "--spec-ngram-mod-n-max".into(),
            aliases: vec!["--spec-ngram-mod-n-max".into()],
            value_hint: Some("N".into()),
            description: "runtime-owned description".into(),
            section: "speculative params".into(),
            category: LlamaOptionCategory::Advanced,
            known_key: None,
            display_name: "Spec ngram mod n max".into(),
            summary: None,
        };
        let mut capabilities = LlamaCapabilities {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            version: "old manifest".into(),
            commit: None,
            options: BTreeMap::from([(option.flag.clone(), option)]),
            speculative_types: vec!["ngram-mod".into()],
            devices: Vec::new(),
        };

        capabilities.refresh_known_registry();

        let refreshed = &capabilities.options["--spec-ngram-mod-n-max"];
        assert_eq!(refreshed.known_key.as_deref(), Some("ngramModMaxTokens"));
        assert_eq!(refreshed.category, LlamaOptionCategory::Speculative);
        assert_eq!(refreshed.description, "runtime-owned description");
    }
}
