use std::collections::BTreeMap;

use crate::error::{AppError, AppResult, ErrorCode};

use super::capabilities::{
    known_option, LlamaCapabilities, LlamaDevice, LlamaOption, LlamaOptionCategory,
    CAPABILITIES_SCHEMA_VERSION,
};

pub fn parse_capabilities(
    version_output: &str,
    help_output: &str,
    devices_output: &str,
) -> AppResult<LlamaCapabilities> {
    let (version, commit) = parse_version(version_output).ok_or_else(|| {
        AppError::new(
            ErrorCode::CapabilityDiscoveryFailed,
            "llama-server returned no recognisable version.",
        )
        .with_details(version_output)
    })?;

    let options = parse_help(help_output);
    if options.is_empty() {
        return Err(AppError::new(
            ErrorCode::CapabilityDiscoveryFailed,
            "llama-server returned help text without any recognisable options.",
        )
        .with_hint("This binary may not be llama-server, or its help format may be incompatible.")
        .with_details(help_output));
    }

    let speculative_types = options
        .get("--spec-type")
        .and_then(|option| option.value_hint.as_deref())
        .map(parse_comma_values)
        .unwrap_or_default();

    Ok(LlamaCapabilities {
        schema_version: CAPABILITIES_SCHEMA_VERSION,
        version,
        commit,
        options,
        speculative_types,
        devices: parse_devices(devices_output),
    })
}

/// Supports both the current `version: … (build …, commit …)` output and older one-line builds.
pub fn parse_version(output: &str) -> Option<(String, Option<String>)> {
    let lines: Vec<_> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let line = lines
        .iter()
        .copied()
        .find(|line| line.to_ascii_lowercase().starts_with("version:"))
        .or_else(|| lines.first().copied())?;

    let value = line
        .split_once(':')
        .filter(|(label, _)| label.trim().eq_ignore_ascii_case("version"))
        .map_or(line, |(_, value)| value.trim());
    let version = value
        .split_once(" (build")
        .map_or(value, |(version, _)| version)
        .trim()
        .to_string();
    if version.is_empty() {
        return None;
    }

    let commit = extract_commit(line).or_else(|| commit_from_version(&version));
    Some((version, commit))
}

fn extract_commit(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let start = lower.find("commit")? + "commit".len();
    let value = line.get(start..)?.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, ':' | '=' | ',' | '(')
    });
    let commit: String = value
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .collect();
    (!commit.is_empty()).then_some(commit)
}

fn commit_from_version(version: &str) -> Option<String> {
    let candidate = version.rsplit_once('-')?.1;
    (candidate.len() >= 7
        && candidate
            .chars()
            .all(|character| character.is_ascii_hexdigit()))
    .then(|| candidate.to_string())
}

#[derive(Debug)]
struct ParsedOption {
    section: String,
    header: String,
    description: Vec<String>,
}

pub fn parse_help(output: &str) -> BTreeMap<String, LlamaOption> {
    let mut section = "common params".to_string();
    let mut parsed = Vec::<ParsedOption>::new();
    let mut current: Option<ParsedOption> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(name) = parse_section_header(trimmed) {
            if let Some(option) = current.take() {
                parsed.push(option);
            }
            section = name;
            continue;
        }

        if looks_like_option_start(line) {
            if let Some(option) = current.take() {
                parsed.push(option);
            }
            let (header, description) = split_header_description(trimmed);
            current = Some(ParsedOption {
                section: section.clone(),
                header: header.to_string(),
                description: (!description.is_empty())
                    .then(|| description.to_string())
                    .into_iter()
                    .collect(),
            });
        } else if let Some(option) = current.as_mut() {
            option.description.push(trimmed.to_string());
        }
    }

    if let Some(option) = current {
        parsed.push(option);
    }

    parsed
        .into_iter()
        .filter_map(build_option)
        .map(|option| (option.flag.clone(), option))
        .collect()
}

fn parse_section_header(line: &str) -> Option<String> {
    let body = line.strip_prefix("-----")?.strip_suffix("-----")?.trim();
    (!body.is_empty()).then(|| body.to_string())
}

fn looks_like_option_start(line: &str) -> bool {
    let leading = line.len().saturating_sub(line.trim_start().len());
    leading <= 8 && line.trim_start().starts_with('-')
}

fn split_header_description(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let suffix = line[index..].trim_start();
        if index - start >= 2 && !suffix.starts_with('-') && line[..start].contains('-') {
            return (line[..start].trim_end(), suffix);
        }
    }
    (line.trim(), "")
}

fn build_option(parsed: ParsedOption) -> Option<LlamaOption> {
    let mut aliases = Vec::new();
    let mut value_parts = Vec::new();
    let mut reached_value = false;

    for token in parsed.header.split_whitespace() {
        let token = token.trim_end_matches(',');
        if !reached_value && token.starts_with('-') {
            aliases.push(token.to_string());
        } else {
            reached_value = true;
            value_parts.push(token);
        }
    }
    if aliases.is_empty() {
        return None;
    }

    let flag = aliases
        .iter()
        .find(|alias| alias.starts_with("--") && !alias.starts_with("--no-"))
        .or_else(|| aliases.iter().find(|alias| alias.starts_with("--")))
        .unwrap_or(&aliases[0])
        .clone();
    let known = known_option(&aliases);
    let display_name = known
        .map(|definition| definition.label.to_string())
        .unwrap_or_else(|| humanize_flag(&flag));

    Some(LlamaOption {
        flag,
        aliases,
        value_hint: (!value_parts.is_empty()).then(|| value_parts.join(" ")),
        description: parsed.description.join("\n"),
        section: parsed.section,
        category: known
            .map(|definition| definition.category)
            .unwrap_or(LlamaOptionCategory::Advanced),
        known_key: known.map(|definition| definition.key.to_string()),
        display_name,
        summary: known.map(|definition| definition.summary.to_string()),
    })
}

fn humanize_flag(flag: &str) -> String {
    let mut words = flag.trim_start_matches('-').split('-');
    let Some(first) = words.next() else {
        return flag.to_string();
    };
    let mut result = capitalize(first);
    for word in words {
        result.push(' ');
        result.push_str(word);
    }
    result
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(characters).collect()
}

fn parse_comma_values(value_hint: &str) -> Vec<String> {
    value_hint
        .trim_matches(|character| matches!(character, '<' | '>' | '[' | ']'))
        .split(',')
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
        .map(str::to_string)
        .collect()
}

pub fn parse_devices(output: &str) -> Vec<LlamaDevice> {
    let mut after_header = false;
    let mut devices = Vec::new();

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.eq_ignore_ascii_case("available devices:") {
            after_header = true;
            continue;
        }
        // Upstream deliberately indents device rows by two spaces. Requiring that indentation
        // prevents backend diagnostics from stderr (which is retained after stdout) from being
        // mistaken for a device merely because they also contain a colon.
        let leading = raw_line.len().saturating_sub(raw_line.trim_start().len());
        if !after_header || leading < 2 || line.is_empty() || line.eq_ignore_ascii_case("(none)") {
            continue;
        }

        let Some((id, remainder)) = line.split_once(':') else {
            continue;
        };
        let id = id.trim();
        if id.is_empty() || id.chars().any(char::is_whitespace) {
            continue;
        }

        let (name, memory_total_mib, memory_free_mib) = parse_device_description(remainder.trim());
        devices.push(LlamaDevice {
            id: id.to_string(),
            name,
            backend: device_backend(id),
            memory_total_mib,
            memory_free_mib,
            raw: line.to_string(),
        });
    }

    devices
}

fn parse_device_description(value: &str) -> (String, Option<u64>, Option<u64>) {
    let Some(open) = value.rfind(" (") else {
        return (value.to_string(), None, None);
    };
    let Some(memory) = value
        .get(open + 2..)
        .and_then(|tail| tail.strip_suffix(')'))
    else {
        return (value.to_string(), None, None);
    };
    if !memory.contains("MiB") {
        return (value.to_string(), None, None);
    }

    let mut parts = memory.split(',');
    let total = parts.next().and_then(parse_mib);
    let free = parts.next().and_then(parse_mib);
    (value[..open].trim().to_string(), total, free)
}

fn parse_mib(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|number| number.parse().ok())
}

fn device_backend(id: &str) -> Option<String> {
    let prefix: String = id
        .chars()
        .take_while(|character| character.is_ascii_alphabetic() || *character == '_')
        .collect();
    (!prefix.is_empty()).then_some(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_VERSION: &str = include_str!("../../tests/fixtures/llama/version-current.txt");
    const CURRENT_HELP: &str = include_str!("../../tests/fixtures/llama/help-current.txt");
    const CUDA_DEVICES: &str = include_str!("../../tests/fixtures/llama/devices-cuda.txt");

    #[test]
    fn parses_current_version_and_commit() {
        assert_eq!(
            parse_version(CURRENT_VERSION),
            Some(("b7900-790b5713".into(), Some("790b5713".into())))
        );
    }

    #[test]
    fn falls_back_to_a_legacy_version_line() {
        assert_eq!(
            parse_version("llama.cpp b4021-a1b2c3d4\n"),
            Some(("llama.cpp b4021-a1b2c3d4".into(), Some("a1b2c3d4".into())))
        );
    }

    #[test]
    fn parses_aliases_wrapped_descriptions_and_unknown_options() {
        let options = parse_help(CURRENT_HELP);
        let context = options.get("--ctx-size").expect("context option");
        assert_eq!(context.aliases, ["-c", "--ctx-size"]);
        assert_eq!(context.value_hint.as_deref(), Some("N"));
        assert_eq!(context.known_key.as_deref(), Some("contextSize"));

        let device = options.get("--device").expect("device option");
        assert!(device.description.contains("use --list-devices"));

        let unknown = options.get("--future-flag").expect("unknown option");
        assert_eq!(unknown.category, LlamaOptionCategory::Advanced);
        assert!(unknown.known_key.is_none());
    }

    #[test]
    fn reads_speculative_types_from_the_runtime_value_hint() {
        let capabilities =
            parse_capabilities(CURRENT_VERSION, CURRENT_HELP, CUDA_DEVICES).expect("parses");
        assert_eq!(
            capabilities.speculative_types,
            ["none", "draft-simple", "draft-eagle3", "ngram-cache"]
        );
    }

    #[test]
    fn parses_device_identity_backend_and_memory() {
        let devices = parse_devices(CUDA_DEVICES);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "CUDA0");
        assert_eq!(devices[0].backend.as_deref(), Some("CUDA"));
        assert_eq!(devices[0].memory_total_mib, Some(24_564));
        assert_eq!(devices[0].memory_free_mib, Some(22_104));
        assert_eq!(devices[1].name, "NVIDIA GeForce RTX 4060 Ti");
    }

    #[test]
    fn an_explicit_none_device_list_is_empty() {
        assert!(parse_devices("Available devices:\n  (none)\n").is_empty());
    }

    #[test]
    fn backend_diagnostics_after_stdout_are_not_devices() {
        let devices = parse_devices(
            "Available devices:\n  CUDA0: RTX (8192 MiB, 4096 MiB free)\nggml_cuda_init: diagnostic\n",
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "CUDA0");
    }

    #[test]
    fn rejects_help_without_flags() {
        let error = parse_capabilities(CURRENT_VERSION, "usage unavailable", CUDA_DEVICES)
            .expect_err("must reject");
        assert_eq!(error.code, ErrorCode::CapabilityDiscoveryFailed);
    }
}
