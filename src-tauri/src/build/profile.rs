use serde::{Deserialize, Serialize};

/// GPU/CPU backend, modelled as data so adding Vulkan or HIP later is a table entry rather than
/// new build code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildBackend {
    Cpu,
    Cuda,
}

impl BuildBackend {
    pub fn slug(self) -> &'static str {
        match self {
            BuildBackend::Cpu => "cpu",
            BuildBackend::Cuda => "cuda",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BuildBackend::Cpu => "CPU",
            BuildBackend::Cuda => "CUDA",
        }
    }

    /// CMake definitions that select this backend, per upstream `docs/build.md`.
    pub fn definitions(self) -> Vec<(&'static str, &'static str)> {
        match self {
            BuildBackend::Cpu => Vec::new(),
            BuildBackend::Cuda => vec![("GGML_CUDA", "ON")],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildConfiguration {
    Release,
    RelWithDebInfo,
    Debug,
}

impl BuildConfiguration {
    pub fn cmake_name(self) -> &'static str {
        match self {
            BuildConfiguration::Release => "Release",
            BuildConfiguration::RelWithDebInfo => "RelWithDebInfo",
            BuildConfiguration::Debug => "Debug",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            BuildConfiguration::Release => "release",
            BuildConfiguration::RelWithDebInfo => "relwithdebinfo",
            BuildConfiguration::Debug => "debug",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProfile {
    pub backend: BuildBackend,
    pub configuration: BuildConfiguration,
    /// `None` lets CMake pick its default generator for the platform.
    pub generator: Option<String>,
    /// `None` lets CMake decide the job count.
    pub parallel_jobs: Option<u32>,
    /// Upstream default is a native build tuned to this machine. Turning it off produces a
    /// binary that runs on any CUDA GPU, at the cost of a longer compile.
    pub native_optimizations: bool,
    /// Explicit `CMAKE_CUDA_ARCHITECTURES`, for when nvcc cannot detect the GPU.
    pub cuda_architectures: Option<String>,
    /// Escape hatch, appended verbatim to the configure step.
    pub additional_cmake_args: Vec<String>,
}

impl Default for BuildProfile {
    fn default() -> Self {
        Self {
            backend: BuildBackend::Cuda,
            configuration: BuildConfiguration::Release,
            generator: None,
            parallel_jobs: None,
            native_optimizations: true,
            cuda_architectures: None,
            additional_cmake_args: Vec::new(),
        }
    }
}

impl BuildProfile {
    pub fn cpu() -> Self {
        Self {
            backend: BuildBackend::Cpu,
            ..Self::default()
        }
    }
}
