use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A CMake generator, discovered from `cmake --help` rather than hardcoded.
///
/// Hardcoding "Visual Studio 17 2022" would already be wrong on a machine with Visual Studio
/// 2026, which is exactly the staleness this project exists to avoid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmakeGenerator {
    pub name: String,
    pub is_default: bool,
    /// Multi-config generators need `--config` at build time instead of `CMAKE_BUILD_TYPE`.
    pub multi_config: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolRequirement {
    /// Nothing can be built without it.
    Required,
    /// Only needed for the CUDA backend.
    RequiredForCuda,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolId {
    Git,
    Cmake,
    VisualStudio,
    Msvc,
    Ninja,
    CudaToolkit,
    Nvcc,
    NvidiaDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: ToolId,
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub detail: Option<String>,
    pub requirement: ToolRequirement,
    /// What the user should do when it is missing. Never just "not found".
    pub remedy: Option<String>,
}

impl ToolStatus {
    pub fn missing(id: ToolId, name: &str, requirement: ToolRequirement, remedy: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            found: false,
            version: None,
            path: None,
            detail: None,
            requirement,
            remedy: Some(remedy.to_string()),
        }
    }

    pub fn found(id: ToolId, name: &str, requirement: ToolRequirement) -> Self {
        Self {
            id,
            name: name.to_string(),
            found: true,
            version: None,
            path: None,
            detail: None,
            requirement,
            remedy: None,
        }
    }

    pub fn with_version(mut self, version: Option<String>) -> Self {
        self.version = version;
        self
    }

    pub fn with_path(mut self, path: Option<PathBuf>) -> Self {
        self.path = path;
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toolchain {
    pub tools: Vec<ToolStatus>,
    pub generators: Vec<CmakeGenerator>,
}

impl Toolchain {
    pub fn tool(&self, id: ToolId) -> Option<&ToolStatus> {
        self.tools.iter().find(|tool| tool.id == id)
    }

    /// True when everything needed for a CPU build is present.
    pub fn can_build_cpu(&self) -> bool {
        self.tools
            .iter()
            .filter(|tool| tool.requirement == ToolRequirement::Required)
            .all(|tool| tool.found)
    }

    /// True when everything needed for a CUDA build is present.
    pub fn can_build_cuda(&self) -> bool {
        self.can_build_cpu()
            && self
                .tools
                .iter()
                .filter(|tool| tool.requirement == ToolRequirement::RequiredForCuda)
                .all(|tool| tool.found)
    }

    pub fn default_generator(&self) -> Option<&CmakeGenerator> {
        self.generators
            .iter()
            .find(|generator| generator.is_default)
    }

    pub fn generator(&self, name: &str) -> Option<&CmakeGenerator> {
        self.generators
            .iter()
            .find(|generator| generator.name == name)
    }
}

/// Reads the first version-looking token out of a `--version` banner.
fn first_version_token(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|token| {
        let cleaned = token.trim_matches(|character: char| !character.is_ascii_alphanumeric());
        let looks_like_version = cleaned.contains('.')
            && cleaned.starts_with(|character: char| character.is_ascii_digit());

        looks_like_version.then(|| cleaned.to_string())
    })
}

pub fn parse_cmake_version(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains("cmake version"))?;
    first_version_token(line.trim_start_matches("cmake version"))
}

pub fn parse_git_version(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains("git version"))?;
    first_version_token(line.trim_start_matches("git version"))
}

pub fn parse_ninja_version(output: &str) -> Option<String> {
    first_version_token(output)
}

/// `nvcc --version` ends with `Cuda compilation tools, release 13.3, V13.3.33`.
pub fn parse_nvcc_version(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains("release "))?;
    let after_release = line.split("release ").nth(1)?;
    let candidate = after_release
        .split(',')
        .next()?
        .trim()
        .trim_end_matches('.');

    (!candidate.is_empty()).then(|| candidate.to_string())
}

/// Parses the generator table from `cmake --help`.
///
/// Lines look like `* Visual Studio 18 2026        = Generates ...`, with continuation lines for
/// long descriptions that must not be mistaken for generators.
pub fn parse_generators(output: &str) -> Vec<CmakeGenerator> {
    let mut generators = Vec::new();

    for line in output.lines() {
        let Some((left, _description)) = line.split_once('=') else {
            continue;
        };

        // Generator entries are indented by at most a marker plus two spaces; deeper
        // indentation belongs to a wrapped description.
        let indent = left.len() - left.trim_start().len();
        if indent > 4 {
            continue;
        }

        let trimmed = left.trim();
        let is_default = trimmed.starts_with('*');
        let name = trimmed.trim_start_matches('*').trim();

        if name.is_empty() || name.contains("  ") {
            continue;
        }

        // `[arch]` placeholders appear in some generator names; keep the base name only.
        let name = name.split('[').next().unwrap_or(name).trim().to_string();
        if name.is_empty() || generators.iter().any(|g: &CmakeGenerator| g.name == name) {
            continue;
        }

        let multi_config = is_multi_config(&name);
        generators.push(CmakeGenerator {
            name,
            is_default,
            multi_config,
        });
    }

    generators
}

/// Multi-config generators build every configuration from one tree and select at build time.
pub fn is_multi_config(generator: &str) -> bool {
    generator.starts_with("Visual Studio")
        || generator == "Xcode"
        || generator.contains("Multi-Config")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualStudioInstall {
    pub display_name: String,
    pub version: String,
    pub installation_path: PathBuf,
}

/// Parses `vswhere -format json` output, taking the first (latest) instance.
pub fn parse_vswhere(output: &str) -> Option<VisualStudioInstall> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Instance {
        display_name: Option<String>,
        installation_version: Option<String>,
        installation_path: Option<String>,
    }

    let instances: Vec<Instance> = serde_json::from_str(output).ok()?;
    let instance = instances.into_iter().next()?;

    Some(VisualStudioInstall {
        display_name: instance
            .display_name
            .unwrap_or_else(|| "Visual Studio".into()),
        version: instance.installation_version.unwrap_or_default(),
        installation_path: PathBuf::from(instance.installation_path?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMAKE_HELP: &str = "\
Generators

The following generators are available on this platform (* marks default):
* Visual Studio 18 2026        = Generates Visual Studio 2026 project files.
                                 Use -A option to specify architecture.
  Visual Studio 17 2022        = Generates Visual Studio 2022 project files.
                                 Use -A option to specify architecture.
  NMake Makefiles              = Generates NMake makefiles.
  Ninja                        = Generates build.ninja files.
  Ninja Multi-Config           = Generates build-<Config>.ninja files.
  Unix Makefiles               = Generates standard UNIX makefiles.
";

    const VSWHERE_JSON: &str = r#"[
  {
    "instanceId": "bd1a0cef",
    "installationPath": "C:\\Program Files\\Microsoft Visual Studio\\18\\Community",
    "installationVersion": "18.7.11911.148",
    "displayName": "Visual Studio Community 2026"
  }
]"#;

    #[test]
    fn parses_the_generator_table_and_marks_the_default() {
        let generators = parse_generators(CMAKE_HELP);
        let names: Vec<&str> = generators.iter().map(|g| g.name.as_str()).collect();

        assert_eq!(
            names,
            vec![
                "Visual Studio 18 2026",
                "Visual Studio 17 2022",
                "NMake Makefiles",
                "Ninja",
                "Ninja Multi-Config",
                "Unix Makefiles",
            ]
        );

        let default = generators.iter().find(|g| g.is_default).expect("a default");
        assert_eq!(default.name, "Visual Studio 18 2026");
    }

    #[test]
    fn wrapped_description_lines_are_not_mistaken_for_generators() {
        let generators = parse_generators(CMAKE_HELP);
        assert!(!generators
            .iter()
            .any(|generator| generator.name.contains("Use -A option")));
    }

    #[test]
    fn identifies_multi_config_generators() {
        let generators = parse_generators(CMAKE_HELP);
        let multi: Vec<&str> = generators
            .iter()
            .filter(|g| g.multi_config)
            .map(|g| g.name.as_str())
            .collect();

        assert_eq!(
            multi,
            vec![
                "Visual Studio 18 2026",
                "Visual Studio 17 2022",
                "Ninja Multi-Config"
            ]
        );
        assert!(!is_multi_config("Ninja"));
        assert!(!is_multi_config("Unix Makefiles"));
    }

    #[test]
    fn parses_tool_versions() {
        assert_eq!(
            parse_cmake_version("cmake version 4.4.0\n\nCMake suite maintained...").as_deref(),
            Some("4.4.0")
        );
        assert_eq!(
            parse_git_version("git version 2.54.0.windows.1").as_deref(),
            Some("2.54.0.windows.1")
        );
        assert_eq!(parse_ninja_version("1.13.1\n").as_deref(), Some("1.13.1"));
    }

    #[test]
    fn parses_the_cuda_release_from_nvcc() {
        let output = "\
nvcc: NVIDIA (R) Cuda compiler driver
Copyright (c) 2005-2026 NVIDIA Corporation
Built on Tue_Jun_10_19:20:14_2026
Cuda compilation tools, release 13.3, V13.3.33
Build cuda_13.3.r13.3/compiler.36000000_0
";
        assert_eq!(parse_nvcc_version(output).as_deref(), Some("13.3"));
    }

    #[test]
    fn unrecognised_version_output_yields_none() {
        assert_eq!(parse_cmake_version("command not found"), None);
        assert_eq!(parse_nvcc_version(""), None);
    }

    #[test]
    fn parses_the_latest_visual_studio_instance() {
        let install = parse_vswhere(VSWHERE_JSON).expect("an instance");

        assert_eq!(install.display_name, "Visual Studio Community 2026");
        assert_eq!(install.version, "18.7.11911.148");
        assert_eq!(
            install.installation_path,
            PathBuf::from(r"C:\Program Files\Microsoft Visual Studio\18\Community")
        );
    }

    #[test]
    fn an_empty_vswhere_result_means_no_visual_studio() {
        assert_eq!(parse_vswhere("[]"), None);
        assert_eq!(parse_vswhere("not json"), None);
    }

    #[test]
    fn build_capability_follows_tool_requirements() {
        let toolchain = Toolchain {
            generators: Vec::new(),
            tools: vec![
                ToolStatus::found(ToolId::Cmake, "CMake", ToolRequirement::Required),
                ToolStatus::found(ToolId::Msvc, "MSVC", ToolRequirement::Required),
                ToolStatus::missing(
                    ToolId::Nvcc,
                    "nvcc",
                    ToolRequirement::RequiredForCuda,
                    "Install the CUDA Toolkit.",
                ),
                ToolStatus::missing(
                    ToolId::Ninja,
                    "Ninja",
                    ToolRequirement::Optional,
                    "Optional.",
                ),
            ],
        };

        assert!(toolchain.can_build_cpu());
        assert!(!toolchain.can_build_cuda());
    }
}
