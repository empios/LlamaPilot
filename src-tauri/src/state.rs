use std::sync::Arc;
use tauri::{AppHandle, Runtime};

use crate::build::BuildSupervisor;
use crate::config::store::read_json;
use crate::config::{AppPaths, JsonStore, Settings};
use crate::error::AppResult;
use crate::git::Git;
use crate::llama::artifacts as capability_artifacts;
use crate::logging::LoggingHandle;
use crate::models::{ModelCatalogService, ModelDownloadSupervisor};
use crate::performance::{PerformanceHistory, PerformanceSweepSupervisor};
use crate::profiles::ProfileRepository;
use crate::runtime::{snapshot, RuntimeRegistry};
use crate::server::ServerSupervisor;
use crate::sources::SourceRegistry;

/// Everything the command layer needs, resolved once at startup.
pub struct AppState {
    pub work: crate::updater::gate::WorkGate,
    pub updates: crate::updater::UpdateService,
    pub paths: AppPaths,
    pub settings: JsonStore<Settings>,
    pub sources: JsonStore<SourceRegistry>,
    pub runtimes: JsonStore<RuntimeRegistry>,
    pub builds: BuildSupervisor,
    pub models: Arc<ModelCatalogService>,
    pub model_downloads: ModelDownloadSupervisor,
    pub performance: JsonStore<PerformanceHistory>,
    pub performance_sweeps: PerformanceSweepSupervisor,
    pub profiles: Arc<ProfileRepository>,
    pub server: Arc<ServerSupervisor>,
    _logging: Option<LoggingHandle>,
}

impl AppState {
    pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> AppResult<Self> {
        let paths = AppPaths::resolve(app)?;
        paths.ensure_directories()?;

        let logging = crate::logging::install(&paths.logs_dir);

        match snapshot::cleanup_staging_directories(&paths.runtimes_dir) {
            Ok(count) if count > 0 => {
                tracing::info!(count, "removed stale runtime staging directories");
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(%error, "could not clean stale runtime staging directories");
            }
        }

        let settings = JsonStore::load(paths.settings_file.clone())?;
        let sources = JsonStore::load(paths.sources_metadata_file.clone())?;
        let models = Arc::new(ModelCatalogService::load(
            paths.model_metadata_cache_file.clone(),
            paths.models_metadata_file.clone(),
        )?);
        let profiles = Arc::new(ProfileRepository::new(paths.profiles_dir.clone()));
        let performance = JsonStore::load(paths.performance_history_file.clone())?;
        let server = ServerSupervisor::new(paths.logs_dir.join("servers"))?;
        // Successful builds are the runtime history. Keep the registry under builds/ as the
        // documented phase-3 history, while every snapshot also carries its own metadata.json.
        let runtimes: JsonStore<RuntimeRegistry> =
            JsonStore::load(paths.builds_metadata_file.clone())?;

        // One-time compatibility with early phase-3 builds that stored the same registry at
        // runtimes/metadata.json. The old file remains as a recovery backup.
        if runtimes.get().runtimes.is_empty() && paths.runtimes_metadata_file.is_file() {
            let legacy: RuntimeRegistry = read_json(&paths.runtimes_metadata_file)?;
            if !legacy.runtimes.is_empty() {
                let migrated = legacy.runtimes.len();
                runtimes.replace(legacy)?;
                tracing::info!(migrated, "migrated legacy runtime history");
            }
        }

        let snapshots = snapshot::discover_runtime_records(&paths.runtimes_dir);
        let removed_inspections: usize = snapshots
            .iter()
            .map(|runtime| {
                capability_artifacts::cleanup_superseded(
                    &runtime.directory,
                    runtime
                        .capabilities
                        .as_ref()
                        .map(|summary| summary.inspection_id.as_str()),
                )
            })
            .sum();
        if removed_inspections > 0 {
            tracing::info!(
                removed_inspections,
                "removed superseded capability inspections"
            );
        }

        let registered = runtimes.get();
        let discovered: Vec<_> = snapshots
            .into_iter()
            .filter(|runtime| registered.find(&runtime.id) != Some(runtime))
            .collect();
        if !discovered.is_empty() {
            let (_, recovered) =
                runtimes.update(|registry| registry.upsert_discovered(discovered))?;
            tracing::info!(
                recovered,
                "reconciled runtime records from snapshot metadata"
            );
        }

        tracing::info!(data_dir = %paths.data_dir.display(), "application state ready");

        Ok(Self {
            work: Default::default(),
            updates: Default::default(),
            paths,
            settings,
            sources,
            runtimes,
            builds: BuildSupervisor::default(),
            models,
            model_downloads: ModelDownloadSupervisor::default(),
            performance,
            performance_sweeps: PerformanceSweepSupervisor::default(),
            profiles,
            server,
            _logging: logging,
        })
    }

    /// A Git handle honouring the user's configured executable path.
    pub fn git(&self) -> Git {
        Git::new(self.settings.get().git.executable)
    }
}
