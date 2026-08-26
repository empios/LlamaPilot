use std::path::Path;

use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::process::OutputLine;

use super::refs::{self, CommitInfo, GitRef, RefCheckout, FOR_EACH_REF_FORMAT, LOG_COMMIT_FORMAT};
use super::remote::{self, GitRemote};
use super::runner::Git;
use super::status::{parse_status, GitStatus};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    pub previous_commit: Option<String>,
    pub current_commit: Option<String>,
    pub changed: bool,
}

/// Safe, read-only and fast-forward-only operations against a Git working copy.
///
/// Nothing here can discard user work: there is no reset, no force checkout, and no implicit
/// stash. Operations that would need one return a specific error instead.
pub struct Repository<'a> {
    git: &'a Git,
    directory: &'a Path,
}

impl<'a> Repository<'a> {
    pub fn new(git: &'a Git, directory: &'a Path) -> Self {
        Self { git, directory }
    }

    pub async fn ensure_is_repository(&self) -> AppResult<()> {
        let output = self
            .git
            .try_run(Some(self.directory), ["rev-parse", "--git-dir"])
            .await?;

        if output.succeeded() {
            return Ok(());
        }

        Err(AppError::new(
            ErrorCode::NotARepository,
            format!("{} is not a Git repository.", self.directory.display()),
        )
        .with_hint("Choose a folder containing a llama.cpp checkout, or clone a new source.")
        .with_details(output.diagnostics()))
    }

    pub async fn status(&self) -> AppResult<GitStatus> {
        let output = self
            .git
            .run(
                Some(self.directory),
                [
                    "status",
                    "--porcelain=v2",
                    "--branch",
                    "--untracked-files=normal",
                ],
            )
            .await?;

        Ok(parse_status(&output.stdout))
    }

    pub async fn head_commit(&self) -> AppResult<Option<CommitInfo>> {
        self.commit("HEAD").await
    }

    pub async fn commit(&self, reference: &str) -> AppResult<Option<CommitInfo>> {
        let output = self
            .git
            .try_run(
                Some(self.directory),
                [
                    "log",
                    "-1",
                    &format!("--format={LOG_COMMIT_FORMAT}"),
                    reference,
                ],
            )
            .await?;

        if !output.succeeded() {
            return Ok(None);
        }

        Ok(refs::parse_commit(&output.stdout))
    }

    pub async fn list_refs(&self) -> AppResult<Vec<GitRef>> {
        let output = self
            .git
            .run(
                Some(self.directory),
                [
                    "for-each-ref",
                    &format!("--format={FOR_EACH_REF_FORMAT}"),
                    "refs/heads",
                    "refs/remotes",
                    "refs/tags",
                ],
            )
            .await?;

        Ok(refs::parse_refs(&output.stdout))
    }

    pub async fn list_remotes(&self) -> AppResult<Vec<GitRemote>> {
        let output = self.git.run(Some(self.directory), ["remote", "-v"]).await?;
        Ok(remote::parse_remotes(&output.stdout))
    }

    pub async fn add_remote(&self, name: &str, url: &str) -> AppResult<()> {
        validate_remote_name(name)?;
        validate_repository_url(url)?;
        self.git
            .run(Some(self.directory), ["remote", "add", name, url])
            .await
            .map(|_| ())
    }

    pub async fn remove_remote(&self, name: &str) -> AppResult<()> {
        validate_remote_name(name)?;
        self.git
            .run(Some(self.directory), ["remote", "remove", name])
            .await
            .map(|_| ())
    }

    /// Fetches everything, or a single remote when one is named.
    pub async fn fetch(
        &self,
        remote_name: Option<&str>,
        sink: mpsc::UnboundedSender<OutputLine>,
    ) -> AppResult<()> {
        let mut args = vec!["fetch".to_string()];
        match remote_name {
            Some(name) => {
                validate_remote_name(name)?;
                args.push(name.to_string());
            }
            None => args.push("--all".to_string()),
        }
        args.extend(["--tags".into(), "--prune".into(), "--progress".into()]);

        self.git
            .run_streaming(Some(self.directory), args, sink)
            .await
            .map(|_| ())
    }

    /// Fast-forwards the current branch onto its upstream.
    ///
    /// Refuses on a dirty worktree, on a detached HEAD, and when no upstream is configured. It
    /// never falls back to a merge commit or a reset.
    pub async fn fast_forward(&self) -> AppResult<UpdateOutcome> {
        let status = self.status().await?;

        if status.has_tracked_changes() {
            return Err(AppError::new(
                ErrorCode::DirtyWorktree,
                "The repository has uncommitted changes.",
            )
            .with_hint(
                "Review them first. Nothing is stashed or discarded without an explicit request.",
            ));
        }

        if status.detached {
            return Err(AppError::new(
                ErrorCode::DetachedHead,
                "HEAD is detached, so there is no branch to update.",
            )
            .with_hint("Switch to a branch before updating."));
        }

        let Some(upstream) = status.upstream.clone() else {
            return Err(AppError::new(
                ErrorCode::NoUpstream,
                "The current branch does not track an upstream branch.",
            )
            .with_hint("Switch to a tracked branch, or set an upstream with Git."));
        };

        let previous_commit = status.commit.clone();

        self.git
            .run(Some(self.directory), ["merge", "--ff-only", &upstream])
            .await?;

        let current_commit = self.status().await?.commit;

        Ok(UpdateOutcome {
            changed: previous_commit != current_commit,
            previous_commit,
            current_commit,
        })
    }

    /// Checks out a branch, tag, or commit.
    ///
    /// Tags and commits intentionally produce a detached HEAD; the caller surfaces that state.
    pub async fn switch_ref(&self, target: &str) -> AppResult<RefCheckout> {
        let target = target.trim();
        if target.is_empty() {
            return Err(AppError::new(
                ErrorCode::UnknownRef,
                "Enter a branch, tag, or commit to switch to.",
            ));
        }

        let status = self.status().await?;
        if status.has_tracked_changes() {
            return Err(AppError::new(
                ErrorCode::DirtyWorktree,
                "The repository has uncommitted changes.",
            )
            .with_hint("Commit or stash them before switching refs."));
        }

        let known_refs = self.list_refs().await?;
        let plan = refs::plan_checkout(target, &known_refs);

        match plan {
            RefCheckout::LocalBranch => {
                self.git
                    .run(Some(self.directory), ["switch", "--", target])
                    .await?;
            }
            RefCheckout::TrackRemoteBranch => {
                let local = refs::local_branch_for_remote(target).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::UnknownRef,
                        format!("Could not derive a local branch name from \"{target}\"."),
                    )
                })?;
                self.git
                    .run(
                        Some(self.directory),
                        ["switch", "--track", "-c", &local, target],
                    )
                    .await?;
            }
            RefCheckout::Detach => {
                self.resolve_ref(target).await?;
                self.git
                    .run(Some(self.directory), ["switch", "--detach", target])
                    .await?;
            }
        }

        Ok(plan)
    }

    /// Verifies that a ref resolves to a commit without changing anything.
    pub async fn resolve_ref(&self, reference: &str) -> AppResult<String> {
        let output = self
            .git
            .try_run(
                Some(self.directory),
                [
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("{reference}^{{commit}}"),
                ],
            )
            .await?;

        let resolved = output.stdout.trim().to_string();
        if !output.succeeded() || resolved.is_empty() {
            return Err(AppError::new(
                ErrorCode::UnknownRef,
                format!("\"{reference}\" is not a known branch, tag, or commit."),
            )
            .with_hint("Fetch the remote first if you expect a newly published ref.")
            .with_details(output.diagnostics()));
        }

        Ok(resolved)
    }

    /// Reads the remote's default branch from the locally cached `<remote>/HEAD` pointer.
    pub async fn default_branch(&self, remote_name: &str) -> AppResult<Option<String>> {
        let output = self
            .git
            .try_run(
                Some(self.directory),
                [
                    "symbolic-ref",
                    "--short",
                    &format!("refs/remotes/{remote_name}/HEAD"),
                ],
            )
            .await?;

        if !output.succeeded() {
            return Ok(None);
        }

        Ok(refs::parse_remote_head(&output.stdout, remote_name))
    }

    pub async fn primary_remote(&self) -> AppResult<Option<GitRemote>> {
        let remotes = self.list_remotes().await?;
        Ok(remotes
            .iter()
            .find(|candidate| candidate.name == "origin")
            .or_else(|| remotes.first())
            .cloned())
    }
}

/// Asks a remote URL for its default branch without cloning it.
pub async fn discover_default_branch(git: &Git, url: &str) -> AppResult<Option<String>> {
    validate_repository_url(url)?;

    let output = git
        .run(None, ["ls-remote", "--symref", url, "HEAD"])
        .await?;

    Ok(refs::parse_symref_default_branch(&output.stdout))
}

/// Clones a repository, streaming Git's progress output.
pub async fn clone_repository(
    git: &Git,
    url: &str,
    destination: &Path,
    branch: Option<&str>,
    sink: mpsc::UnboundedSender<OutputLine>,
) -> AppResult<()> {
    validate_repository_url(url)?;

    if destination.exists() {
        let is_empty = std::fs::read_dir(destination)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);

        if !is_empty {
            return Err(AppError::new(
                ErrorCode::DirectoryNotEmpty,
                "The destination folder already exists and is not empty.",
            )
            .with_hint("Pick an empty folder, or add the existing checkout as a source instead.")
            .with_details(destination.display().to_string()));
        }
    }

    let mut args: Vec<String> = vec!["clone".into(), "--progress".into()];
    if let Some(branch) = branch.map(str::trim).filter(|branch| !branch.is_empty()) {
        args.push("--branch".into());
        args.push(branch.to_string());
    }
    args.push(url.to_string());
    args.push(destination.to_string_lossy().into_owned());

    git.run_streaming(None, args, sink).await.map(|_| ())
}

/// Remote names become path components inside `.git`, so they are restricted to a safe subset.
pub fn validate_remote_name(name: &str) -> AppResult<()> {
    let trimmed = name.trim();

    let is_valid = !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('-');

    if is_valid {
        return Ok(());
    }

    Err(AppError::new(
        ErrorCode::InvalidPath,
        "Remote names may only contain letters, digits, dots, dashes, and underscores.",
    )
    .with_details(name.to_string()))
}

/// Accepts the URL forms Git itself supports for network and local clones, and rejects anything
/// that could be interpreted as a Git option.
pub fn validate_repository_url(url: &str) -> AppResult<()> {
    let trimmed = url.trim();

    if trimmed.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            "A repository URL is required.",
        ));
    }

    if trimmed.starts_with('-') {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            "A repository URL cannot start with \"-\".",
        )
        .with_details(trimmed.to_string()));
    }

    let is_supported = ["https://", "http://", "ssh://", "git://", "file://"]
        .iter()
        .any(|scheme| trimmed.starts_with(scheme))
        || trimmed.contains('@') && trimmed.contains(':')
        || Path::new(trimmed).is_absolute();

    if is_supported {
        return Ok(());
    }

    Err(AppError::new(
        ErrorCode::InvalidPath,
        "That does not look like a Git repository URL.",
    )
    .with_hint("Use an https://, ssh://, git://, scp-style, or absolute local path.")
    .with_details(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_url_forms_git_supports() {
        for url in [
            "https://github.com/ggml-org/llama.cpp",
            "http://internal.example/llama.cpp.git",
            "ssh://git@github.com/ggml-org/llama.cpp.git",
            "git://example.com/llama.cpp",
            "git@github.com:some-user/llama.cpp.git",
        ] {
            assert!(
                validate_repository_url(url).is_ok(),
                "{url} should be valid"
            );
        }
    }

    #[test]
    fn rejects_option_lookalikes_and_bare_words() {
        for url in ["--upload-pack=calc", "-c", "just-a-word", "   "] {
            let error = validate_repository_url(url).expect_err("must reject");
            assert_eq!(error.code, ErrorCode::InvalidPath);
        }
    }

    #[test]
    fn accepts_conventional_remote_names() {
        for name in ["origin", "experimental", "fork-2", "my_remote", "gg.dev"] {
            assert!(validate_remote_name(name).is_ok(), "{name} should be valid");
        }
    }

    #[test]
    fn rejects_remote_names_that_could_escape_the_git_directory() {
        for name in ["", "../evil", "with space", "-flag", ".hidden", "a/b"] {
            assert!(
                validate_remote_name(name).is_err(),
                "{name} should be rejected"
            );
        }
    }
}
