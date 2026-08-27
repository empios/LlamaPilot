use std::path::Path;

use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::artifacts;
use crate::profiles::{
    build_command_preview, normalize_input, CommandPreview, LaunchProfile, ProfileInput,
    ResolvedProfileTarget,
};
use crate::runtime::{RuntimeRecord, RuntimeRegistry};
use crate::state::AppState;

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> AppResult<Vec<LaunchProfile>> {
    let profiles = state.profiles.clone();
    run_blocking(move || profiles.list()).await
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    input: ProfileInput,
) -> AppResult<LaunchProfile> {
    let profiles = state.profiles.clone();
    let models = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let runtimes = state.runtimes.get();
    run_blocking(move || {
        let prepared = prepare_profile(input, None, &runtimes, &models.scan(&roots)?)?;
        build_command_preview(
            prepared.input.clone(),
            &prepared.runtime,
            &prepared.capabilities,
            &prepared.target,
        )?;
        profiles.create(prepared.input, prepared.target)
    })
    .await
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    id: String,
    input: ProfileInput,
) -> AppResult<LaunchProfile> {
    if state.performance_sweeps.is_running() {
        return Err(crate::performance::sweep_in_progress_error());
    }
    update_profile_record(&state, id, input).await
}

pub(crate) async fn update_profile_record(
    state: &AppState,
    id: String,
    input: ProfileInput,
) -> AppResult<LaunchProfile> {
    let profiles = state.profiles.clone();
    let models = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let runtimes = state.runtimes.get();
    run_blocking(move || {
        let existing = profiles.find(&id)?;
        let prepared = prepare_profile(input, Some(&existing), &runtimes, &models.scan(&roots)?)?;
        build_command_preview(
            prepared.input.clone(),
            &prepared.runtime,
            &prepared.capabilities,
            &prepared.target,
        )?;
        profiles.update(&id, prepared.input, prepared.target)
    })
    .await
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, id: String) -> AppResult<()> {
    if state.performance_sweeps.is_running() {
        return Err(crate::performance::sweep_in_progress_error());
    }
    let snapshot = state.server.snapshot().await;
    if snapshot.state.is_active() && snapshot.profile_id.as_deref() == Some(&id) {
        return Err(AppError::new(
            ErrorCode::ServerAlreadyRunning,
            "The active server profile cannot be deleted.",
        )
        .with_hint("Stop llama-server, then delete the profile."));
    }
    let profiles = state.profiles.clone();
    run_blocking(move || profiles.delete(&id)).await
}

#[tauri::command]
pub async fn preview_profile_command(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    input: ProfileInput,
) -> AppResult<CommandPreview> {
    let profiles = state.profiles.clone();
    let models = state.models.clone();
    let roots = state.settings.get().workspace.model_directories;
    let runtimes = state.runtimes.get();
    run_blocking(move || {
        let existing = profile_id
            .as_deref()
            .map(|id| profiles.find(id))
            .transpose()?;
        let prepared = prepare_profile(input, existing.as_ref(), &runtimes, &models.scan(&roots)?)?;
        build_command_preview(
            prepared.input,
            &prepared.runtime,
            &prepared.capabilities,
            &prepared.target,
        )
    })
    .await
}

pub(crate) struct PreparedProfile {
    pub input: ProfileInput,
    pub runtime: RuntimeRecord,
    pub capabilities: crate::llama::capabilities::LlamaCapabilities,
    pub target: ResolvedProfileTarget,
}

pub(crate) fn prepare_profile(
    input: ProfileInput,
    existing: Option<&LaunchProfile>,
    runtimes: &RuntimeRegistry,
    catalog: &crate::models::ModelCatalog,
) -> AppResult<PreparedProfile> {
    let input = normalize_input(input)?;
    let runtime = runtimes.find(&input.runtime_id).cloned().ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeNotFound,
            "The runtime selected by this profile no longer exists.",
        )
        .with_details(input.runtime_id.clone())
    })?;
    if !runtime.is_available() {
        return Err(AppError::new(
            ErrorCode::RuntimeNotFound,
            "The selected runtime executable is missing.",
        )
        .with_details(runtime.executable.display().to_string()));
    }
    let summary = runtime.capabilities.as_ref().ok_or_else(|| {
        AppError::new(
            ErrorCode::CapabilityDiscoveryFailed,
            "The selected runtime has not been inspected.",
        )
        .with_hint("Inspect it on the Build page before creating a profile.")
    })?;
    let capabilities = artifacts::load(&runtime.directory, &summary.inspection_id)?.capabilities;

    if catalog
        .models
        .iter()
        .find(|model| model.id == input.model_id)
        .is_some_and(|model| model.is_drafter())
    {
        return Err(AppError::new(
            ErrorCode::InvalidProfile,
            "A drafter cannot be used as the primary model.",
        )
        .with_hint(
            "Choose the full model on the Profile tab and attach this file on the Speculative tab.",
        ));
    }

    let target =
        if let Some(existing) = existing.filter(|profile| profile.model_id == input.model_id) {
            require_file(&existing.model_path, "The profile's pinned model")?;
            if let Some(projector) = &existing.projector_path {
                require_file(projector, "The profile's pinned projector")?;
            }
            ResolvedProfileTarget {
                runtime_label: runtime.label(),
                model_name: existing.model_name.clone(),
                model_path: existing.model_path.clone(),
                projector_path: existing.projector_path.clone(),
            }
        } else {
            let model = catalog
                .models
                .iter()
                .find(|model| model.id == input.model_id)
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidProfile,
                        "The selected model is no longer in the model catalog.",
                    )
                    .with_details(input.model_id.clone())
                })?;
            if !model.complete {
                return Err(AppError::new(
                    ErrorCode::InvalidProfile,
                    "The selected model is missing one or more shards.",
                )
                .with_hint("Restore every shard before creating a launch profile."));
            }
            let model_path = model.primary_path.clone().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidProfile,
                    "The selected model has no first shard.",
                )
            })?;
            let projector_path = model.projector_id.as_deref().and_then(|projector_id| {
                catalog
                    .projectors
                    .iter()
                    .find(|projector| projector.id == projector_id)
                    .map(|projector| projector.path.clone())
            });
            ResolvedProfileTarget {
                runtime_label: runtime.label(),
                model_name: model.display_name.clone(),
                model_path,
                projector_path,
            }
        };

    Ok(PreparedProfile {
        input,
        runtime,
        capabilities,
        target,
    })
}

fn require_file(path: &Path, label: &str) -> AppResult<()> {
    if !path.is_file() {
        return Err(
            AppError::new(ErrorCode::InvalidProfile, format!("{label} is missing."))
                .with_details(path.display().to_string()),
        );
    }
    Ok(())
}

async fn run_blocking<T, F>(operation: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| {
            AppError::internal("The profile operation stopped unexpectedly.")
                .with_details(error.to_string())
        })?
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Write;

    use crate::build::profile::{BuildBackend, BuildConfiguration};
    use crate::llama::{artifacts, parser};
    use crate::models::ModelCatalogService;
    use crate::profiles::ProfileOptionSetting;

    use super::*;

    fn write_string(buffer: &mut Vec<u8>, value: &str) {
        buffer.extend_from_slice(&(value.len() as u64).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
    }

    fn write_model(path: &Path) {
        let entries = [
            ("general.type", "model"),
            ("general.name", "Tiny Test Model"),
            ("general.architecture", "llama"),
        ];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i64.to_le_bytes());
        bytes.extend_from_slice(&(entries.len() as i64).to_le_bytes());
        for (key, value) in entries {
            write_string(&mut bytes, key);
            bytes.extend_from_slice(&8_u32.to_le_bytes());
            write_string(&mut bytes, value);
        }
        let mut file = fs::File::create(path).expect("model fixture");
        file.write_all(&bytes).expect("write model fixture");
    }

    #[test]
    fn resolves_real_catalog_and_capability_sidecars_into_a_preview() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let model_root = temp.path().join("models");
        let runtime_root = temp.path().join("runtime");
        fs::create_dir_all(&model_root).expect("model root");
        fs::create_dir_all(&runtime_root).expect("runtime root");
        write_model(&model_root.join("tiny.gguf"));

        let model_service = ModelCatalogService::load(
            temp.path().join("cache.json"),
            temp.path().join("model-overrides.json"),
        )
        .expect("model service");
        let catalog = model_service
            .scan(std::slice::from_ref(&model_root))
            .expect("model scan");

        let executable = runtime_root.join(if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        });
        fs::write(&executable, b"fixture").expect("runtime executable");
        let inspection = parser::parse_capabilities(
            "version: b9000-deadbeef (build 1, commit deadbeef)",
            "--model FNAME  model file\n--host HOST  listen host\n--port PORT  listen port\n-c, --ctx-size N|auto  context size\n",
            "Available devices:\n  (none)\n",
        )
        .expect("capabilities");
        let inspection = crate::llama::capabilities::RuntimeInspection {
            capabilities: inspection,
            raw: crate::llama::capabilities::LlamaRawOutputs {
                version: crate::llama::capabilities::RawCommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                },
                help: crate::llama::capabilities::RawCommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                },
                devices: crate::llama::capabilities::RawCommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                },
            },
        };
        let summary = artifacts::persist(&runtime_root, &inspection).expect("sidecar");
        let runtime = RuntimeRecord {
            id: "runtime-1".into(),
            source_id: "source-1".into(),
            source_name: "llama.cpp".into(),
            repository: "https://example.invalid/llama.cpp".into(),
            commit: "deadbeef".into(),
            short_commit: "deadbee".into(),
            branch: "master".into(),
            backend: BuildBackend::Cpu,
            configuration: BuildConfiguration::Release,
            generator: "Ninja".into(),
            build_date: "2026-08-25T12:00:00Z".into(),
            directory: runtime_root,
            executable,
            size_bytes: 7,
            file_count: 1,
            capabilities: Some(summary),
        };
        let mut runtimes = RuntimeRegistry::default();
        runtimes.insert(runtime);
        let input = ProfileInput {
            name: "End to end".into(),
            description: None,
            runtime_id: "runtime-1".into(),
            model_id: catalog.models[0].id.clone(),
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: false,
            options: BTreeMap::from([("contextSize".into(), ProfileOptionSetting::Auto)]),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
        };

        let prepared = prepare_profile(input, None, &runtimes, &catalog).expect("prepared");
        let preview = build_command_preview(
            prepared.input,
            &prepared.runtime,
            &prepared.capabilities,
            &prepared.target,
        )
        .expect("preview");
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| { pair[0] == "--model" && Path::new(&pair[1]).ends_with("tiny.gguf") }));
        assert!(preview
            .arguments
            .windows(2)
            .any(|pair| pair == ["--ctx-size", "auto"]));
    }
}
