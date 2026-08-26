use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::config::paths::{require_existing_directory, validate_absolute_path};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::git::{self, GitRefKind, GitRemote, GitStatus, RefCheckout, Repository, UpdateOutcome};
use crate::process::OutputLine;
use crate::state::AppState;

use super::record::{
    suggest_directory_name, suggest_source_name, LlamaSource, SourceRefs, SourceStatus,
};
use super::registry::missing_source;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneRequest {
    pub repository: String,
    /// Parent folder for the clone. Defaults to the configured sources workspace.
    pub destination_parent: Option<PathBuf>,
    /// Folder name inside the parent. Defaults to the repository name.
    pub directory_name: Option<String>,
    pub branch: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchRefOutcome {
    pub source: LlamaSource,
    pub checkout: RefCheckout,
    pub detached: bool,
}

pub fn list(state: &AppState) -> Vec<LlamaSource> {
    state.sources.get().sources
}

pub fn require(state: &AppState, id: &str) -> AppResult<LlamaSource> {
    state
        .sources
        .get()
        .find(id)
        .cloned()
        .ok_or_else(|| missing_source(id))
}

/// Registers a llama.cpp checkout the user already has on disk.
pub async fn add_existing(
    state: &AppState,
    directory: &Path,
    name: Option<String>,
) -> AppResult<LlamaSource> {
    let directory = require_existing_directory(directory, "The source folder")?;
    let git = state.git();
    let repository = Repository::new(&git, &directory);

    repository.ensure_is_repository().await?;

    let remote = repository.primary_remote().await?;
    let status = repository.status().await?;
    let head = repository.head_commit().await?;

    let repository_url = remote
        .as_ref()
        .and_then(|remote| remote.fetch_url.clone())
        .unwrap_or_else(|| directory.display().to_string());

    let existing_names = state.sources.get().names();
    let source = LlamaSource {
        id: uuid::Uuid::new_v4().to_string(),
        name: name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| suggest_source_name(&repository_url, &existing_names)),
        repository: repository_url,
        directory,
        remote: remote
            .map(|remote| remote.name)
            .unwrap_or_else(|| "origin".into()),
        current_ref: describe_current_ref(&status),
        current_commit: head
            .as_ref()
            .map(|commit| commit.commit.clone())
            .unwrap_or_default(),
        added_at: now(),
        last_fetched_at: None,
    };

    let (_, inserted) = state
        .sources
        .update(|registry| registry.insert(source.clone()))?;
    inserted?;

    Ok(source)
}

/// Clones a repository and registers the result as a source.
pub async fn clone(
    state: &AppState,
    request: CloneRequest,
    progress: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<LlamaSource> {
    let repository_url = request.repository.trim().to_string();
    git::repository::validate_repository_url(&repository_url)?;

    let parent = match request.destination_parent {
        Some(parent) => validate_absolute_path(&parent, "The destination folder")?,
        None => state.settings.get().sources_directory(&state.paths),
    };

    let folder = request
        .directory_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| suggest_directory_name(&repository_url));
    validate_directory_name(&folder)?;

    let destination = parent.join(folder);

    std::fs::create_dir_all(&parent).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not create {}.", parent.display()),
        )
        .with_details(error.to_string())
    })?;

    git::clone_repository(
        &state.git(),
        &repository_url,
        &destination,
        request.branch.as_deref(),
        progress,
    )
    .await?;

    add_existing(state, &destination, request.name).await
}

/// Forgets a source, and optionally deletes its working copy.
pub fn remove(state: &AppState, id: &str, delete_directory: bool) -> AppResult<()> {
    let (_, removed) = state.sources.update(|registry| registry.remove(id))?;
    let removed = removed?;

    if delete_directory {
        std::fs::remove_dir_all(&removed.directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!(
                    "Removed the source, but could not delete {}.",
                    removed.directory.display()
                ),
            )
            .with_hint("Close anything using the folder and delete it manually.")
            .with_details(error.to_string())
        })?;
    }

    Ok(())
}

/// Reads live Git state and refreshes the cached ref/commit on the stored record.
pub async fn status(state: &AppState, id: &str) -> AppResult<SourceStatus> {
    let source = require(state, id)?;

    if !source.directory.is_dir() {
        return Ok(SourceStatus {
            directory_exists: false,
            git: GitStatus::default(),
            head: None,
            upstream_head: None,
            remotes: Vec::new(),
            update_available: false,
            source,
        });
    }

    let git = state.git();
    let repository = Repository::new(&git, &source.directory);
    repository.ensure_is_repository().await?;

    let git_status = repository.status().await?;
    let head = repository.head_commit().await?;
    let remotes = repository.list_remotes().await?;

    let upstream_head = match git_status.upstream.as_deref() {
        Some(upstream) => repository.commit(upstream).await?,
        None => None,
    };

    let current_ref = describe_current_ref(&git_status);
    let current_commit = head
        .as_ref()
        .map(|commit| commit.commit.clone())
        .unwrap_or_default();

    let (_, updated) = state.sources.update(|registry| {
        registry.update(id, |record| {
            record.current_ref = current_ref.clone();
            record.current_commit = current_commit.clone();
        })
    })?;
    let source = updated?;

    Ok(SourceStatus {
        source,
        directory_exists: true,
        update_available: git_status.behind > 0,
        git: git_status,
        head,
        upstream_head,
        remotes,
    })
}

pub async fn fetch(
    state: &AppState,
    id: &str,
    remote_name: Option<&str>,
    progress: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<()> {
    let source = require(state, id)?;
    let git = state.git();

    Repository::new(&git, &source.directory)
        .fetch(remote_name, progress)
        .await?;

    let fetched_at = now();
    let (_, updated) = state.sources.update(|registry| {
        registry.update(id, |record| {
            record.last_fetched_at = Some(fetched_at.clone());
        })
    })?;
    updated?;

    Ok(())
}

pub async fn update(state: &AppState, id: &str) -> AppResult<UpdateOutcome> {
    let source = require(state, id)?;
    let git = state.git();

    Repository::new(&git, &source.directory)
        .fast_forward()
        .await
}

pub async fn refs(state: &AppState, id: &str) -> AppResult<SourceRefs> {
    let source = require(state, id)?;
    let git = state.git();

    let all_refs = Repository::new(&git, &source.directory).list_refs().await?;

    let mut local_branches = Vec::new();
    let mut remote_branches = Vec::new();
    let mut tags = Vec::new();

    for reference in all_refs {
        match reference.kind {
            GitRefKind::LocalBranch => local_branches.push(reference),
            GitRefKind::RemoteBranch => remote_branches.push(reference),
            GitRefKind::Tag => tags.push(reference),
        }
    }

    local_branches.sort_by(|left, right| left.name.cmp(&right.name));
    remote_branches.sort_by(|left, right| left.name.cmp(&right.name));
    // Newest tags first: llama.cpp publishes build tags continuously.
    tags.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    Ok(SourceRefs {
        local_branches,
        remote_branches,
        tags,
    })
}

pub async fn switch_ref(state: &AppState, id: &str, target: &str) -> AppResult<SwitchRefOutcome> {
    let source = require(state, id)?;
    let git = state.git();
    let repository = Repository::new(&git, &source.directory);

    let checkout = repository.switch_ref(target).await?;
    let git_status = repository.status().await?;
    let head = repository.head_commit().await?;

    let current_ref = describe_current_ref(&git_status);
    let current_commit = head.map(|commit| commit.commit).unwrap_or_default();

    let (_, updated) = state.sources.update(|registry| {
        registry.update(id, |record| {
            record.current_ref = current_ref.clone();
            record.current_commit = current_commit.clone();
        })
    })?;

    Ok(SwitchRefOutcome {
        source: updated?,
        checkout,
        detached: git_status.detached,
    })
}

pub async fn remotes(state: &AppState, id: &str) -> AppResult<Vec<GitRemote>> {
    let source = require(state, id)?;
    let git = state.git();

    Repository::new(&git, &source.directory)
        .list_remotes()
        .await
}

pub async fn add_remote(
    state: &AppState,
    id: &str,
    name: &str,
    url: &str,
) -> AppResult<Vec<GitRemote>> {
    let source = require(state, id)?;
    let git = state.git();
    let repository = Repository::new(&git, &source.directory);

    repository.add_remote(name, url).await?;
    repository.list_remotes().await
}

pub async fn remove_remote(state: &AppState, id: &str, name: &str) -> AppResult<Vec<GitRemote>> {
    let source = require(state, id)?;
    let git = state.git();
    let repository = Repository::new(&git, &source.directory);

    repository.remove_remote(name).await?;
    repository.list_remotes().await
}

/// How the current checkout should be labelled: the branch name, or the commit when detached.
pub fn describe_current_ref(status: &GitStatus) -> String {
    if let Some(branch) = &status.branch {
        return branch.clone();
    }

    status
        .short_commit()
        .map(|commit| format!("detached @ {commit}"))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Clone folder names must not escape the chosen parent directory.
fn validate_directory_name(name: &str) -> AppResult<()> {
    let trimmed = name.trim();

    let is_valid = !trimmed.is_empty()
        && trimmed != "."
        && trimmed != ".."
        && !trimmed.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);

    if is_valid {
        return Ok(());
    }

    Err(
        AppError::invalid_path("The folder name must be a single name without path separators.")
            .with_details(name.to_string()),
    )
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_an_attached_branch_by_name() {
        let status =
            crate::git::status::parse_status("# branch.oid a1b2c3d4e5f6\n# branch.head master\n");

        assert_eq!(describe_current_ref(&status), "master");
    }

    #[test]
    fn labels_a_detached_head_with_its_commit() {
        let status = crate::git::status::parse_status(
            "# branch.oid 92fac1a0b1c2d3e4\n# branch.head (detached)\n",
        );

        assert_eq!(describe_current_ref(&status), "detached @ 92fac1a");
    }

    #[test]
    fn rejects_folder_names_containing_separators() {
        for name in ["", ".", "..", "a/b", r"a\b", "bad:name", "with*star"] {
            assert!(
                validate_directory_name(name).is_err(),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_ordinary_folder_names() {
        for name in ["llama.cpp", "llama-cpp-fork", "llama_cpp2"] {
            assert!(
                validate_directory_name(name).is_ok(),
                "{name} should be valid"
            );
        }
    }
}
