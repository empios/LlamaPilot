use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::llama::{artifacts as capability_artifacts, discovery as capability_discovery};
use crate::platform::ProcessGroup;
use crate::process::{self, CommandSpec, OutputLine};
use crate::runtime::record::RuntimeRecord;
use crate::runtime::snapshot;
use crate::sources::service as sources;
use crate::state::AppState;

use super::directory::{build_directory, cached_generator, is_configured};
use super::plan::{artifact_directories, plan_build};
use super::profile::BuildProfile;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRequest {
    pub source_id: String,
    pub profile: BuildProfile,
    /// Deletes the build tree before configuring. Slow, but resolves a corrupt cache.
    #[serde(default)]
    pub clean: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutcome {
    pub runtime: RuntimeRecord,
    pub build_directory: PathBuf,
    pub reconfigured: bool,
}

/// Tracks the one build allowed at a time, so it can be cancelled.
#[derive(Default)]
pub struct BuildSupervisor {
    active: std::sync::Mutex<Option<ActiveBuild>>,
}

struct ActiveBuild {
    id: uuid::Uuid,
    group: Arc<ProcessGroup>,
}

struct BuildPermit<'a> {
    supervisor: &'a BuildSupervisor,
    id: uuid::Uuid,
    group: Arc<ProcessGroup>,
}

impl BuildPermit<'_> {
    fn group(&self) -> &ProcessGroup {
        &self.group
    }

    fn ensure_not_cancelled(&self) -> AppResult<()> {
        if self.group.is_terminated() {
            return Err(AppError::new(
                ErrorCode::BuildCancelled,
                "The build was cancelled.",
            ));
        }

        Ok(())
    }
}

impl std::fmt::Debug for BuildPermit<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BuildPermit")
            .field("id", &self.id)
            .finish()
    }
}

impl Drop for BuildPermit<'_> {
    fn drop(&mut self) {
        self.supervisor.finish(self.id);
    }
}

impl BuildSupervisor {
    fn begin(&self) -> AppResult<BuildPermit<'_>> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err(
                AppError::new(ErrorCode::BuildInProgress, "A build is already running.")
                    .with_hint("Wait for it to finish, or cancel it first."),
            );
        }

        let id = uuid::Uuid::new_v4();
        let group = Arc::new(ProcessGroup::new()?);
        *slot = Some(ActiveBuild {
            id,
            group: Arc::clone(&group),
        });

        Ok(BuildPermit {
            supervisor: self,
            id,
            group,
        })
    }

    fn finish(&self, id: uuid::Uuid) {
        let mut slot = self.lock();
        if slot.as_ref().is_some_and(|active| active.id == id) {
            *slot = None;
        }
    }

    /// Terminates the running build and everything it spawned.
    pub fn cancel(&self) -> bool {
        let group = self.lock().as_ref().map(|active| Arc::clone(&active.group));
        let Some(group) = group else {
            return false;
        };

        group.terminate();
        true
    }

    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ActiveBuild>> {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn cmake_program(state: &AppState) -> PathBuf {
    state
        .settings
        .get()
        .build
        .cmake_executable
        .unwrap_or_else(|| PathBuf::from("cmake"))
}

fn apply_build_defaults(mut profile: BuildProfile, parallel_jobs: Option<u32>) -> BuildProfile {
    if profile.parallel_jobs.is_none() {
        profile.parallel_jobs = parallel_jobs;
    }
    profile
}

/// Configures and builds llama.cpp, then snapshots the result into an immutable runtime.
///
/// Every failure leaves the previously built runtimes untouched: the snapshot is the last step
/// and writes to a fresh directory, so a broken build can never replace a working one.
pub async fn build(
    state: &AppState,
    request: BuildRequest,
    progress: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<BuildOutcome> {
    let source = sources::require(state, &request.source_id)?;
    if !source.directory.is_dir() {
        return Err(
            AppError::invalid_path("The source folder no longer exists on disk.")
                .with_details(source.directory.display().to_string()),
        );
    }

    // The permit covers the entire operation, including preparation, snapshotting, and registry
    // persistence. Cancelling marks it terminated but does not free the slot until this task has
    // actually unwound, so an old task can never clear or corrupt a newer build.
    let permit = state.builds.begin()?;
    permit.ensure_not_cancelled()?;

    let status = sources::status(state, &request.source_id).await?;
    permit.ensure_not_cancelled()?;
    let commit = status
        .head
        .as_ref()
        .map(|head| head.commit.clone())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::NotARepository,
                "The source has no commit to build.",
            )
        })?;

    let settings = state.settings.get();
    let mut profile = apply_build_defaults(request.profile, settings.build.parallel_jobs);
    let toolchain = super::detect::detect(&settings).await;
    if !toolchain.backends.contains(&profile.backend) {
        return Err(AppError::new(
            ErrorCode::ConfigureFailed,
            "This backend is not supported on this platform.",
        ));
    }
    if !toolchain.can_build_cpu()
        || (profile.backend == super::profile::BuildBackend::Cuda && !toolchain.can_build_cuda())
    {
        return Err(AppError::new(
            ErrorCode::ConfigureFailed,
            "The selected backend is missing required build tools.",
        )
        .with_hint("Open Build and follow the toolchain instructions, then re-detect."));
    }
    if !cfg!(windows)
        && profile.generator.is_none()
        && toolchain
            .tool(super::toolchain::ToolId::Make)
            .is_some_and(|tool| !tool.found)
        && toolchain
            .tool(super::toolchain::ToolId::Ninja)
            .is_some_and(|tool| tool.found)
    {
        profile.generator = Some("Ninja".into());
    }
    let builds_root = settings.builds_directory(&state.paths);
    let directory = build_directory(&builds_root, &source.name, &profile, std::env::consts::ARCH);

    let plan = plan_build(&source.directory, &directory, &profile);
    let configuration = profile.configuration.cmake_name().to_string();

    let runtime_id = uuid::Uuid::new_v4().to_string();
    let snapshot_target = snapshot::snapshot_directory(
        &state.paths.runtimes_dir,
        &commit,
        profile.backend.slug(),
        &runtime_id,
    );

    if request.clean && directory.exists() {
        std::fs::remove_dir_all(&directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not clean {}.", directory.display()),
            )
            .with_details(error.to_string())
        })?;
    }

    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not create {}.", directory.display()),
        )
        .with_details(error.to_string())
    })?;

    // A cache made by a different generator cannot be reused, and CMake's own error for this is
    // opaque, so it is detected up front.
    let generator_changed = match (cached_generator(&directory), profile.generator.as_deref()) {
        (Some(cached), Some(requested)) => cached != requested,
        _ => false,
    };
    let reconfigure = request.clean || generator_changed || !is_configured(&directory);

    run_build(
        state,
        &plan,
        &configuration,
        true,
        &permit,
        progress.clone(),
    )
    .await?;
    permit.ensure_not_cancelled()?;

    let artifacts = snapshot::locate_artifacts(&artifact_directories(&plan, &configuration))?;
    let staged = snapshot::stage_runtime(&artifacts, &snapshot_target)?;
    permit.ensure_not_cancelled()?;

    let _ = progress.send(OutputLine {
        stream: crate::process::OutputStream::Stdout,
        text: "Inspecting llama-server capabilities…".to_string(),
    });
    let inspection =
        capability_discovery::inspect(&staged.executable(), Some(permit.group())).await?;
    permit.ensure_not_cancelled()?;
    let capabilities = capability_artifacts::persist(staged.directory(), &inspection)?;
    permit.ensure_not_cancelled()?;

    let runtime = RuntimeRecord {
        id: runtime_id,
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        repository: source.repository.clone(),
        short_commit: commit.chars().take(7).collect(),
        commit,
        branch: status.source.current_ref.clone(),
        backend: profile.backend,
        configuration: profile.configuration,
        generator: profile
            .generator
            .clone()
            .unwrap_or_else(|| "CMake default".to_string()),
        build_date: chrono::Utc::now().to_rfc3339(),
        directory: snapshot_target.clone(),
        executable: snapshot_target.join(snapshot::SERVER_EXECUTABLE),
        size_bytes: staged.size_bytes(),
        file_count: staged.file_count(),
        capabilities: Some(capabilities),
    };

    staged.commit(&runtime)?;

    if let Err(mut error) = state
        .runtimes
        .update(|registry| registry.insert(runtime.clone()))
    {
        if let Err(cleanup_error) = std::fs::remove_dir_all(&snapshot_target) {
            let cleanup_details = format!(
                "The registry write failed, and the unpublished runtime could not be removed from {}: {}",
                snapshot_target.display(),
                cleanup_error
            );
            error.details = Some(match error.details.take() {
                Some(details) => format!("{details}\n{cleanup_details}"),
                None => cleanup_details,
            });
        }
        return Err(error);
    }

    Ok(BuildOutcome {
        runtime,
        build_directory: directory,
        reconfigured: reconfigure,
    })
}

async fn run_build(
    state: &AppState,
    plan: &super::plan::BuildPlan,
    configuration: &str,
    reconfigure: bool,
    permit: &BuildPermit<'_>,
    progress: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<()> {
    let cmake = cmake_program(state);
    let _ = configuration;

    if reconfigure {
        permit.ensure_not_cancelled()?;
        let spec = CommandSpec::new(&cmake).args(&plan.configure_args);
        announce(&progress, &spec);

        let output = process::stream_in_group(&spec, progress.clone(), permit.group()).await?;
        if !output.succeeded() {
            if permit.group().is_terminated() {
                return Err(
                    AppError::new(ErrorCode::BuildCancelled, "The build was cancelled.")
                        .with_details(output.diagnostics()),
                );
            }
            return Err(classify_cmake_failure(
                ErrorCode::ConfigureFailed,
                "CMake could not configure the build.",
                &output,
            ));
        }
    }

    permit.ensure_not_cancelled()?;
    let spec = CommandSpec::new(&cmake).args(&plan.build_args);
    announce(&progress, &spec);

    let output = process::stream_in_group(&spec, progress, permit.group()).await?;
    if !output.succeeded() {
        if permit.group().is_terminated() {
            return Err(
                AppError::new(ErrorCode::BuildCancelled, "The build was cancelled.")
                    .with_details(output.diagnostics()),
            );
        }
        return Err(classify_cmake_failure(
            ErrorCode::BuildFailed,
            "The build failed to compile.",
            &output,
        ));
    }

    Ok(())
}

fn announce(progress: &mpsc::UnboundedSender<OutputLine>, spec: &CommandSpec) {
    let _ = progress.send(OutputLine {
        stream: crate::process::OutputStream::Stdout,
        text: format!("> {}", spec.to_display_string()),
    });
}

/// Turns raw CMake/compiler output into a specific, actionable message.
///
/// The raw log is always attached; the message only ever adds a diagnosis on top of it.
fn classify_cmake_failure(
    code: ErrorCode,
    fallback: &str,
    output: &process::CommandOutput,
) -> AppError {
    let diagnostics = output.diagnostics();
    let lowered = diagnostics.to_lowercase();

    // A terminated job object surfaces as an abnormal exit rather than a compiler error.
    if output.exit_code.is_none() || output.exit_code == Some(1) && lowered.trim().is_empty() {
        return AppError::new(ErrorCode::BuildCancelled, "The build was cancelled.")
            .with_details(diagnostics);
    }

    let error = if lowered.contains("no cuda toolset found")
        || lowered.contains("cannot find cuda")
        || lowered.contains("could not find cudatoolkit")
    {
        AppError::new(code, "CUDA support was requested but the CUDA toolset was not found.")
            .with_hint(
                "Install the CUDA Toolkit. On Windows include Visual Studio integration; on Linux use a supported host compiler.",
            )
    } else if lowered.contains("no cmake_cuda_compiler could be found")
        || lowered.contains("nvcc") && lowered.contains("not found")
    {
        AppError::new(code, "The CUDA compiler (nvcc) could not be found.").with_hint(
            "Install the CUDA Toolkit, or set CMAKE_CUDA_COMPILER in additional arguments.",
        )
    } else if lowered.contains("no cmake_cxx_compiler could be found")
        || lowered.contains("cl.exe") && lowered.contains("not found")
    {
        AppError::new(code, "No C++ compiler was found.").with_hint(
            "Install a C++ compiler: Apple Command Line Tools on macOS, build-essential on Ubuntu, or Desktop development with C++ in Visual Studio on Windows.",
        )
    } else if lowered.contains("generator") && lowered.contains("does not match") {
        AppError::new(
            code,
            "The existing build directory was made with a different generator.",
        )
        .with_hint("Use Clean build to recreate it.")
    } else if lowered.contains("out of memory")
        || lowered.contains("c1060")
        || lowered.contains("virtual memory")
    {
        AppError::new(code, "The compiler ran out of memory.")
            .with_hint("Lower the parallel job count in Settings and try again.")
    } else if lowered.contains("nvcc warning : cannot find valid gpu") {
        AppError::new(code, "nvcc could not detect a GPU architecture.")
            .with_hint("Set CUDA architectures explicitly, or turn off native optimizations.")
    } else {
        AppError::new(
            code,
            format!(
                "{fallback} CMake exited with code {}.",
                output
                    .exit_code
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ),
        )
        .with_hint("The full output is below; the first error line is usually the real cause.")
    };

    error.with_details(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(exit_code: Option<i32>, stderr: &str) -> process::CommandOutput {
        process::CommandOutput {
            exit_code,
            stdout: String::new(),
            stderr: stderr.to_string(),
            duration_ms: 10,
        }
    }

    #[test]
    fn recognises_a_missing_cuda_toolset() {
        let error = classify_cmake_failure(
            ErrorCode::ConfigureFailed,
            "failed",
            &output(Some(1), "CMake Error: No CUDA toolset found."),
        );

        assert_eq!(error.code, ErrorCode::ConfigureFailed);
        assert!(error.message.contains("CUDA"));
        assert!(error.hint.is_some());
    }

    #[test]
    fn recognises_a_missing_cxx_compiler() {
        let error = classify_cmake_failure(
            ErrorCode::ConfigureFailed,
            "failed",
            &output(Some(1), "No CMAKE_CXX_COMPILER could be found."),
        );

        assert!(error.message.contains("C++ compiler"));
        assert!(error
            .hint
            .expect("hint")
            .contains("Desktop development with C++"));
    }

    #[test]
    fn recognises_compiler_memory_exhaustion() {
        let error = classify_cmake_failure(
            ErrorCode::BuildFailed,
            "failed",
            &output(Some(1), "fatal error C1060: compiler is out of heap space"),
        );

        assert!(error.hint.expect("hint").contains("parallel job count"));
    }

    #[test]
    fn a_terminated_build_reports_cancellation_rather_than_a_compile_error() {
        let error = classify_cmake_failure(ErrorCode::BuildFailed, "failed", &output(None, ""));
        assert_eq!(error.code, ErrorCode::BuildCancelled);
    }

    #[test]
    fn unrecognised_failures_keep_the_raw_log_and_the_requested_code() {
        let error = classify_cmake_failure(
            ErrorCode::BuildFailed,
            "The build failed to compile.",
            &output(Some(2), "some novel compiler diagnostic"),
        );

        assert_eq!(error.code, ErrorCode::BuildFailed);
        assert_eq!(
            error.details.as_deref(),
            Some("some novel compiler diagnostic")
        );
    }

    #[test]
    fn global_parallel_jobs_are_used_when_the_profile_leaves_them_automatic() {
        let profile = apply_build_defaults(BuildProfile::default(), Some(12));
        assert_eq!(profile.parallel_jobs, Some(12));
    }

    #[test]
    fn an_explicit_profile_job_count_overrides_the_global_default() {
        let profile = apply_build_defaults(
            BuildProfile {
                parallel_jobs: Some(4),
                ..BuildProfile::default()
            },
            Some(12),
        );
        assert_eq!(profile.parallel_jobs, Some(4));
    }

    #[test]
    fn only_one_build_may_run_at_a_time() {
        let supervisor = BuildSupervisor::default();

        let _first = supervisor.begin().expect("first build starts");
        assert!(supervisor.is_running());

        let second = supervisor.begin().expect_err("second build is rejected");
        assert_eq!(second.code, ErrorCode::BuildInProgress);
    }

    #[test]
    fn cancelling_keeps_the_slot_until_the_active_task_finishes() {
        let supervisor = BuildSupervisor::default();

        let running = supervisor.begin().expect("starts");
        assert!(supervisor.cancel());
        assert!(supervisor.is_running());
        assert_eq!(
            running
                .ensure_not_cancelled()
                .expect_err("permit observes cancellation")
                .code,
            ErrorCode::BuildCancelled
        );
        assert_eq!(
            supervisor
                .begin()
                .expect_err("new build remains blocked")
                .code,
            ErrorCode::BuildInProgress
        );

        drop(running);
        assert!(!supervisor.is_running());
        assert!(supervisor.begin().is_ok());
    }

    #[test]
    fn cancelling_when_idle_reports_that_nothing_was_running() {
        let supervisor = BuildSupervisor::default();
        assert!(!supervisor.cancel());
    }
}
