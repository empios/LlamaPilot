use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::process::Command;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

use crate::error::{AppError, AppResult, ErrorCode};

/// Prevents a console window from flashing when a child process is spawned from a GUI app.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hide_console_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

/// A Windows job object holding a process and everything it spawns.
///
/// Builds and llama-server may both spawn descendants. Killing only the process we started can
/// leave those children alive. The job object gives us one handle that terminates the whole tree,
/// and `KILL_ON_JOB_CLOSE` means the tree also dies if this application exits unexpectedly.
pub struct ProcessGroup {
    handle: HANDLE,
    terminated: AtomicBool,
}

// The handle is owned exclusively by this value and only used through the Win32 calls below,
// which are themselves thread-safe.
unsafe impl Send for ProcessGroup {}
unsafe impl Sync for ProcessGroup {}

impl std::fmt::Debug for ProcessGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProcessGroup")
    }
}

impl ProcessGroup {
    pub fn new() -> AppResult<Self> {
        // SAFETY: a null name and null security attributes create an unnamed, private job.
        let handle = unsafe { CreateJobObjectW(None, None) }.map_err(|error| {
            AppError::internal("Could not create a Windows job object for process supervision.")
                .with_details(error.to_string())
        })?;

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        // SAFETY: `limits` outlives the call and its size is described exactly.
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if let Err(error) = configured {
            // SAFETY: the handle was created successfully just above.
            let _ = unsafe { CloseHandle(handle) };
            return Err(
                AppError::internal("Could not configure the Windows job object.")
                    .with_details(error.to_string()),
            );
        }

        Ok(Self {
            handle,
            terminated: AtomicBool::new(false),
        })
    }

    pub fn prepare(&self, _command: &mut Command) {}

    pub fn release(&self, _process_id: u32) {}

    /// Adds an already-spawned process to the group.
    pub fn adopt(&self, process_id: u32) -> AppResult<()> {
        // SAFETY: the process id comes from a child we just spawned and still hold.
        let process =
            unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, process_id) }
                .map_err(|error| {
                    AppError::internal("Could not open the child process for supervision.")
                        .with_details(error.to_string())
                })?;

        // SAFETY: both handles are valid and owned here.
        let assigned = unsafe { AssignProcessToJobObject(self.handle, process) };
        // SAFETY: the process handle is no longer needed once assignment is attempted.
        let _ = unsafe { CloseHandle(process) };

        assigned.map_err(|error| {
            AppError::internal("Could not place the child process under supervision.")
                .with_details(error.to_string())
        })?;

        // Cancellation may race with process creation. If the job was terminated before this
        // process could be assigned, terminate it again now that the child belongs to the job.
        if self.is_terminated() {
            // SAFETY: the handle is valid for the lifetime of this value.
            let _ = unsafe { TerminateJobObject(self.handle, 1) };
        }

        Ok(())
    }

    /// Terminates every process in the group.
    pub fn terminate(&self) {
        self.terminated.store(true, Ordering::Release);
        // SAFETY: the handle is valid for the lifetime of this value.
        let _ = unsafe { TerminateJobObject(self.handle, 1) };
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::Acquire)
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // Closing the handle terminates the tree, because of KILL_ON_JOB_CLOSE.
        // SAFETY: the handle is valid and dropped exactly once.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

pub fn reveal_in_file_manager(path: &Path) -> AppResult<()> {
    let argument = if path.is_dir() {
        path.as_os_str().to_os_string()
    } else {
        let mut selected = std::ffi::OsString::from("/select,");
        selected.push(path.as_os_str());
        selected
    };

    std::process::Command::new("explorer.exe")
        .arg(argument)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            AppError::new(ErrorCode::Io, "Could not open the folder in File Explorer.")
                .with_details(error.to_string())
        })
}
