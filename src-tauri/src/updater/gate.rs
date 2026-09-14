use std::sync::Mutex;

use crate::error::{AppError, AppResult, ErrorCode};

/// One admission point for commands that launch work or read/write persistent data.
/// The mutex is never held over an await; leases count work until its future completes.
#[derive(Default)]
pub struct WorkGate(Mutex<GateState>);

#[derive(Default)]
struct GateState {
    active: usize,
    installing: bool,
}

pub struct WorkLease<'a> {
    gate: &'a WorkGate,
    exclusive: bool,
}

impl WorkGate {
    pub fn enter(&self) -> AppResult<WorkLease<'_>> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.installing {
            return Err(AppError::new(
                ErrorCode::UpdateBusy,
                "An application update is being installed. Please wait.",
            ));
        }
        state.active += 1;
        Ok(WorkLease {
            gate: self,
            exclusive: false,
        })
    }

    pub fn install(&self) -> AppResult<WorkLease<'_>> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.installing || state.active > 0 {
            return Err(AppError::new(
                ErrorCode::UpdateBusy,
                "Waiting for application operations to finish.",
            ));
        }
        state.installing = true;
        Ok(WorkLease {
            gate: self,
            exclusive: true,
        })
    }

    pub fn busy(&self) -> bool {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.active > 0 || state.installing
    }
}

impl Drop for WorkLease<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.0.lock().unwrap_or_else(|e| e.into_inner());
        if self.exclusive {
            state.installing = false;
        } else {
            state.active -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_and_installation_are_mutually_exclusive_and_recover_after_failure() {
        let gate = WorkGate::default();
        let work = gate.enter().unwrap();
        assert!(gate.install().is_err());
        drop(work);
        let install = gate.install().unwrap();
        assert!(gate.enter().is_err());
        assert!(gate.install().is_err());
        drop(install);
        assert!(gate.enter().is_ok());
    }

    #[test]
    fn all_concurrent_work_must_finish() {
        let gate = WorkGate::default();
        let first = gate.enter().unwrap();
        let second = gate.enter().unwrap();
        drop(first);
        assert!(gate.install().is_err());
        drop(second);
        assert!(gate.install().is_ok());
    }

    #[test]
    fn simultaneous_start_and_install_never_both_succeed() {
        use std::sync::{Arc, Barrier};
        for _ in 0..100 {
            let gate = Arc::new(WorkGate::default());
            let barrier = Arc::new(Barrier::new(2));
            let worker_gate = gate.clone();
            let worker_barrier = barrier.clone();
            let worker = std::thread::spawn(move || {
                worker_barrier.wait();
                let lease = worker_gate.enter();
                worker_barrier.wait();
                lease.is_ok()
            });
            barrier.wait();
            let install = gate.install();
            barrier.wait();
            assert_ne!(install.is_ok(), worker.join().unwrap());
        }
    }
}
