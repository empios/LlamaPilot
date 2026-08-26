use std::path::{Path, PathBuf};

use super::profile::BuildProfile;

/// Reduces a generator name to a short, filesystem-safe token.
fn generator_slug(generator: Option<&str>) -> String {
    let Some(generator) = generator.map(str::trim).filter(|value| !value.is_empty()) else {
        return "default".to_string();
    };

    let slug: String = generator
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();

    let collapsed = slug
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if collapsed.is_empty() {
        "default".to_string()
    } else {
        collapsed
    }
}

/// Build directory name, keyed by everything that makes two build trees incompatible.
///
/// CMake cannot reuse a binary directory across generators or, for single-config generators,
/// across build types, so each combination gets its own tree instead of silently corrupting one.
pub fn build_directory_name(
    source_name: &str,
    profile: &BuildProfile,
    architecture: &str,
) -> String {
    let source_slug: String = source_name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '.' {
                character
            } else {
                '-'
            }
        })
        .collect();

    let source_slug = source_slug
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    format!(
        "{}-{}-{}-{}-{}",
        if source_slug.is_empty() {
            "source"
        } else {
            &source_slug
        },
        profile.backend.slug(),
        architecture,
        generator_slug(profile.generator.as_deref()),
        profile.configuration.slug(),
    )
}

pub fn build_directory(
    builds_root: &Path,
    source_name: &str,
    profile: &BuildProfile,
    architecture: &str,
) -> PathBuf {
    builds_root.join(build_directory_name(source_name, profile, architecture))
}

/// A configured tree records the generator it was made with in `CMakeCache.txt`.
pub fn is_configured(build_directory: &Path) -> bool {
    build_directory.join("CMakeCache.txt").is_file()
}

/// Reads `CMAKE_GENERATOR` back out of an existing cache, so a generator change can be detected
/// and reported instead of producing a confusing CMake error.
pub fn cached_generator(build_directory: &Path) -> Option<String> {
    let cache = std::fs::read_to_string(build_directory.join("CMakeCache.txt")).ok()?;
    parse_cached_generator(&cache)
}

pub fn parse_cached_generator(cache: &str) -> Option<String> {
    cache.lines().find_map(|line| {
        let value = line.strip_prefix("CMAKE_GENERATOR:INTERNAL=")?;
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::profile::{BuildBackend, BuildConfiguration};

    fn profile() -> BuildProfile {
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
    fn the_name_encodes_source_backend_architecture_generator_and_configuration() {
        assert_eq!(
            build_directory_name("llama.cpp", &profile(), "x64"),
            "llama.cpp-cuda-x64-visual-studio-18-2026-release"
        );
    }

    #[test]
    fn different_generators_get_different_trees() {
        let visual_studio = build_directory_name("llama.cpp", &profile(), "x64");
        let ninja = build_directory_name(
            "llama.cpp",
            &BuildProfile {
                generator: Some("Ninja".into()),
                ..profile()
            },
            "x64",
        );

        assert_ne!(visual_studio, ninja);
    }

    #[test]
    fn different_backends_and_configurations_get_different_trees() {
        let cuda_release = build_directory_name("llama.cpp", &profile(), "x64");
        let cpu_release = build_directory_name(
            "llama.cpp",
            &BuildProfile {
                backend: BuildBackend::Cpu,
                ..profile()
            },
            "x64",
        );
        let cuda_debug = build_directory_name(
            "llama.cpp",
            &BuildProfile {
                configuration: BuildConfiguration::Debug,
                ..profile()
            },
            "x64",
        );

        assert_ne!(cuda_release, cpu_release);
        assert_ne!(cuda_release, cuda_debug);
    }

    #[test]
    fn no_generator_is_recorded_as_the_cmake_default() {
        let name = build_directory_name(
            "llama.cpp",
            &BuildProfile {
                generator: None,
                ..profile()
            },
            "x64",
        );

        assert!(name.contains("-default-"));
    }

    #[test]
    fn source_names_are_reduced_to_safe_path_segments() {
        let name = build_directory_name("My Fork/llama.cpp!", &profile(), "x64");

        assert!(name.starts_with("my-fork-llama.cpp-"));
        assert!(!name.contains('/'));
        assert!(!name.contains('!'));
        assert!(!name.contains(' '));
    }

    #[test]
    fn reads_the_generator_back_out_of_a_cmake_cache() {
        let cache = "\
//Name of generator.
CMAKE_GENERATOR:INTERNAL=Visual Studio 18 2026
//Generator instance identifier.
CMAKE_GENERATOR_INSTANCE:INTERNAL=C:/Program Files/Microsoft Visual Studio/18/Community
";

        assert_eq!(
            parse_cached_generator(cache).as_deref(),
            Some("Visual Studio 18 2026")
        );
    }

    #[test]
    fn a_cache_without_a_generator_yields_none() {
        assert_eq!(
            parse_cached_generator("CMAKE_HOME_DIRECTORY:INTERNAL=/src"),
            None
        );
    }
}
