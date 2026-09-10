use crate::error::{AppError, AppResult, ErrorCode};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tokio::process::Command;

pub fn hide_console_window(_command: &mut Command) {}

/// Each command leads a separate process group. A minimal forked watchdog owns a pipe
/// whose EOF signals parent death. Only async-signal-safe libc calls run after fork.
#[derive(Debug, Default)]
pub struct ProcessGroup {
    terminated: AtomicBool,
    children: Mutex<Vec<Watchdog>>,
}

#[derive(Debug)]
struct Watchdog {
    process: libc::pid_t,
    watcher: libc::pid_t,
    writer: libc::c_int,
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        // SAFETY: writer and watcher are exclusively owned; closing the pipe wakes the watcher.
        unsafe {
            libc::close(self.writer);
            while libc::waitpid(self.watcher, std::ptr::null_mut(), 0) == -1 {
                if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                    break;
                }
            }
        }
    }
}

impl ProcessGroup {
    pub fn new() -> AppResult<Self> {
        Ok(Self::default())
    }
    pub fn prepare(&self, command: &mut Command) {
        command.process_group(0);
    }
    pub fn adopt(&self, process_id: u32) -> AppResult<()> {
        let mut children = self.children.lock().unwrap_or_else(|e| e.into_inner());
        let process =
            i32::try_from(process_id).map_err(|_| AppError::internal("Invalid child PID."))?;
        let mut pipe = [-1; 2];
        // SAFETY: pipe points to two writable integers. No Rust or allocator calls occur in
        // the forked child. All inherited descriptors except the pipe reader are closed.
        unsafe {
            if libc::pipe(pipe.as_mut_ptr()) != 0 {
                return Err(supervision_error());
            }
            if libc::fcntl(pipe[0], libc::F_SETFD, libc::FD_CLOEXEC) == -1
                || libc::fcntl(pipe[1], libc::F_SETFD, libc::FD_CLOEXEC) == -1
            {
                libc::close(pipe[0]);
                libc::close(pipe[1]);
                return Err(supervision_error());
            }
            let max_fd = libc::sysconf(libc::_SC_OPEN_MAX);
            if max_fd < 0 {
                libc::close(pipe[0]);
                libc::close(pipe[1]);
                return Err(supervision_error());
            }
            let watcher = libc::fork();
            if watcher == 0 {
                for fd in 0..max_fd {
                    if fd as i32 != pipe[0] {
                        libc::close(fd as i32);
                    }
                }
                let mut byte = 0u8;
                loop {
                    let count = libc::read(pipe[0], &mut byte as *mut u8 as *mut libc::c_void, 1);
                    if count == 0 {
                        break;
                    }
                    if count < 0 {
                        continue;
                    }
                }
                libc::kill(-process, libc::SIGKILL);
                libc::_exit(0);
            }
            libc::close(pipe[0]);
            if watcher < 0 {
                libc::close(pipe[1]);
                return Err(supervision_error());
            }
            let guard = Watchdog {
                process,
                watcher,
                writer: pipe[1],
            };
            if self.is_terminated() {
                drop(guard);
            } else {
                children.push(guard);
            }
        }
        Ok(())
    }
    /// Release immediately after wait(), preventing retained PIDs from being reused later.
    pub fn release(&self, process_id: u32) {
        self.children
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|child| child.process as u32 != process_id);
    }
    pub fn terminate(&self) {
        self.terminated.store(true, Ordering::Release);
        self.children
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::Acquire)
    }
}

fn supervision_error() -> AppError {
    AppError::internal("Could not start the process watchdog.")
        .with_details(std::io::Error::last_os_error().to_string())
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
