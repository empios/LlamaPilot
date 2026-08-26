#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod unix;

#[cfg(windows)]
pub use windows::{hide_console_window, reveal_in_file_manager, ProcessGroup};

#[cfg(not(windows))]
pub use unix::{hide_console_window, reveal_in_file_manager, ProcessGroup};
