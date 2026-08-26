use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::process::Command;

use crate::error::{AppError, AppResult, ErrorCode};

/// No-op outside Windows: there is no console window to suppress.
pub fn hide_console_window(_command: &mut Command) {}

/// Placeholder matching the Windows job object API.
///
/// Windows is the MVP target; on other platforms cancellation falls back to killing the direct
/// child, which is why `adopt` and `terminate` are deliberately inert rather than pretending.
#[derive(Debug, Default)]
pub struct ProcessGroup {
    terminated: AtomicBool,
}

impl ProcessGroup {
    pub fn new() -> AppResult<Self> {
        Ok(Self::default())
    }

    pub fn adopt(&self, _process_id: u32) -> AppResult<()> {
        Ok(())
    }

    pub fn terminate(&self) {
        self.terminated.store(true, Ordering::Release);
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::Acquire)
    }
}

pub fn reveal_in_file_manager(path: &Path) -> AppResult<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let target = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    std::process::Command::new(program)
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                "Could not open the folder in the file manager.",
            )
            .with_details(error.to_string())
        })
}
