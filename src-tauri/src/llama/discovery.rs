use std::path::Path;
use std::time::Duration;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::platform::ProcessGroup;
use crate::process::{self, CommandOutput, CommandSpec};

use super::capabilities::{LlamaRawOutputs, RawCommandOutput, RuntimeInspection};
use super::parser;

const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(20);

/// Interrogates one exact llama-server executable. Parsing stays in `parser`; this module only
/// owns process execution, timeouts, and preserving both output streams.
pub async fn inspect(
    executable: &Path,
    process_group: Option<&ProcessGroup>,
) -> AppResult<RuntimeInspection> {
    if !executable.is_file() {
        return Err(
            AppError::invalid_path("The llama-server executable no longer exists.")
                .with_details(executable.display().to_string()),
        );
    }

    let version = invoke(executable, "--version", process_group).await?;
    let help = invoke(executable, "--help", process_group).await?;
    let devices = invoke(executable, "--list-devices", process_group).await?;

    let raw = LlamaRawOutputs {
        version: into_raw(version),
        help: into_raw(help),
        devices: into_raw(devices),
    };
    let capabilities = parser::parse_capabilities(
        &raw.version.combined(),
        &raw.help.combined(),
        &raw.devices.combined(),
    )?;

    Ok(RuntimeInspection { capabilities, raw })
}

async fn invoke(
    executable: &Path,
    argument: &str,
    process_group: Option<&ProcessGroup>,
) -> AppResult<CommandOutput> {
    let mut spec = CommandSpec::new(executable).arg(argument);
    if let Some(directory) = executable.parent() {
        spec = spec.current_dir(directory);
    }

    let run = async {
        match process_group {
            Some(group) => process::capture_in_group(&spec, group).await,
            None => process::capture(&spec).await,
        }
    };
    let output = tokio::time::timeout(DISCOVERY_TIMEOUT, run)
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::CapabilityDiscoveryFailed,
                format!("llama-server {argument} did not finish within 20 seconds."),
            )
            .with_hint("The binary may be incompatible or blocked by security software.")
            .with_details(spec.to_display_string())
        })??;

    if process_group.is_some_and(ProcessGroup::is_terminated) {
        return Err(
            AppError::new(ErrorCode::BuildCancelled, "The build was cancelled.")
                .with_details(output.diagnostics()),
        );
    }

    if output.succeeded() {
        return Ok(output);
    }

    Err(AppError::new(
        ErrorCode::CapabilityDiscoveryFailed,
        format!(
            "llama-server {argument} exited with code {}.",
            output
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
    )
    .with_hint("The raw output below comes directly from the selected runtime.")
    .with_details(output.diagnostics()))
}

fn into_raw(output: CommandOutput) -> RawCommandOutput {
    RawCommandOutput {
        stdout: output.stdout,
        stderr: output.stderr,
    }
}
