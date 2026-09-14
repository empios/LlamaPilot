pub mod gate;

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

pub const RELEASES_URL: &str = "https://github.com/empios/LlamaPilot/releases/latest";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub phase: String,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub last_checked: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub can_check: bool,
    pub can_install: bool,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub deferred_version: Option<String>,
    pub busy: bool,
    pub releases_url: String,
}

#[derive(Default)]
struct Package {
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
}

pub struct UpdateService {
    status: Mutex<Option<UpdateStatus>>,
    package: tokio::sync::Mutex<Package>,
}

impl Default for UpdateService {
    fn default() -> Self {
        Self {
            status: Mutex::new(None),
            package: tokio::sync::Mutex::new(Package::default()),
        }
    }
}

impl UpdateService {
    pub fn snapshot(&self, app: &AppHandle) -> UpdateStatus {
        let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
        status.get_or_insert_with(|| initial_status(app)).clone()
    }

    fn change(&self, app: &AppHandle, change: impl FnOnce(&mut UpdateStatus)) {
        let status = {
            let mut lock = self.status.lock().unwrap_or_else(|e| e.into_inner());
            let status = lock.get_or_insert_with(|| initial_status(app));
            change(status);
            status.clone()
        };
        let _ = app.emit("app-update-status", status);
    }

    fn failure(&self, app: &AppHandle, error: impl std::fmt::Display) -> AppError {
        let message = format!("Application update failed: {error}");
        self.change(app, |s| {
            s.phase = "error".into();
            s.error = Some(message.clone());
        });
        AppError::internal(message)
    }

    pub async fn check(&self, app: &AppHandle) -> AppResult<()> {
        let mut package = self
            .package
            .try_lock()
            .map_err(|_| AppError::unsupported("An update operation is already in progress."))?;
        if !self.snapshot(app).can_check {
            return Err(AppError::unsupported(
                "Update checking is unavailable in this build. Use the releases page.",
            ));
        }
        // Do not discard a verified, downloaded update on a periodic check.
        if package.bytes.is_some() {
            return Ok(());
        }
        self.change(app, |s| {
            s.phase = "checking".into();
            s.error = None;
        });
        let result = async {
            let updater = app
                .updater_builder()
                .timeout(Duration::from_secs(30))
                .build()?;
            updater.check().await
        }
        .await;
        self.change(app, |s| {
            s.last_checked = Some(chrono::Utc::now().to_rfc3339())
        });
        match result {
            Ok(update) => {
                self.change(app, |s| {
                    s.phase = if update.is_some() {
                        "available"
                    } else {
                        "idle"
                    }
                    .into();
                    s.version = update.as_ref().map(|u| u.version.clone());
                    s.notes = update.as_ref().and_then(|u| u.body.clone());
                    s.downloaded = 0;
                    s.total = None;
                });
                package.update = update;
                Ok(())
            }
            // The plugin reports this for unsuccessful HTTP responses, including a
            // missing feed before the first updater-enabled release is published.
            // This is not a successful check and must never mean "up to date".
            Err(tauri_plugin_updater::Error::ReleaseNotFound) => {
                package.update = None;
                self.change(app, |s| {
                    s.phase = "unavailable".into();
                    s.version = None;
                    s.notes = None;
                    s.downloaded = 0;
                    s.total = None;
                    s.error = None;
                });
                Ok(())
            }
            Err(error) => Err(self.failure(app, error)),
        }
    }

    pub async fn download(&self, app: &AppHandle) -> AppResult<()> {
        let mut package = self
            .package
            .try_lock()
            .map_err(|_| AppError::unsupported("An update operation is already in progress."))?;
        if !self.snapshot(app).can_install {
            return Err(AppError::unsupported(
                "Use the installer on the releases page for this installation.",
            ));
        }
        let mut update = package
            .update
            .clone()
            .ok_or_else(|| AppError::unsupported("Check for an update first."))?;
        update.timeout = Some(Duration::from_secs(15 * 60));
        self.change(app, |s| {
            s.phase = "downloading".into();
            s.error = None;
            s.downloaded = 0;
            s.total = None;
        });
        package.bytes = None;
        let result = update
            .download(
                |count, total| {
                    self.change(app, |s| {
                        s.downloaded += count as u64;
                        s.total = total;
                    });
                },
                || {},
            )
            .await;
        match result {
            Ok(bytes) => {
                package.bytes = Some(bytes);
                self.change(app, |s| s.phase = "ready".into());
                Ok(())
            }
            Err(error) => Err(self.failure(app, error)),
        }
    }

    pub fn defer(&self, app: &AppHandle) {
        self.change(app, |s| s.deferred_version = s.version.clone());
    }

    pub async fn install(
        &self,
        app: &AppHandle,
        state: &AppState,
        automatic: bool,
    ) -> AppResult<()> {
        let package = self
            .package
            .try_lock()
            .map_err(|_| AppError::unsupported("An update operation is already in progress."))?;
        if !self.snapshot(app).can_install {
            return Err(AppError::unsupported(
                "This installation requires a manual update.",
            ));
        }
        let update = package
            .update
            .as_ref()
            .ok_or_else(|| AppError::unsupported("Check for an update first."))?;
        let bytes = package
            .bytes
            .as_ref()
            .ok_or_else(|| AppError::unsupported("Download and verify the update first."))?;
        // Admission also serializes changes to the user's automatic-install preference.
        let _exclusive = state.work.install()?;
        if automatic
            && (!state.settings.get().updates.auto_install
                || self.snapshot(app).deferred_version.as_ref() == Some(&update.version))
        {
            return Err(AppError::unsupported(
                "Automatic installation is disabled or deferred.",
            ));
        }
        // Acquire admission before checking the persistent server. New commands cannot race
        // the snapshot; completed start/restart commands leave an active server behind.
        if persistent_work_active(state).await {
            return Err(AppError::new(
                ErrorCode::UpdateBusy,
                "Stop the server and wait for active work before installing.",
            ));
        }
        self.change(app, |s| {
            s.phase = "installing".into();
            s.error = None;
        });
        // This synchronous call exits on Windows after launching NSIS. On other platforms
        // keep the exclusive lease through restart. The bytes were verified by download().
        if let Err(error) = update.install(bytes) {
            return Err(self.failure(app, error));
        }
        app.restart();
    }
}

pub async fn persistent_work_active(state: &AppState) -> bool {
    state.builds.is_running()
        || state.model_downloads.is_running()
        || state.performance_sweeps.is_running()
        || state.server.snapshot().await.state.is_active()
}

fn initial_status(app: &AppHandle) -> UpdateStatus {
    let configured = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|v| v.get("pubkey"))
        .and_then(|v| v.as_str())
        .is_some_and(|key| !key.trim().is_empty());
    let installation = installation_support(app);
    let can_check = !cfg!(debug_assertions) && configured;
    let reason = if cfg!(debug_assertions) {
        Some("Automatic updates are disabled in development builds.".into())
    } else if !configured {
        Some("This build has no update signing key. Download a release installer.".into())
    } else {
        installation.err()
    };
    UpdateStatus {
        phase: "idle".into(),
        current_version: app.package_info().version.to_string(),
        version: None,
        notes: None,
        last_checked: None,
        downloaded: 0,
        total: None,
        can_check,
        can_install: can_check && reason.is_none(),
        reason,
        error: None,
        deferred_version: None,
        busy: false,
        releases_url: RELEASES_URL.into(),
    }
}

fn installation_support(_app: &AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let marker = exe.parent().map(|p| p.join("llamapilot-install-kind"));
        let kind = tauri::utils::platform::bundle_type();
        let marker = marker.and_then(|p| std::fs::read_to_string(p).ok());
        if windows_install_supported(kind, marker.as_deref()) {
            return Ok(());
        }
        Err("MSI and standalone executables require a manual installer update.".into())
    }
    #[cfg(target_os = "linux")]
    {
        use tauri::Manager;

        let path = _app
            .env()
            .appimage
            .ok_or("DEB and unpackaged builds require a manual package update.")?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|_| "The AppImage is not writable. Update it manually.".to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if exe.starts_with("/Volumes")
            || !exe
                .ancestors()
                .any(|p| p.extension().is_some_and(|ext| ext == "app"))
        {
            return Err(
                "Copy LlamaPilot from the disk image to Applications before updating.".into(),
            );
        }
        Ok(())
    }
}

#[cfg(any(windows, test))]
fn windows_install_supported(
    kind: Option<tauri::utils::config::BundleType>,
    marker: Option<&str>,
) -> bool {
    matches!(kind, Some(tauri::utils::config::BundleType::Nsis)) && marker == Some("nsis-v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::utils::config::BundleType;

    #[test]
    fn msi_and_unmarked_executables_never_use_the_nsis_updater() {
        assert!(windows_install_supported(
            Some(BundleType::Nsis),
            Some("nsis-v1")
        ));
        assert!(!windows_install_supported(
            Some(BundleType::Msi),
            Some("nsis-v1")
        ));
        assert!(!windows_install_supported(None, Some("nsis-v1")));
        assert!(!windows_install_supported(Some(BundleType::Nsis), None));
    }
}
