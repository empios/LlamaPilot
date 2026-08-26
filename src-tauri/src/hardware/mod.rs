pub mod nvidia;

use serde::Serialize;
use sysinfo::System;

pub use nvidia::GpuInfo;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub brand: String,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareSnapshot {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub nvidia_driver: Option<String>,
    /// True when an NVIDIA GPU was detected. It does not imply the CUDA toolkit is installed —
    /// that is reported separately by toolchain detection.
    pub nvidia_present: bool,
}

/// Collects a point-in-time view of the machine.
///
/// Deliberately shallow: this is context for sizing a model, not a hardware monitor.
pub async fn snapshot() -> HardwareSnapshot {
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_all();

    let cpus = system.cpus();
    let cpu = CpuInfo {
        brand: cpus
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|brand| !brand.is_empty())
            .unwrap_or_else(|| "Unknown CPU".to_string()),
        physical_cores: System::physical_core_count(),
        logical_cores: cpus.len(),
    };

    let memory = MemoryInfo {
        total_bytes: system.total_memory(),
        available_bytes: system.available_memory(),
    };

    let gpus = nvidia::query_gpus().await;
    let nvidia_driver = gpus.iter().find_map(|gpu| gpu.driver_version.clone());

    HardwareSnapshot {
        nvidia_present: !gpus.is_empty(),
        nvidia_driver,
        gpus,
        cpu,
        memory,
    }
}
