use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::git::{CommitInfo, GitRemote, GitStatus};

/// A llama.cpp Git working copy known to the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaSource {
    pub id: String,
    pub name: String,
    pub repository: String,
    pub directory: PathBuf,
    pub remote: String,
    pub current_ref: String,
    pub current_commit: String,
    pub added_at: String,
    pub last_fetched_at: Option<String>,
}

/// Live Git state for a source, recomputed on demand rather than cached in the registry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub source: LlamaSource,
    pub directory_exists: bool,
    pub git: GitStatus,
    pub head: Option<CommitInfo>,
    pub upstream_head: Option<CommitInfo>,
    pub remotes: Vec<GitRemote>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRefs {
    pub local_branches: Vec<crate::git::GitRef>,
    pub remote_branches: Vec<crate::git::GitRef>,
    pub tags: Vec<crate::git::GitRef>,
}

/// Turns a repository URL into a short, unique, human-friendly source name.
pub fn suggest_source_name(repository: &str, existing: &[String]) -> String {
    let described = crate::git::remote::describe_repository(repository);
    let base = described
        .split('/')
        .next_back()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("llama.cpp")
        .to_string();

    if !existing.iter().any(|name| name == &base) {
        return base;
    }

    (2..)
        .map(|suffix| format!("{base} ({suffix})"))
        .find(|candidate| !existing.iter().any(|name| name == candidate))
        .unwrap_or(base)
}

/// Folder name used when cloning into the managed workspace.
pub fn suggest_directory_name(repository: &str) -> String {
    let described = crate::git::remote::describe_repository(repository);
    let candidate = described
        .split('/')
        .next_back()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("llama.cpp");

    let sanitized: String = candidate
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();

    if sanitized.trim_matches(['-', '.']).is_empty() {
        "llama.cpp".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_the_repository_name() {
        assert_eq!(
            suggest_source_name("https://github.com/ggml-org/llama.cpp", &[]),
            "llama.cpp"
        );
    }

    #[test]
    fn disambiguates_duplicate_names() {
        let existing = vec!["llama.cpp".to_string(), "llama.cpp (2)".to_string()];

        assert_eq!(
            suggest_source_name("https://github.com/some-user/llama.cpp", &existing),
            "llama.cpp (3)"
        );
    }

    #[test]
    fn directory_names_drop_characters_that_are_unsafe_on_windows() {
        assert_eq!(
            suggest_directory_name("https://example.com/team/llama:cpp"),
            "llama-cpp"
        );
        assert_eq!(
            suggest_directory_name("https://github.com/ggml-org/llama.cpp.git"),
            "llama.cpp"
        );
    }
}
