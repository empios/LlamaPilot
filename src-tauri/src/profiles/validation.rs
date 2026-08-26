use std::collections::BTreeSet;
use std::path::Path;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::capabilities::{LlamaCapabilities, LlamaOption};

use super::record::{ProfileInput, ProfileOptionSetting};

const NON_QUANTIZED_CACHE_TYPES: &[&str] = &["f32", "f16", "bf16"];
const EXTERNAL_DRAFT_TYPES: &[&str] = &[
    "draft-simple",
    "draft-eagle3",
    "draft-mtp",
    "draft-dflash",
    "draft-dspark",
];

pub fn validate_advanced_options(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
) -> AppResult<Vec<String>> {
    let mut warnings = BTreeSet::new();

    validate_cache_types(input, capabilities)?;
    validate_multi_gpu(input, capabilities, &mut warnings)?;
    validate_speculative(input, capabilities, &mut warnings)?;

    Ok(warnings.into_iter().collect())
}

fn validate_cache_types(input: &ProfileInput, capabilities: &LlamaCapabilities) -> AppResult<()> {
    for key in [
        "kvCacheTypeK",
        "kvCacheTypeV",
        "draftKvCacheTypeK",
        "draftKvCacheTypeV",
    ] {
        let Some((setting, option)) = setting_for(input, capabilities, key) else {
            continue;
        };
        let Some(value) = custom_value(setting) else {
            continue;
        };
        let allowed = advertised_values(option);
        if !allowed.is_empty() && !allowed.iter().any(|allowed| allowed == value) {
            return Err(invalid_profile(format!(
                "{} does not support cache type {value:?}.",
                option.display_name
            ))
            .with_hint(format!("Use one of: {}.", allowed.join(", "))));
        }
    }
    Ok(())
}

fn validate_multi_gpu(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    warnings: &mut BTreeSet<String>,
) -> AppResult<()> {
    let split_mode =
        custom_setting(input, capabilities, "splitMode").map(|value| value.to_ascii_lowercase());
    if let Some(value) = split_mode.as_deref() {
        let option = option_for_known_key(capabilities, "splitMode").expect("setting has option");
        let choices = value_hint_choices(option);
        if !choices.is_empty() && !choices.iter().any(|choice| choice == value) {
            return Err(invalid_profile(format!(
                "Split mode {value:?} is not advertised by this runtime."
            ))
            .with_hint(format!("Use one of: {}.", choices.join(", "))));
        }
    }

    let selected_devices = custom_setting(input, capabilities, "device")
        .map(parse_csv)
        .transpose()?;
    if let Some(devices) = &selected_devices {
        if devices.is_empty() {
            return Err(invalid_profile(
                "Choose at least one device or use Default.",
            ));
        }
        if devices.iter().any(|device| device == "none") && devices.len() > 1 {
            return Err(invalid_profile(
                "Device value 'none' cannot be combined with accelerator devices.",
            ));
        }
        if !capabilities.devices.is_empty() {
            for device in devices {
                if device != "none"
                    && !capabilities
                        .devices
                        .iter()
                        .any(|advertised| advertised.id == *device)
                {
                    return Err(invalid_profile(format!(
                        "Device {device:?} was not reported by the selected runtime."
                    ))
                    .with_hint("Inspect the runtime again or choose one of its listed devices."));
                }
            }
        }
    }

    if let Some(value) = custom_setting(input, capabilities, "tensorSplit") {
        if split_mode.as_deref() == Some("none") {
            return Err(
                invalid_profile("Tensor split cannot be used with single-GPU split mode.")
                    .with_hint(
                        "Use layer, row, or tensor mode, or return Tensor split to Default.",
                    ),
            );
        }
        let proportions = parse_non_negative_numbers(value, "Tensor split")?;
        if !proportions.iter().any(|value| *value > 0.0) {
            return Err(invalid_profile(
                "Tensor split must assign a positive proportion to at least one device.",
            ));
        }
        if let Some(devices) = &selected_devices {
            if devices.iter().any(|device| device == "none") {
                return Err(invalid_profile(
                    "Tensor split cannot be used when accelerator devices are disabled.",
                ));
            }
            if proportions.len() != devices.len() {
                return Err(invalid_profile(format!(
                    "Tensor split has {} proportions for {} explicitly selected devices.",
                    proportions.len(),
                    devices.len()
                ))
                .with_hint("Provide one comma-separated proportion per device, in device order."));
            }
        } else if !capabilities.devices.is_empty() && proportions.len() > capabilities.devices.len()
        {
            return Err(invalid_profile(format!(
                "Tensor split has more proportions ({}) than detected devices ({}).",
                proportions.len(),
                capabilities.devices.len()
            )));
        }
    }

    if let Some(value) = custom_setting(input, capabilities, "mainGpu") {
        let index = parse_integer(value, "Main GPU")?;
        if index < 0 {
            return Err(invalid_profile("Main GPU index cannot be negative."));
        }
        if selected_devices
            .as_ref()
            .is_some_and(|devices| devices.iter().any(|device| device == "none"))
        {
            return Err(invalid_profile(
                "Main GPU cannot be selected when accelerator devices are disabled.",
            ));
        }
        let available = selected_devices
            .as_ref()
            .filter(|devices| !devices.iter().any(|device| device == "none"))
            .map(Vec::len)
            .unwrap_or(capabilities.devices.len());
        if available > 0 && index as usize >= available {
            return Err(invalid_profile(format!(
                "Main GPU index {index} is outside the {available} selected/detected devices."
            )));
        }
        if matches!(split_mode.as_deref(), Some("layer" | "tensor")) {
            warnings.insert(
                "Main GPU is ignored by layer and tensor split modes; it only affects none and row."
                    .into(),
            );
        }
    }

    let flash_disabled = setting_boolean(input, capabilities, "flashAttention") == Some(false);
    let cache_k = custom_setting(input, capabilities, "kvCacheTypeK");
    let cache_v = custom_setting(input, capabilities, "kvCacheTypeV");
    let quantized_k = cache_k.is_some_and(is_quantized_cache_type);
    let quantized_v = cache_v.is_some_and(is_quantized_cache_type);

    if quantized_v && flash_disabled {
        return Err(invalid_profile(
            "Quantized V cache cannot be combined with disabled Flash Attention.",
        )
        .with_hint("Enable Flash Attention, use Auto, or choose f32/f16/bf16 for the V cache."));
    }

    if split_mode.as_deref() == Some("tensor") {
        if flash_disabled {
            return Err(
                invalid_profile("Tensor split mode requires Flash Attention.")
                    .with_hint("Set Flash Attention to Enabled or Auto."),
            );
        }
        if quantized_k || quantized_v {
            return Err(invalid_profile(
                "Tensor split mode does not support quantized KV cache types.",
            )
            .with_hint("Use f32, f16, or bf16 for both K and V cache types."));
        }
        warnings.insert(
            "Tensor split is experimental, architecture-dependent, and requires a supported Flash Attention backend."
                .into(),
        );
        if setting_boolean(input, capabilities, "kvCacheOffload") == Some(false) {
            warnings.insert(
                "Tensor split normally distributes KV across GPUs, but KV cache offload is disabled."
                    .into(),
            );
        }
        if setting_boolean_by_alias(input, capabilities, "--fit") == Some(true) {
            return Err(invalid_profile(
                "Automatic memory fitting is not supported with tensor split mode.",
            )
            .with_hint(
                "Set Fit to off/default and size context, parallel slots, and GPU layers manually.",
            ));
        }
    }

    Ok(())
}

fn validate_speculative(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    warnings: &mut BTreeSet<String>,
) -> AppResult<()> {
    let selected = custom_setting(input, capabilities, "speculativeType")
        .map(parse_csv)
        .transpose()?
        .unwrap_or_default();
    let selected_set: BTreeSet<_> = selected.iter().map(String::as_str).collect();
    if selected_set.contains("none") && selected_set.len() > 1 {
        return Err(invalid_profile(
            "Speculative type 'none' cannot be combined with another strategy.",
        ));
    }
    if !capabilities.speculative_types.is_empty() {
        for strategy in &selected {
            if !capabilities
                .speculative_types
                .iter()
                .any(|advertised| advertised == strategy)
            {
                return Err(invalid_profile(format!(
                    "Speculative strategy {strategy:?} is not advertised by this runtime."
                ))
                .with_hint(format!(
                    "Use one of: {}.",
                    capabilities.speculative_types.join(", ")
                )));
            }
        }
    }

    let active: BTreeSet<_> = selected_set
        .iter()
        .copied()
        .filter(|strategy| *strategy != "none")
        .collect();
    let has_any_spec_setting = input.options.iter().any(|(stored_key, setting)| {
        *setting != ProfileOptionSetting::Default
            && resolve_option(capabilities, stored_key)
                .and_then(|option| option.known_key.as_deref())
                .is_some_and(|key| key != "speculativeType" && is_speculative_key(key))
    });
    if active.is_empty() && has_any_spec_setting {
        return Err(invalid_profile(
            "Speculative tuning is set, but no speculative strategy is enabled.",
        )
        .with_hint("Choose a strategy or return its tuning fields to Default."));
    }

    let uses_external_draft = EXTERNAL_DRAFT_TYPES
        .iter()
        .any(|strategy| active.contains(strategy));
    let external_keys = [
        "draftModel",
        "draftDevice",
        "draftGpuLayers",
        "draftKvCacheTypeK",
        "draftKvCacheTypeV",
    ];
    if uses_external_draft {
        let draft_model = custom_setting(input, capabilities, "draftModel")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                invalid_profile("The selected draft strategy requires a draft model.")
                    .with_hint("Choose a complete local draft GGUF model.")
            })?;
        if !Path::new(draft_model).is_file() {
            return Err(invalid_profile("The selected draft model file is missing.")
                .with_details(draft_model.to_string()));
        }
    } else {
        reject_configured_keys(
            input,
            capabilities,
            &external_keys,
            "External draft-model settings require draft-simple, draft-eagle3, draft-mtp, draft-dflash, or draft-dspark.",
        )?;
    }

    let uses_draft_strategy = active.iter().any(|strategy| strategy.starts_with("draft-"));
    if !uses_draft_strategy {
        reject_configured_keys(
            input,
            capabilities,
            &[
                "specDraftMinTokens",
                "specDraftSplitProbability",
                "specDraftMinProbability",
                "specDraftBackendSampling",
            ],
            "These draft controls require an active draft-* strategy.",
        )?;
    }

    for (strategy, keys) in [
        (
            "ngram-mod",
            &[
                "ngramModMinTokens",
                "ngramModMaxTokens",
                "ngramModMatchTokens",
            ][..],
        ),
        (
            "ngram-simple",
            &["ngramSimpleSizeN", "ngramSimpleSizeM", "ngramSimpleMinHits"][..],
        ),
        (
            "ngram-map-k",
            &["ngramMapKSizeN", "ngramMapKSizeM", "ngramMapKMinHits"][..],
        ),
        (
            "ngram-map-k4v",
            &["ngramMapK4vSizeN", "ngramMapK4vSizeM", "ngramMapK4vMinHits"][..],
        ),
    ] {
        if !active.contains(strategy) {
            reject_configured_keys(
                input,
                capabilities,
                keys,
                &format!("These controls require the {strategy} strategy."),
            )?;
        }
    }

    validate_integer_range(input, capabilities, "specDraftMaxTokens", 0, i64::MAX)?;
    validate_integer_range(input, capabilities, "specDraftMinTokens", 0, i64::MAX)?;
    validate_min_max(
        input,
        capabilities,
        "specDraftMinTokens",
        "specDraftMaxTokens",
        "Minimum draft tokens cannot exceed maximum draft tokens.",
    )?;
    validate_probability(input, capabilities, "specDraftSplitProbability")?;
    validate_probability(input, capabilities, "specDraftMinProbability")?;

    validate_integer_range(input, capabilities, "ngramModMinTokens", 0, 1024)?;
    validate_integer_range(input, capabilities, "ngramModMaxTokens", 0, 1024)?;
    validate_integer_range(input, capabilities, "ngramModMatchTokens", 1, 1024)?;
    validate_min_max(
        input,
        capabilities,
        "ngramModMinTokens",
        "ngramModMaxTokens",
        "ngram-mod minimum tokens cannot exceed its maximum tokens.",
    )?;
    for key in [
        "ngramSimpleSizeN",
        "ngramSimpleSizeM",
        "ngramMapKSizeN",
        "ngramMapKSizeM",
        "ngramMapK4vSizeN",
        "ngramMapK4vSizeM",
    ] {
        validate_integer_range(input, capabilities, key, 1, 1024)?;
    }
    for key in [
        "ngramSimpleMinHits",
        "ngramMapKMinHits",
        "ngramMapK4vMinHits",
    ] {
        validate_integer_range(input, capabilities, key, 1, i64::MAX)?;
    }

    if (active.contains("draft-dflash") || active.contains("draft-dspark"))
        && custom_setting(input, capabilities, "specDraftMaxTokens").is_some()
    {
        warnings.insert(
            "DFlash and DSpark clamp maximum draft tokens to the draft model's trained block size."
                .into(),
        );
    }

    Ok(())
}

fn reject_configured_keys(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    keys: &[&str],
    message: &str,
) -> AppResult<()> {
    if keys
        .iter()
        .any(|key| setting_for(input, capabilities, key).is_some())
    {
        return Err(invalid_profile(message));
    }
    Ok(())
}

fn validate_integer_range(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    key: &str,
    minimum: i64,
    maximum: i64,
) -> AppResult<()> {
    let Some(value) = custom_setting(input, capabilities, key) else {
        return Ok(());
    };
    let parsed = parse_integer(value, key)?;
    if parsed < minimum || parsed > maximum {
        return Err(invalid_profile(format!(
            "{key} must be between {minimum} and {maximum}."
        )));
    }
    Ok(())
}

fn validate_probability(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    key: &str,
) -> AppResult<()> {
    let Some(value) = custom_setting(input, capabilities, key) else {
        return Ok(());
    };
    let parsed = value
        .parse::<f64>()
        .map_err(|_| invalid_profile(format!("{key} must be a number between 0 and 1.")))?;
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(invalid_profile(format!(
            "{key} must be a number between 0 and 1."
        )));
    }
    Ok(())
}

fn validate_min_max(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    minimum_key: &str,
    maximum_key: &str,
    message: &str,
) -> AppResult<()> {
    let Some(minimum) = custom_setting(input, capabilities, minimum_key) else {
        return Ok(());
    };
    let Some(maximum) = custom_setting(input, capabilities, maximum_key) else {
        return Ok(());
    };
    if parse_integer(minimum, minimum_key)? > parse_integer(maximum, maximum_key)? {
        return Err(invalid_profile(message));
    }
    Ok(())
}

fn setting_for<'a>(
    input: &'a ProfileInput,
    capabilities: &'a LlamaCapabilities,
    known_key: &str,
) -> Option<(&'a ProfileOptionSetting, &'a LlamaOption)> {
    input.options.iter().find_map(|(stored_key, setting)| {
        let option = resolve_option(capabilities, stored_key)?;
        (option.known_key.as_deref() == Some(known_key)).then_some((setting, option))
    })
}

fn custom_setting<'a>(
    input: &'a ProfileInput,
    capabilities: &'a LlamaCapabilities,
    known_key: &str,
) -> Option<&'a str> {
    setting_for(input, capabilities, known_key).and_then(|(setting, _)| custom_value(setting))
}

fn custom_value(setting: &ProfileOptionSetting) -> Option<&str> {
    match setting {
        ProfileOptionSetting::Custom { value } => Some(value.trim()),
        ProfileOptionSetting::Default | ProfileOptionSetting::Auto => None,
    }
}

fn setting_boolean(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    known_key: &str,
) -> Option<bool> {
    setting_for(input, capabilities, known_key)
        .and_then(|(setting, _)| custom_value(setting))
        .and_then(parse_boolean)
}

fn setting_boolean_by_alias(
    input: &ProfileInput,
    capabilities: &LlamaCapabilities,
    alias: &str,
) -> Option<bool> {
    input.options.iter().find_map(|(stored_key, setting)| {
        let option = resolve_option(capabilities, stored_key)?;
        option
            .aliases
            .iter()
            .any(|actual| actual == alias)
            .then(|| custom_value(setting).and_then(parse_boolean))
            .flatten()
    })
}

fn parse_boolean(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "on" | "enabled" | "1" => Some(true),
        "false" | "off" | "disabled" | "0" => Some(false),
        _ => None,
    }
}

fn resolve_option<'a>(capabilities: &'a LlamaCapabilities, key: &str) -> Option<&'a LlamaOption> {
    capabilities.options.values().find(|option| {
        option.known_key.as_deref() == Some(key)
            || option.flag == key
            || option.aliases.iter().any(|alias| alias == key)
    })
}

fn option_for_known_key<'a>(
    capabilities: &'a LlamaCapabilities,
    key: &str,
) -> Option<&'a LlamaOption> {
    capabilities
        .options
        .values()
        .find(|option| option.known_key.as_deref() == Some(key))
}

fn advertised_values(option: &LlamaOption) -> Vec<String> {
    option
        .description
        .lines()
        .find_map(|line| {
            let (label, values) = line.split_once(':')?;
            label
                .trim()
                .eq_ignore_ascii_case("allowed values")
                .then_some(values)
        })
        .map(|values| {
            values
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn value_hint_choices(option: &LlamaOption) -> Vec<String> {
    option
        .value_hint
        .as_deref()
        .unwrap_or_default()
        .trim_matches(|character| matches!(character, '<' | '>' | '[' | ']' | '{' | '}'))
        .split([',', '|'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn parse_csv(value: &str) -> AppResult<Vec<String>> {
    let values: Vec<_> = value
        .split(',')
        .map(|part| part.trim().to_string())
        .collect();
    if values.iter().any(String::is_empty) {
        return Err(invalid_profile(
            "Comma-separated values cannot contain an empty entry.",
        ));
    }
    Ok(values)
}

fn parse_non_negative_numbers(value: &str, label: &str) -> AppResult<Vec<f64>> {
    let values: Vec<_> = value
        .split([',', '/'])
        .map(|part| {
            let parsed = part.trim().parse::<f64>().map_err(|_| {
                invalid_profile(format!("{label} must contain only numeric proportions."))
            })?;
            if !parsed.is_finite() || parsed < 0.0 {
                return Err(invalid_profile(format!(
                    "{label} proportions must be finite and non-negative."
                )));
            }
            Ok(parsed)
        })
        .collect::<AppResult<_>>()?;
    if values.is_empty() {
        return Err(invalid_profile(format!("{label} cannot be empty.")));
    }
    Ok(values)
}

fn parse_integer(value: &str, label: &str) -> AppResult<i64> {
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| invalid_profile(format!("{label} must be a whole number.")))
}

fn is_quantized_cache_type(value: &str) -> bool {
    !NON_QUANTIZED_CACHE_TYPES
        .iter()
        .any(|allowed| value.eq_ignore_ascii_case(allowed))
}

fn is_speculative_key(key: &str) -> bool {
    key.starts_with("draft") || key.starts_with("specDraft") || key.starts_with("ngram")
}

fn invalid_profile(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::InvalidProfile, message)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::llama::capabilities::{
        LlamaDevice, LlamaOptionCategory, CAPABILITIES_SCHEMA_VERSION,
    };

    use super::*;

    fn option(
        flag: &str,
        aliases: &[&str],
        value_hint: Option<&str>,
        known_key: &str,
        description: &str,
    ) -> LlamaOption {
        LlamaOption {
            flag: flag.into(),
            aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
            value_hint: value_hint.map(str::to_string),
            description: description.into(),
            section: "common params".into(),
            category: LlamaOptionCategory::Advanced,
            known_key: Some(known_key.into()),
            display_name: known_key.into(),
            summary: None,
        }
    }

    fn capabilities() -> LlamaCapabilities {
        let options = [
            option(
                "--split-mode",
                &["-sm", "--split-mode"],
                Some("{none,layer,row,tensor}"),
                "splitMode",
                "",
            ),
            option(
                "--tensor-split",
                &["-ts", "--tensor-split"],
                Some("N0,N1,..."),
                "tensorSplit",
                "",
            ),
            option(
                "--device",
                &["-dev", "--device"],
                Some("<dev1,dev2,..>"),
                "device",
                "",
            ),
            option(
                "--main-gpu",
                &["-mg", "--main-gpu"],
                Some("INDEX"),
                "mainGpu",
                "",
            ),
            option(
                "--flash-attn",
                &["--flash-attn", "--no-flash-attn"],
                None,
                "flashAttention",
                "",
            ),
            option(
                "--cache-type-k",
                &["--cache-type-k"],
                Some("TYPE"),
                "kvCacheTypeK",
                "allowed values: f32, f16, q8_0, q4_0",
            ),
            option(
                "--cache-type-v",
                &["--cache-type-v"],
                Some("TYPE"),
                "kvCacheTypeV",
                "allowed values: f32, f16, q8_0, q4_0",
            ),
            option(
                "--spec-type",
                &["--spec-type"],
                Some("none,draft-mtp,draft-dflash,ngram-mod"),
                "speculativeType",
                "",
            ),
            option(
                "--spec-draft-model",
                &["--spec-draft-model"],
                Some("FNAME"),
                "draftModel",
                "",
            ),
            option(
                "--spec-draft-n-max",
                &["--spec-draft-n-max"],
                Some("N"),
                "specDraftMaxTokens",
                "",
            ),
            option(
                "--spec-ngram-mod-n-min",
                &["--spec-ngram-mod-n-min"],
                Some("N"),
                "ngramModMinTokens",
                "",
            ),
            option(
                "--spec-ngram-mod-n-max",
                &["--spec-ngram-mod-n-max"],
                Some("N"),
                "ngramModMaxTokens",
                "",
            ),
        ]
        .into_iter()
        .map(|option| (option.flag.clone(), option))
        .collect();
        LlamaCapabilities {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            version: "test".into(),
            commit: None,
            options,
            speculative_types: vec![
                "none".into(),
                "draft-mtp".into(),
                "draft-dflash".into(),
                "ngram-mod".into(),
            ],
            devices: vec![
                LlamaDevice {
                    id: "CUDA0".into(),
                    name: "GPU 0".into(),
                    backend: Some("CUDA".into()),
                    memory_total_mib: None,
                    memory_free_mib: None,
                    raw: String::new(),
                },
                LlamaDevice {
                    id: "CUDA1".into(),
                    name: "GPU 1".into(),
                    backend: Some("CUDA".into()),
                    memory_total_mib: None,
                    memory_free_mib: None,
                    raw: String::new(),
                },
            ],
        }
    }

    fn input() -> ProfileInput {
        ProfileInput {
            name: "test".into(),
            description: None,
            runtime_id: "runtime".into(),
            model_id: "model".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: false,
            options: BTreeMap::new(),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
        }
    }

    fn custom(value: &str) -> ProfileOptionSetting {
        ProfileOptionSetting::Custom {
            value: value.into(),
        }
    }

    #[test]
    fn rejects_tensor_mode_with_quantized_cache_or_disabled_flash() {
        let mut input = input();
        input.options.insert("splitMode".into(), custom("tensor"));
        input.options.insert("kvCacheTypeV".into(), custom("q8_0"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());

        input.options.insert("kvCacheTypeV".into(), custom("f16"));
        input
            .options
            .insert("flashAttention".into(), custom("false"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());
    }

    #[test]
    fn validates_tensor_split_shape_and_emits_experimental_warning() {
        let mut input = input();
        input.options.insert("splitMode".into(), custom("tensor"));
        input.options.insert("tensorSplit".into(), custom("3,1"));
        let warnings = validate_advanced_options(&input, &capabilities()).expect("valid split");
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("experimental")));

        input.options.insert("tensorSplit".into(), custom("3,-1"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());
    }

    #[test]
    fn device_none_is_exclusive_and_disables_main_gpu_selection() {
        let mut input = input();
        input.options.insert("device".into(), custom("none,CUDA0"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());

        input.options.insert("device".into(), custom("none"));
        input.options.insert("mainGpu".into(), custom("0"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());
    }

    #[test]
    fn rejects_strategy_tuning_when_that_strategy_is_inactive() {
        let mut input = input();
        input
            .options
            .insert("speculativeType".into(), custom("ngram-mod"));
        input
            .options
            .insert("specDraftMaxTokens".into(), custom("64"));
        input
            .options
            .insert("ngramModMinTokens".into(), custom("80"));
        input
            .options
            .insert("ngramModMaxTokens".into(), custom("64"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());

        input.options.remove("ngramModMinTokens");
        input.options.remove("ngramModMaxTokens");
        input
            .options
            .insert("draftModel".into(), custom("missing.gguf"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());
    }

    #[test]
    fn mtp_requires_and_accepts_an_external_draft_model() {
        let draft = tempfile::NamedTempFile::new().expect("draft fixture");
        let draft_path = draft.path().to_string_lossy().into_owned();
        let mut input = input();
        input
            .options
            .insert("speculativeType".into(), custom("draft-mtp"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());

        input
            .options
            .insert("draftModel".into(), custom(&draft_path));
        validate_advanced_options(&input, &capabilities()).expect("valid MTP drafter");
    }

    #[test]
    fn accepts_runtime_advertised_cache_types_and_rejects_other_values() {
        let mut input = input();
        input.options.insert("kvCacheTypeK".into(), custom("q4_0"));
        validate_advanced_options(&input, &capabilities()).expect("advertised type");

        input
            .options
            .insert("kvCacheTypeK".into(), custom("iq2_xxs"));
        assert!(validate_advanced_options(&input, &capabilities()).is_err());
    }
}
