use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::capabilities::{LlamaCapabilities, LlamaOption};
use crate::process::CommandSpec;
use crate::runtime::RuntimeRecord;

use super::record::{normalize_input, ProfileInput, ProfileOptionSetting};
use super::validation::validate_advanced_options;

const MODEL_FLAGS: &[&str] = &["-m", "--model"];
const HOST_FLAGS: &[&str] = &["--host"];
const PORT_FLAGS: &[&str] = &["--port"];
const MMPROJ_FLAGS: &[&str] = &["-mm", "--mmproj"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProfileTarget {
    pub runtime_label: String,
    pub model_name: String,
    pub model_path: PathBuf,
    pub projector_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreview {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub plain: String,
    pub powershell: String,
    pub posix: String,
    pub runtime_label: String,
    pub model_name: String,
    pub capability_version: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct GeneratedProfileCommand {
    pub spec: CommandSpec,
    pub preview: CommandPreview,
}

/// Builds the exact argument array later consumed by the process supervisor. Rendered strings are
/// derived from this array and are never used for execution.
pub fn build_command_preview(
    input: ProfileInput,
    runtime: &RuntimeRecord,
    capabilities: &LlamaCapabilities,
    target: &ResolvedProfileTarget,
) -> AppResult<CommandPreview> {
    Ok(build_profile_command(input, runtime, capabilities, target)?.preview)
}

pub(crate) fn build_profile_command(
    input: ProfileInput,
    runtime: &RuntimeRecord,
    capabilities: &LlamaCapabilities,
    target: &ResolvedProfileTarget,
) -> AppResult<GeneratedProfileCommand> {
    let input = normalize_input(input)?;
    if input.runtime_id != runtime.id {
        return Err(invalid_profile(
            "The selected runtime does not match this profile.",
        ));
    }

    let mut warnings = validate_advanced_options(&input, capabilities)?;

    let mut command = CommandSpec::new(&runtime.executable);
    command = command
        .arg(required_flag(capabilities, MODEL_FLAGS, "model")?)
        .arg(&target.model_path)
        .arg(required_flag(capabilities, HOST_FLAGS, "host")?)
        .arg(&input.host)
        .arg(required_flag(capabilities, PORT_FLAGS, "port")?)
        .arg(input.port.to_string());

    if let Some(projector) = &target.projector_path {
        command = command
            .arg(required_flag(
                capabilities,
                MMPROJ_FLAGS,
                "multimodal projector",
            )?)
            .arg(projector);
    }

    for (key, setting) in &input.options {
        let option = resolve_option(capabilities, key).ok_or_else(|| {
            invalid_profile(format!(
                "Option {key:?} is not supported by the selected runtime."
            ))
            .with_hint("Return it to Default or choose a runtime that advertises this option.")
        })?;
        if is_core_option(option) {
            return Err(invalid_profile(format!(
                "Option {} is managed by the profile identity and cannot be overridden here.",
                option.flag
            )));
        }
        command = emit_setting(command, option, setting)?;
    }

    command = command.args(input.additional_arguments.iter());
    for (key, value) in &input.environment {
        command = command.env(key, value);
    }

    let program = command.program_display();
    let arguments = command.args_display();
    let environment = input.environment;
    let plain = command.to_display_string();
    let powershell = powershell_preview(&program, &arguments, &environment);
    let posix = posix_preview(&program, &arguments, &environment);
    if input.auto_select_port {
        warnings.push(format!(
            "Port {} is the preferred port; the process supervisor may choose another free port when starting.",
            input.port
        ));
    }

    let preview = CommandPreview {
        program,
        arguments,
        environment,
        plain,
        powershell,
        posix,
        runtime_label: target.runtime_label.clone(),
        model_name: target.model_name.clone(),
        capability_version: capabilities.version.clone(),
        warnings,
    };

    Ok(GeneratedProfileCommand {
        spec: command,
        preview,
    })
}

fn emit_setting(
    mut command: CommandSpec,
    option: &LlamaOption,
    setting: &ProfileOptionSetting,
) -> AppResult<CommandSpec> {
    match setting {
        ProfileOptionSetting::Default => Ok(command),
        ProfileOptionSetting::Auto => {
            if !supports_auto(option) {
                return Err(invalid_profile(format!(
                    "{} does not advertise an auto value in this runtime.",
                    option.display_name
                ))
                .with_details(option.description.clone()));
            }
            Ok(command.arg(&option.flag).arg("auto"))
        }
        ProfileOptionSetting::Custom { value } if option.value_hint.is_some() => {
            if value.is_empty() {
                return Err(invalid_profile(format!(
                    "{} requires a custom value.",
                    option.display_name
                )));
            }
            Ok(command.arg(&option.flag).arg(value))
        }
        ProfileOptionSetting::Custom { value } => {
            let enabled = parse_boolean(value).ok_or_else(|| {
                invalid_profile(format!(
                    "{} is a switch and expects true or false.",
                    option.display_name
                ))
            })?;
            if enabled {
                let positive = option
                    .aliases
                    .iter()
                    .find(|alias| !alias.starts_with("--no-"))
                    .unwrap_or(&option.flag);
                command = command.arg(positive);
            } else {
                let negative = option
                    .aliases
                    .iter()
                    .find(|alias| alias.starts_with("--no-"))
                    .ok_or_else(|| {
                        invalid_profile(format!(
                            "{} has no explicit disabled flag in this runtime; use Default instead.",
                            option.display_name
                        ))
                    })?;
                command = command.arg(negative);
            }
            Ok(command)
        }
    }
}

fn resolve_option<'a>(capabilities: &'a LlamaCapabilities, key: &str) -> Option<&'a LlamaOption> {
    capabilities.options.values().find(|option| {
        option.known_key.as_deref() == Some(key)
            || option.flag == key
            || option.aliases.iter().any(|alias| alias == key)
    })
}

fn required_flag(
    capabilities: &LlamaCapabilities,
    aliases: &[&str],
    concept: &str,
) -> AppResult<String> {
    find_by_aliases(capabilities, aliases)
        .map(|option| option.flag.clone())
        .ok_or_else(|| {
            invalid_profile(format!(
                "The selected runtime does not advertise a {concept} option."
            ))
            .with_hint("Inspect the runtime again or select a compatible llama-server build.")
        })
}

fn find_by_aliases<'a>(
    capabilities: &'a LlamaCapabilities,
    aliases: &[&str],
) -> Option<&'a LlamaOption> {
    capabilities.options.values().find(|option| {
        option
            .aliases
            .iter()
            .any(|actual| aliases.iter().any(|expected| actual == expected))
    })
}

fn is_core_option(option: &LlamaOption) -> bool {
    [MODEL_FLAGS, HOST_FLAGS, PORT_FLAGS, MMPROJ_FLAGS]
        .into_iter()
        .flatten()
        .any(|core| option.aliases.iter().any(|alias| alias == core))
}

fn supports_auto(option: &LlamaOption) -> bool {
    option
        .value_hint
        .as_deref()
        .into_iter()
        .flat_map(|value| value.split(|character: char| !character.is_ascii_alphanumeric()))
        .any(|word| word.eq_ignore_ascii_case("auto"))
}

fn parse_boolean(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "on" | "enabled" | "1" => Some(true),
        "false" | "off" | "disabled" | "0" => Some(false),
        _ => None,
    }
}

fn powershell_preview(
    program: &str,
    arguments: &[String],
    environment: &BTreeMap<String, String>,
) -> String {
    let invocation = std::iter::once(program)
        .chain(arguments.iter().map(String::as_str))
        .map(quote_powershell)
        .collect::<Vec<_>>()
        .join(" ");
    let invocation = format!("& {invocation}");
    if environment.is_empty() {
        return invocation;
    }

    let mut lines = vec!["& {".to_string()];
    for (index, key) in environment.keys().enumerate() {
        lines.push(format!(
            "  $__llamaControlPrevious{index} = [Environment]::GetEnvironmentVariable({}, 'Process')",
            quote_powershell(key)
        ));
    }
    lines.push("  try {".to_string());
    for (key, value) in environment {
        lines.push(format!(
            "    [Environment]::SetEnvironmentVariable({}, {}, 'Process')",
            quote_powershell(key),
            quote_powershell(value)
        ));
    }
    lines.push(format!("    {invocation}"));
    lines.push("  } finally {".to_string());
    for (index, key) in environment.keys().enumerate() {
        lines.push(format!(
            "    [Environment]::SetEnvironmentVariable({}, $__llamaControlPrevious{index}, 'Process')",
            quote_powershell(key)
        ));
    }
    lines.push("  }".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}

fn quote_powershell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn invalid_profile(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::InvalidProfile, message)
}

fn posix_preview(
    program: &str,
    arguments: &[String],
    environment: &BTreeMap<String, String>,
) -> String {
    fn quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
    let mut words = vec!["env".to_string()];
    words.extend(
        environment
            .iter()
            .map(|(key, value)| quote(&format!("{key}={value}"))),
    );
    words.push(quote(program));
    words.extend(arguments.iter().map(|arg| quote(arg)));
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::build::profile::{BuildBackend, BuildConfiguration};
    use crate::llama::capabilities::{LlamaOptionCategory, CAPABILITIES_SCHEMA_VERSION};

    use super::*;

    fn option(
        flag: &str,
        aliases: &[&str],
        value_hint: Option<&str>,
        known_key: Option<&str>,
        description: &str,
    ) -> LlamaOption {
        LlamaOption {
            flag: flag.into(),
            aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
            value_hint: value_hint.map(str::to_string),
            description: description.into(),
            section: "common params".into(),
            category: LlamaOptionCategory::Advanced,
            known_key: known_key.map(str::to_string),
            display_name: flag.into(),
            summary: None,
        }
    }

    fn capabilities() -> LlamaCapabilities {
        let options = [
            option("--model", &["-m", "--model"], Some("FNAME"), None, ""),
            option(
                "--alias",
                &["-a", "--alias"],
                Some("STRING"),
                Some("modelAlias"),
                "model names used by the API",
            ),
            option("--host", &["--host"], Some("HOST"), None, ""),
            option("--port", &["--port"], Some("PORT"), None, ""),
            option("--mmproj", &["-mm", "--mmproj"], Some("FNAME"), None, ""),
            option(
                "--ctx-size",
                &["-c", "--ctx-size"],
                Some("N|auto"),
                Some("contextSize"),
                "auto uses model metadata",
            ),
            option(
                "--flash-attn",
                &["--flash-attn", "--no-flash-attn"],
                None,
                Some("flashAttention"),
                "",
            ),
            option(
                "--spec-type",
                &["--spec-type"],
                Some("none,draft-mtp"),
                Some("speculativeType"),
                "",
            ),
            option(
                "--spec-draft-model",
                &["--spec-draft-model", "--model-draft"],
                Some("FNAME"),
                Some("draftModel"),
                "",
            ),
            option("--future", &["--future"], Some("VALUE"), None, ""),
        ]
        .into_iter()
        .map(|option| (option.flag.clone(), option))
        .collect();
        LlamaCapabilities {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            version: "b9000-test".into(),
            commit: Some("deadbeef".into()),
            options,
            speculative_types: vec!["none".into(), "draft-mtp".into()],
            devices: Vec::new(),
        }
    }

    fn runtime() -> RuntimeRecord {
        RuntimeRecord {
            id: "runtime-1".into(),
            source_id: "source-1".into(),
            source_name: "llama.cpp".into(),
            repository: "https://example.invalid/llama.cpp".into(),
            commit: "deadbeef".into(),
            short_commit: "deadbee".into(),
            branch: "master".into(),
            backend: BuildBackend::Cuda,
            configuration: BuildConfiguration::Release,
            generator: "Ninja".into(),
            build_date: "2026-08-25T12:00:00Z".into(),
            directory: PathBuf::from("/runtime"),
            executable: PathBuf::from("/runtime/llama-server.exe"),
            size_bytes: 1,
            file_count: 1,
            capabilities: None,
        }
    }

    fn input() -> ProfileInput {
        ProfileInput {
            name: "Test".into(),
            description: None,
            runtime_id: "runtime-1".into(),
            model_id: "model-1".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: false,
            options: BTreeMap::new(),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
        }
    }

    fn target() -> ResolvedProfileTarget {
        ResolvedProfileTarget {
            runtime_label: "master @ deadbee · CUDA".into(),
            model_name: "Vision Model".into(),
            model_path: PathBuf::from("/models/vision model.gguf"),
            projector_path: Some(PathBuf::from("/models/mmproj vision.gguf")),
        }
    }

    #[test]
    fn emits_only_non_default_supported_options_in_argument_order() {
        let mut input = input();
        input.options.insert(
            "contextSize".into(),
            ProfileOptionSetting::Custom {
                value: "8192".into(),
            },
        );
        input.options.insert(
            "flashAttention".into(),
            ProfileOptionSetting::Custom {
                value: "false".into(),
            },
        );
        input.additional_arguments = vec!["--future".into(), "value with spaces".into()];

        let preview =
            build_command_preview(input, &runtime(), &capabilities(), &target()).expect("preview");
        assert_eq!(
            preview.arguments,
            [
                "--model",
                "/models/vision model.gguf",
                "--host",
                "127.0.0.1",
                "--port",
                "8080",
                "--mmproj",
                "/models/mmproj vision.gguf",
                "--ctx-size",
                "8192",
                "--no-flash-attn",
                "--future",
                "value with spaces",
            ]
        );
    }

    #[test]
    fn emits_api_model_aliases_for_client_requests() {
        let mut input = input();
        input.options.insert(
            "modelAlias".into(),
            ProfileOptionSetting::Custom {
                value: "qwen-coder,local-coder".into(),
            },
        );

        let preview =
            build_command_preview(input, &runtime(), &capabilities(), &target()).expect("preview");
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| pair == ["--alias", "qwen-coder,local-coder"]));
    }

    #[test]
    fn auto_is_emitted_only_when_the_runtime_advertises_it() {
        let mut accepted = input();
        accepted
            .options
            .insert("contextSize".into(), ProfileOptionSetting::Auto);
        let preview = build_command_preview(accepted, &runtime(), &capabilities(), &target())
            .expect("auto supported");
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| pair == ["--ctx-size", "auto"]));

        let mut rejected = input();
        rejected
            .options
            .insert("--future".into(), ProfileOptionSetting::Auto);
        assert_eq!(
            build_command_preview(rejected, &runtime(), &capabilities(), &target())
                .expect_err("auto unsupported")
                .code,
            ErrorCode::InvalidProfile
        );
    }

    #[test]
    fn emits_an_mtp_drafter_as_a_separate_model_argument() {
        let draft = tempfile::NamedTempFile::new().expect("draft fixture");
        let draft_path = draft.path().to_string_lossy().into_owned();
        let mut input = input();
        input.options.insert(
            "speculativeType".into(),
            ProfileOptionSetting::Custom {
                value: "draft-mtp".into(),
            },
        );
        input.options.insert(
            "draftModel".into(),
            ProfileOptionSetting::Custom {
                value: draft_path.clone(),
            },
        );

        let preview =
            build_command_preview(input, &runtime(), &capabilities(), &target()).expect("preview");
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| pair == ["--spec-type", "draft-mtp"]));
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| pair == ["--spec-draft-model", draft_path.as_str()]));
    }

    #[test]
    fn powershell_preview_scopes_and_restores_environment_values() {
        let mut input = input();
        input
            .environment
            .insert("LLAMA_LOG_COLORS".into(), "it's on".into());
        let preview =
            build_command_preview(input, &runtime(), &capabilities(), &target()).expect("preview");
        assert!(preview.powershell.contains("GetEnvironmentVariable"));
        assert!(preview.powershell.contains("'it''s on'"));
        assert!(preview.powershell.contains("finally"));
        assert_eq!(preview.environment["LLAMA_LOG_COLORS"], "it's on");
    }

    #[test]
    fn unknown_or_core_overrides_are_rejected() {
        let mut unknown = input();
        unknown.options.insert(
            "--not-there".into(),
            ProfileOptionSetting::Custom { value: "1".into() },
        );
        assert!(build_command_preview(unknown, &runtime(), &capabilities(), &target()).is_err());

        let mut core = input();
        core.options.insert(
            "--host".into(),
            ProfileOptionSetting::Custom {
                value: "0.0.0.0".into(),
            },
        );
        assert!(build_command_preview(core, &runtime(), &capabilities(), &target()).is_err());
    }
}

#[cfg(all(test, unix))]
mod posix_tests {
    use super::*;
    #[test]
    fn shell_preview_round_trips_metacharacters_without_execution() {
        let value = "space ' quote $HOME $(false) `false` ; *\nnext";
        let preview = posix_preview(
            "/usr/bin/printf",
            &["%s".into(), value.into()],
            &BTreeMap::new(),
        );
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &preview])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), value);
    }
}
