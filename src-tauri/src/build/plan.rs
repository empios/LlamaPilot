use std::path::{Path, PathBuf};

use serde::Serialize;

use super::profile::BuildProfile;
use super::toolchain::is_multi_config;

/// The llama.cpp CMake target that produces the server binary.
pub const SERVER_TARGET: &str = "llama-server";

/// Everything needed to run a build, derived purely from a profile.
///
/// Keeping this a pure function is what makes the argument construction testable without CMake,
/// a source tree, or a GPU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlan {
    pub build_directory: PathBuf,
    pub configure_args: Vec<String>,
    pub build_args: Vec<String>,
    pub multi_config: bool,
}

pub fn plan_build(
    source_directory: &Path,
    build_directory: &Path,
    profile: &BuildProfile,
) -> BuildPlan {
    let multi_config = profile
        .generator
        .as_deref()
        .map(is_multi_config)
        // With no explicit generator CMake picks the platform default, which is a Visual Studio
        // generator on Windows and therefore multi-config.
        .unwrap_or(cfg!(windows));

    let mut configure = vec![
        "-S".to_string(),
        source_directory.display().to_string(),
        "-B".to_string(),
        build_directory.display().to_string(),
    ];

    if let Some(generator) = profile
        .generator
        .as_deref()
        .filter(|g| !g.trim().is_empty())
    {
        configure.push("-G".to_string());
        configure.push(generator.to_string());
    }

    // Single-config generators bake the configuration in at configure time; multi-config ones
    // take it at build time via --config.
    if !multi_config {
        configure.push(format!(
            "-DCMAKE_BUILD_TYPE={}",
            profile.configuration.cmake_name()
        ));
    }

    for (key, value) in profile.backend.definitions() {
        configure.push(format!("-D{key}={value}"));
    }

    if !profile.native_optimizations {
        configure.push("-DGGML_NATIVE=OFF".to_string());
    }

    if let Some(architectures) = profile
        .cuda_architectures
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        configure.push(format!("-DCMAKE_CUDA_ARCHITECTURES={architectures}"));
    }

    // Only the server is needed, and skipping the rest of the tools roughly halves build time.
    configure.push("-DBUILD_SHARED_LIBS=OFF".to_string());
    configure.push("-DCMAKE_BUILD_RPATH_USE_ORIGIN=ON".to_string());
    configure.push("-DLLAMA_BUILD_SERVER=ON".to_string());
    configure.push("-DLLAMA_BUILD_TESTS=OFF".to_string());
    configure.push("-DLLAMA_BUILD_EXAMPLES=OFF".to_string());

    configure.extend(
        profile
            .additional_cmake_args
            .iter()
            .map(|argument| argument.trim().to_string())
            .filter(|argument| !argument.is_empty()),
    );

    let mut build = vec!["--build".to_string(), build_directory.display().to_string()];

    if multi_config {
        build.push("--config".to_string());
        build.push(profile.configuration.cmake_name().to_string());
    }

    build.push("--target".to_string());
    build.push(SERVER_TARGET.to_string());

    build.push("--parallel".to_string());
    if let Some(jobs) = profile.parallel_jobs.filter(|jobs| *jobs > 0) {
        build.push(jobs.to_string());
    }

    BuildPlan {
        build_directory: build_directory.to_path_buf(),
        configure_args: configure,
        build_args: build,
        multi_config,
    }
}

/// Where the built binary lands, which differs between generator kinds.
///
/// llama.cpp sets `CMAKE_RUNTIME_OUTPUT_DIRECTORY` to `<build>/bin`, and multi-config generators
/// append the configuration below that.
pub fn artifact_directories(plan: &BuildPlan, configuration: &str) -> Vec<PathBuf> {
    let bin = plan.build_directory.join("bin");

    if plan.multi_config {
        vec![bin.join(configuration), bin]
    } else {
        vec![bin]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::profile::{BuildBackend, BuildConfiguration};

    fn source() -> PathBuf {
        PathBuf::from("/src/llama.cpp")
    }

    fn build() -> PathBuf {
        PathBuf::from("/builds/cuda-release")
    }

    fn cuda_profile() -> BuildProfile {
        BuildProfile {
            backend: BuildBackend::Cuda,
            configuration: BuildConfiguration::Release,
            generator: Some("Visual Studio 18 2026".into()),
            parallel_jobs: None,
            native_optimizations: true,
            cuda_architectures: None,
            additional_cmake_args: Vec::new(),
        }
    }

    #[test]
    fn a_cuda_build_matches_the_upstream_documented_shape() {
        let plan = plan_build(&source(), &build(), &cuda_profile());

        assert_eq!(
            plan.configure_args,
            vec![
                "-S",
                "/src/llama.cpp",
                "-B",
                "/builds/cuda-release",
                "-G",
                "Visual Studio 18 2026",
                "-DGGML_CUDA=ON",
                "-DGGML_METAL=OFF",
                "-DBUILD_SHARED_LIBS=OFF",
                "-DCMAKE_BUILD_RPATH_USE_ORIGIN=ON",
                "-DLLAMA_BUILD_SERVER=ON",
                "-DLLAMA_BUILD_TESTS=OFF",
                "-DLLAMA_BUILD_EXAMPLES=OFF",
            ]
        );
        assert_eq!(
            plan.build_args,
            vec![
                "--build",
                "/builds/cuda-release",
                "--config",
                "Release",
                "--target",
                "llama-server",
                "--parallel",
            ]
        );
    }

    #[test]
    fn a_cpu_build_explicitly_disables_gpu_backends() {
        let profile = BuildProfile {
            backend: BuildBackend::Cpu,
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(plan.configure_args.contains(&"-DGGML_CUDA=OFF".to_string()));
        assert!(plan
            .configure_args
            .contains(&"-DGGML_METAL=OFF".to_string()));
    }

    #[test]
    fn single_config_generators_take_the_build_type_at_configure_time() {
        let profile = BuildProfile {
            generator: Some("Ninja".into()),
            configuration: BuildConfiguration::RelWithDebInfo,
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(!plan.multi_config);
        assert!(plan
            .configure_args
            .contains(&"-DCMAKE_BUILD_TYPE=RelWithDebInfo".to_string()));
        assert!(!plan.build_args.contains(&"--config".to_string()));
    }

    #[test]
    fn multi_config_generators_take_the_configuration_at_build_time() {
        let profile = BuildProfile {
            generator: Some("Ninja Multi-Config".into()),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(plan.multi_config);
        assert!(!plan
            .configure_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_BUILD_TYPE")));
        assert!(plan.build_args.contains(&"--config".to_string()));
    }

    #[test]
    fn disabling_native_optimizations_emits_the_documented_flag() {
        let profile = BuildProfile {
            native_optimizations: false,
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(plan
            .configure_args
            .contains(&"-DGGML_NATIVE=OFF".to_string()));
    }

    #[test]
    fn explicit_cuda_architectures_are_passed_through() {
        let profile = BuildProfile {
            cuda_architectures: Some(" 86;89 ".into()),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(plan
            .configure_args
            .contains(&"-DCMAKE_CUDA_ARCHITECTURES=86;89".to_string()));
    }

    #[test]
    fn blank_cuda_architectures_are_ignored() {
        let profile = BuildProfile {
            cuda_architectures: Some("   ".into()),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert!(!plan
            .configure_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_CUDA_ARCHITECTURES")));
    }

    #[test]
    fn additional_arguments_are_appended_verbatim_and_stay_separate() {
        let profile = BuildProfile {
            additional_cmake_args: vec![
                "-DGGML_CUDA_FORCE_MMQ=ON".into(),
                "  ".into(),
                "-DCMAKE_CUDA_COMPILER=C:/cuda/bin/nvcc.exe".into(),
            ],
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        let tail = &plan.configure_args[plan.configure_args.len() - 2..];
        assert_eq!(
            tail,
            [
                "-DGGML_CUDA_FORCE_MMQ=ON",
                "-DCMAKE_CUDA_COMPILER=C:/cuda/bin/nvcc.exe"
            ]
        );
    }

    #[test]
    fn an_explicit_job_count_is_passed_to_parallel() {
        let profile = BuildProfile {
            parallel_jobs: Some(16),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        let parallel = plan
            .build_args
            .iter()
            .position(|arg| arg == "--parallel")
            .expect("parallel flag");
        assert_eq!(plan.build_args[parallel + 1], "16");
    }

    #[test]
    fn a_zero_job_count_falls_back_to_cmake_deciding() {
        let profile = BuildProfile {
            parallel_jobs: Some(0),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert_eq!(
            plan.build_args.last().map(String::as_str),
            Some("--parallel")
        );
    }

    #[test]
    fn multi_config_artifacts_are_searched_under_the_configuration_directory() {
        let plan = plan_build(&source(), &build(), &cuda_profile());

        assert_eq!(
            artifact_directories(&plan, "Release"),
            vec![
                PathBuf::from("/builds/cuda-release/bin/Release"),
                PathBuf::from("/builds/cuda-release/bin"),
            ]
        );
    }

    #[test]
    fn single_config_artifacts_land_directly_in_bin() {
        let profile = BuildProfile {
            generator: Some("Ninja".into()),
            ..cuda_profile()
        };
        let plan = plan_build(&source(), &build(), &profile);

        assert_eq!(
            artifact_directories(&plan, "Release"),
            vec![PathBuf::from("/builds/cuda-release/bin")]
        );
    }
}
