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
    pub unified_memory: bool,
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

    let unified_memory = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    let gpus = if unified_memory {
        apple_gpus(&memory).await
    } else {
        nvidia::query_gpus().await
    };
    let nvidia_driver = gpus.iter().find_map(|gpu| gpu.driver_version.clone());

    HardwareSnapshot {
        nvidia_present: !unified_memory && !gpus.is_empty(),
        unified_memory,
        nvidia_driver,
        gpus,
        cpu,
        memory,
    }
}

async fn apple_gpus(memory: &MemoryInfo) -> Vec<GpuInfo> {
    let spec = crate::process::CommandSpec::new("/usr/sbin/system_profiler")
        .args(["SPDisplaysDataType", "-json"]);
    let Ok(output) = crate::process::capture(&spec).await else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&output.stdout) else {
        return Vec::new();
    };
    let Some(displays) = value["SPDisplaysDataType"].as_array() else {
        return Vec::new();
    };
    displays
        .iter()
        .enumerate()
        .filter_map(|(index, display)| {
            let name = display["sppci_model"].as_str()?;
            Some(GpuInfo {
                index: index as u32,
                name: name.to_string(),
                total_memory_mib: memory.total_bytes / (1024 * 1024),
                used_memory_mib: memory.total_bytes.saturating_sub(memory.available_bytes)
                    / (1024 * 1024),
                free_memory_mib: memory.available_bytes / (1024 * 1024),
                driver_version: None,
            })
        })
        .collect()
}
