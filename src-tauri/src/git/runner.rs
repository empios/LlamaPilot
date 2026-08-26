use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::process::{self, CommandOutput, CommandSpec, OutputLine};

/// Thin wrapper that turns Git invocations into structured results.
///
/// It knows nothing about llama.cpp; higher layers compose it into safe workflows.
#[derive(Debug, Clone)]
pub struct Git {
    program: PathBuf,
}

impl Default for Git {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Git {
    pub fn new(explicit_program: Option<PathBuf>) -> Self {
        Self {
            program: explicit_program.unwrap_or_else(|| PathBuf::from("git")),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    fn spec<I, S>(&self, repository: Option<&Path>, args: I) -> CommandSpec
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut spec = CommandSpec::new(&self.program)
            // Never block on an interactive credential or passphrase prompt inside a GUI.
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(args);

        if let Some(repository) = repository {
            spec = spec.current_dir(repository);
        }

        spec
    }

    /// Runs Git and returns the output regardless of exit status.
    pub async fn try_run<I, S>(
        &self,
        repository: Option<&Path>,
        args: I,
    ) -> AppResult<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let spec = self.spec(repository, args);
        tracing::debug!(command = %spec.to_display_string(), "running git");

        process::capture(&spec)
            .await
            .map_err(|error| match error.code {
                ErrorCode::ProcessNotFound => git_not_found(&self.program),
                _ => error,
            })
    }

    /// Runs Git and turns a non-zero exit into an error carrying the raw diagnostics.
    pub async fn run<I, S>(&self, repository: Option<&Path>, args: I) -> AppResult<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.try_run(repository, args).await?;
        if output.succeeded() {
            return Ok(output);
        }

        Err(classify_failure(&output))
    }

    /// Runs Git while forwarding progress output line by line.
    pub async fn run_streaming<I, S>(
        &self,
        repository: Option<&Path>,
        args: I,
        sink: mpsc::UnboundedSender<OutputLine>,
    ) -> AppResult<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let spec = self.spec(repository, args);
        tracing::debug!(command = %spec.to_display_string(), "streaming git");

        let output = process::stream(&spec, sink)
            .await
            .map_err(|error| match error.code {
                ErrorCode::ProcessNotFound => git_not_found(&self.program),
                _ => error,
            })?;

        if output.succeeded() {
            return Ok(output);
        }

        Err(classify_failure(&output))
    }

    pub async fn version(&self) -> AppResult<String> {
        let output = self.run(None, ["--version"]).await?;
        Ok(output.stdout.trim().to_string())
    }
}

fn git_not_found(program: &Path) -> AppError {
    AppError::new(
        ErrorCode::GitNotFound,
        format!("Git was not found at \"{}\".", program.display()),
    )
    .with_hint("Install Git for Windows, or set an explicit git path in Settings.")
}

/// Maps well-known Git failure text onto specific error codes.
///
/// Anything unrecognised stays a generic `gitFailed` with the raw output attached, so the UI can
/// still show exactly what Git said.
fn classify_failure(output: &CommandOutput) -> AppError {
    let diagnostics = output.diagnostics();
    let lowered = diagnostics.to_lowercase();

    let error = if lowered.contains("not a git repository") {
        AppError::new(
            ErrorCode::NotARepository,
            "That folder is not a Git repository.",
        )
        .with_hint(
            "Choose the folder containing llama.cpp's .git directory, or clone a new source.",
        )
    } else if lowered.contains("not possible to fast-forward")
        || lowered.contains("divergent branches")
        || lowered.contains("refusing to merge unrelated histories")
    {
        AppError::new(
            ErrorCode::NotFastForward,
            "The branch cannot be fast-forwarded.",
        )
        .with_hint(
            "Local commits diverge from the upstream branch. Review the changes before updating.",
        )
    } else if lowered.contains("would be overwritten")
        || lowered.contains("local changes")
        || lowered.contains("please commit your changes or stash them")
    {
        AppError::new(
            ErrorCode::DirtyWorktree,
            "The working tree has local changes that would be overwritten.",
        )
        .with_hint("Commit, stash, or discard the changes first.")
    } else if lowered.contains("unknown revision")
        || lowered.contains("did not match any file(s) known to git")
        || lowered.contains("invalid reference")
        || lowered.contains("pathspec")
    {
        AppError::new(ErrorCode::UnknownRef, "That reference does not exist.").with_hint(
            "Fetch first, then pick a branch, tag, or commit that the repository knows about.",
        )
    } else if lowered.contains("remote") && lowered.contains("already exists") {
        AppError::new(
            ErrorCode::RemoteExists,
            "A remote with that name already exists.",
        )
    } else if lowered.contains("no such remote") {
        AppError::new(ErrorCode::RemoteNotFound, "That remote does not exist.")
    } else if lowered.contains("already exists and is not an empty directory") {
        AppError::new(
            ErrorCode::DirectoryNotEmpty,
            "The destination folder already exists and is not empty.",
        )
        .with_hint("Pick an empty folder, or add the existing checkout as a source instead.")
    } else {
        AppError::new(
            ErrorCode::GitFailed,
            format!(
                "Git exited with code {}.",
                output
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ),
        )
    };

    error.with_details(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(stderr: &str) -> CommandOutput {
        CommandOutput {
            exit_code: Some(128),
            stdout: String::new(),
            stderr: stderr.to_string(),
            duration_ms: 3,
        }
    }

    #[test]
    fn recognises_a_missing_repository() {
        let error = classify_failure(&failure(
            "fatal: not a git repository (or any of the parent directories): .git",
        ));

        assert_eq!(error.code, ErrorCode::NotARepository);
        assert!(error.hint.is_some());
        assert!(error.details.is_some());
    }

    #[test]
    fn recognises_a_non_fast_forward_update() {
        let error = classify_failure(&failure("fatal: Not possible to fast-forward, aborting."));
        assert_eq!(error.code, ErrorCode::NotFastForward);
    }

    #[test]
    fn recognises_a_dirty_worktree() {
        let error = classify_failure(&failure(
            "error: Your local changes to the following files would be overwritten by checkout:\n\tggml/src/ggml.c",
        ));
        assert_eq!(error.code, ErrorCode::DirtyWorktree);
    }

    #[test]
    fn recognises_an_unknown_ref() {
        let error = classify_failure(&failure("fatal: invalid reference: does-not-exist"));
        assert_eq!(error.code, ErrorCode::UnknownRef);
    }

    #[test]
    fn recognises_a_non_empty_clone_destination() {
        let error = classify_failure(&failure(
            "fatal: destination path 'llama.cpp' already exists and is not an empty directory.",
        ));
        assert_eq!(error.code, ErrorCode::DirectoryNotEmpty);
    }

    #[test]
    fn unrecognised_failures_keep_the_raw_output() {
        let error = classify_failure(&failure("fatal: something entirely new"));

        assert_eq!(error.code, ErrorCode::GitFailed);
        assert_eq!(
            error.details.as_deref(),
            Some("fatal: something entirely new")
        );
    }
}
