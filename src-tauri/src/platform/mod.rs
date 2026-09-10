#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod unix;

#[cfg(windows)]
pub use windows::{hide_console_window, reveal_in_file_manager, ProcessGroup};

#[cfg(not(windows))]
pub use unix::{hide_console_window, reveal_in_file_manager, ProcessGroup};

/// Preserve user PATH precedence and append conventional desktop tool locations.
pub fn tool_path() -> std::ffi::OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    #[allow(unused_mut)]
    let mut paths: Vec<_> = std::env::split_paths(&current).collect();
    #[cfg(unix)]
    for directory in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/cuda/bin",
    ] {
        let path = std::path::PathBuf::from(directory);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    std::env::join_paths(paths).unwrap_or(current)
}

pub fn resolve_tool(program: &std::ffi::OsStr) -> Option<std::path::PathBuf> {
    which::which_in(program, Some(tool_path()), std::env::current_dir().ok()?).ok()
}
