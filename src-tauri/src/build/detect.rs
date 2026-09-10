use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::process::{self, CommandSpec};

use super::toolchain::{
    parse_cmake_version, parse_generators, parse_git_version, parse_ninja_version,
    parse_nvcc_version, parse_vswhere, CmakeGenerator, ToolId, ToolRequirement, ToolStatus,
    Toolchain,
};

/// Runs a `--version`-style probe and returns stdout+stderr, or `None` when the tool is absent.
async fn probe(program: &Path, args: &[&str]) -> Option<String> {
    let spec = CommandSpec::new(program).args(args);
    let output = process::capture(&spec).await.ok()?;

    output.succeeded().then(|| {
        let mut combined = output.stdout;
        combined.push_str(&output.stderr);
        combined
    })
}

fn resolve(explicit: Option<&PathBuf>, program: &str) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return path.is_file().then(|| path.clone());
    }
    crate::platform::resolve_tool(std::ffi::OsStr::new(program))
}

/// Detects everything needed to build llama.cpp on this machine.
///
/// A missing tool is a normal outcome, never an error: the result describes what is present and
/// what to do about what is not.
pub async fn detect(settings: &Settings) -> Toolchain {
    let mut tools = Vec::new();

    tools.push(detect_git(settings).await);

    let (cmake, generators) = detect_cmake(settings).await;
    tools.push(cmake);

    if cfg!(windows) {
        tools.push(detect_visual_studio().await);
        tools.push(detect_msvc().await);
    } else {
        tools.push(detect_cxx().await);
        tools.push(detect_make().await);
    }
    tools.push(detect_ninja().await);
    if !cfg!(target_os = "macos") {
        tools.push(detect_cuda_toolkit());
        tools.push(detect_nvcc().await);
        tools.push(detect_nvidia_driver().await);
    }
    use super::profile::BuildBackend;
    let mut backends = vec![BuildBackend::Cpu];
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            backends.push(BuildBackend::Metal);
        }
    } else {
        backends.push(BuildBackend::Cuda);
    }
    let default_backend = if backends.contains(&BuildBackend::Metal) {
        BuildBackend::Metal
    } else if tools
        .iter()
        .filter(|tool| tool.requirement == ToolRequirement::RequiredForCuda)
        .all(|tool| tool.found)
        && backends.contains(&BuildBackend::Cuda)
    {
        BuildBackend::Cuda
    } else {
        BuildBackend::Cpu
    };
    Toolchain {
        tools,
        generators,
        backends,
        default_backend,
    }
}

async fn detect_git(settings: &Settings) -> ToolStatus {
    let Some(path) = resolve(settings.git.executable.as_ref(), "git") else {
        return ToolStatus::missing(
            ToolId::Git,
            "Git",
            ToolRequirement::Required,
            "Install Git using your platform package manager or git-scm.com, or select its executable in Settings.",
        );
    };

    let version = probe(&path, &["--version"])
        .await
        .as_deref()
        .and_then(parse_git_version);

    ToolStatus::found(ToolId::Git, "Git", ToolRequirement::Required)
        .with_version(version)
        .with_path(Some(path))
}

async fn detect_cmake(settings: &Settings) -> (ToolStatus, Vec<CmakeGenerator>) {
    let Some(path) = resolve(settings.build.cmake_executable.as_ref(), "cmake") else {
        let status = ToolStatus::missing(
            ToolId::Cmake,
            "CMake",
            ToolRequirement::Required,
            "Install CMake using your platform package manager or cmake.org, or select its executable in Settings.",
        );
        return (status, Vec::new());
    };

    let version = probe(&path, &["--version"])
        .await
        .as_deref()
        .and_then(parse_cmake_version);

    // The generator list comes from CMake itself, so a newly released Visual Studio works
    // without any change here.
    let generators = probe(&path, &["--help"])
        .await
        .map(|help| parse_generators(&help))
        .unwrap_or_default();

    let status = ToolStatus::found(ToolId::Cmake, "CMake", ToolRequirement::Required)
        .with_version(version)
        .with_path(Some(path))
        .with_detail(format!("{} generators available", generators.len()));

    (status, generators)
}

fn vswhere_path() -> Option<PathBuf> {
    let program_files =
        std::env::var_os("ProgramFiles(x86)").or_else(|| std::env::var_os("ProgramFiles"))?;

    let candidate = PathBuf::from(program_files)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");

    candidate.is_file().then_some(candidate)
}

const VS_MISSING_REMEDY: &str =
    "Install Visual Studio 2022 or newer with the \"Desktop development with C++\" workload.";

async fn detect_visual_studio() -> ToolStatus {
    if !cfg!(windows) {
        return ToolStatus::missing(
            ToolId::VisualStudio,
            "Visual Studio",
            ToolRequirement::Optional,
            "Only used on Windows.",
        );
    }

    let Some(vswhere) = vswhere_path() else {
        return ToolStatus::missing(
            ToolId::VisualStudio,
            "Visual Studio",
            ToolRequirement::Required,
            VS_MISSING_REMEDY,
        );
    };

    let output = probe(
        &vswhere,
        &[
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-format",
            "json",
        ],
    )
    .await;

    let Some(install) = output.as_deref().and_then(parse_vswhere) else {
        return ToolStatus::missing(
            ToolId::VisualStudio,
            "Visual Studio",
            ToolRequirement::Required,
            VS_MISSING_REMEDY,
        );
    };

    ToolStatus::found(
        ToolId::VisualStudio,
        "Visual Studio",
        ToolRequirement::Required,
    )
    .with_version(Some(install.version))
    .with_path(Some(install.installation_path))
    .with_detail(install.display_name)
}

/// Reads the MSVC toolset version recorded inside the Visual Studio installation.
async fn detect_msvc() -> ToolStatus {
    let install = match vswhere_path() {
        Some(vswhere) => probe(
            &vswhere,
            &[
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-format",
                "json",
            ],
        )
        .await
        .as_deref()
        .and_then(parse_vswhere),
        None => None,
    };

    let Some(install) = install else {
        return ToolStatus::missing(
            ToolId::Msvc,
            "MSVC toolset",
            ToolRequirement::Required,
            VS_MISSING_REMEDY,
        );
    };

    let version_file = install
        .installation_path
        .join("VC")
        .join("Auxiliary")
        .join("Build")
        .join("Microsoft.VCToolsVersion.default.txt");

    let version = std::fs::read_to_string(&version_file)
        .ok()
        .map(|contents| contents.trim().to_string())
        .filter(|contents| !contents.is_empty());

    let tools_root = version.as_ref().map(|version| {
        install
            .installation_path
            .join("VC")
            .join("Tools")
            .join("MSVC")
            .join(version)
    });

    match version {
        Some(version) => ToolStatus::found(ToolId::Msvc, "MSVC toolset", ToolRequirement::Required)
            .with_version(Some(version))
            .with_path(tools_root),
        None => ToolStatus::missing(
            ToolId::Msvc,
            "MSVC toolset",
            ToolRequirement::Required,
            "Visual Studio is installed but the C++ toolset is not. Add \"Desktop development with C++\" in the Visual Studio installer.",
        ),
    }
}

async fn detect_ninja() -> ToolStatus {
    let Some(path) = resolve(None, "ninja") else {
        return ToolStatus::missing(
            ToolId::Ninja,
            "Ninja",
            ToolRequirement::Optional,
            "Optional. Ninja builds are usually faster than the Visual Studio generator.",
        );
    };

    let version = probe(&path, &["--version"])
        .await
        .as_deref()
        .and_then(parse_ninja_version);

    ToolStatus::found(ToolId::Ninja, "Ninja", ToolRequirement::Optional)
        .with_version(version)
        .with_path(Some(path))
}

fn detect_cuda_toolkit() -> ToolStatus {
    let root = std::env::var_os("CUDA_PATH")
        .or_else(|| std::env::var_os("CUDA_HOME"))
        .map(PathBuf::from)
        .or_else(|| {
            resolve(None, "nvcc")
                .and_then(|p| std::fs::canonicalize(p).ok())
                .and_then(|p| p.parent()?.parent().map(Path::to_path_buf))
        });

    match root.filter(|path| path.is_dir()) {
        Some(path) => ToolStatus::found(ToolId::CudaToolkit, "CUDA Toolkit", ToolRequirement::RequiredForCuda)
            .with_path(Some(path))
            .with_detail("Located via CUDA_PATH, CUDA_HOME or nvcc"),
        None => ToolStatus::missing(
            ToolId::CudaToolkit,
            "CUDA Toolkit",
            ToolRequirement::RequiredForCuda,
            "Install the NVIDIA CUDA Toolkit. Only the CUDA backend needs it; CPU builds work without it.",
        ),
    }
}

async fn detect_nvcc() -> ToolStatus {
    let from_toolkit = std::env::var_os("CUDA_PATH")
        .map(PathBuf::from)
        .map(|root| {
            root.join("bin")
                .join(if cfg!(windows) { "nvcc.exe" } else { "nvcc" })
        })
        .filter(|path| path.is_file());

    let Some(path) = from_toolkit.or_else(|| resolve(None, "nvcc")) else {
        return ToolStatus::missing(
            ToolId::Nvcc,
            "nvcc",
            ToolRequirement::RequiredForCuda,
            "The CUDA compiler was not found. Install the CUDA Toolkit, or add its bin folder to PATH.",
        );
    };

    let version = probe(&path, &["--version"])
        .await
        .as_deref()
        .and_then(parse_nvcc_version);

    ToolStatus::found(ToolId::Nvcc, "nvcc", ToolRequirement::RequiredForCuda)
        .with_version(version)
        .with_path(Some(path))
}

async fn detect_nvidia_driver() -> ToolStatus {
    let gpus = crate::hardware::nvidia::query_gpus().await;

    let Some(driver) = gpus.iter().find_map(|gpu| gpu.driver_version.clone()) else {
        return ToolStatus::missing(
            ToolId::NvidiaDriver,
            "NVIDIA driver",
            ToolRequirement::RequiredForCuda,
            "No NVIDIA GPU was reported by nvidia-smi. CUDA builds will compile but will not run here.",
        );
    };

    ToolStatus::found(
        ToolId::NvidiaDriver,
        "NVIDIA driver",
        ToolRequirement::RequiredForCuda,
    )
    .with_version(Some(driver))
    .with_detail(
        gpus.iter()
            .map(|gpu| gpu.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

async fn detect_cxx() -> ToolStatus {
    let remedy = if cfg!(target_os = "macos") {
        "Install Apple Command Line Tools with xcode-select --install."
    } else {
        "Install a C++ compiler (Ubuntu: sudo apt install build-essential)."
    };
    for name in ["c++", "clang++", "g++"] {
        if let Some(path) = resolve(None, name) {
            if let Some(version) = probe(&path, &["--version"]).await {
                return ToolStatus::found(ToolId::Cxx, "C++ compiler", ToolRequirement::Required)
                    .with_path(Some(path))
                    .with_version(version.lines().next().map(str::to_string));
            }
        }
    }
    ToolStatus::missing(
        ToolId::Cxx,
        "C++ compiler",
        ToolRequirement::Required,
        remedy,
    )
}

async fn detect_make() -> ToolStatus {
    if let Some(path) = resolve(None, "make") {
        if probe(&path, &["--version"]).await.is_some() {
            return ToolStatus::found(ToolId::Make, "Make", ToolRequirement::Required)
                .with_path(Some(path));
        }
    }
    // Ninja is a valid replacement for Make; users can select its generator.
    let requirement = if resolve(None, "ninja").is_some() {
        ToolRequirement::Optional
    } else {
        ToolRequirement::Required
    };
    ToolStatus::missing(
        ToolId::Make,
        "Make",
        requirement,
        "Install Make or Ninja; select Ninja as the generator if Make is unavailable.",
    )
}
