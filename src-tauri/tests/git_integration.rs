//! End-to-end checks for the Git layer against a real repository created in a temp directory.
//!
//! These are the only tests that need an external tool. They skip themselves when `git` is
//! missing so the rest of the suite stays runnable on a bare machine, and they never touch the
//! user's global Git configuration: identity is passed per invocation with `-c`.

use std::path::Path;

use llama_control_lib::error::ErrorCode;
use llama_control_lib::git::{
    clone_repository, discover_default_branch, Git, RefCheckout, Repository,
};
use llama_control_lib::process::{CommandSpec, OutputLine};
use tokio::sync::mpsc;

const IDENTITY: [&str; 4] = [
    "-c",
    "user.name=Llama Control Tests",
    "-c",
    "user.email=tests@example.invalid",
];

async fn git_is_available() -> bool {
    Git::default().version().await.is_ok()
}

/// Runs git with a test identity, panicking on failure so setup problems are obvious.
async fn run_git(directory: &Path, args: &[&str]) -> String {
    let mut all_args: Vec<&str> = IDENTITY.to_vec();
    all_args.extend_from_slice(args);

    let spec = CommandSpec::new("git")
        .args(all_args)
        .current_dir(directory)
        .env("GIT_TERMINAL_PROMPT", "0");

    let output = llama_control_lib::process::capture(&spec)
        .await
        .expect("git should be runnable");

    assert!(
        output.succeeded(),
        "git {args:?} failed: {}",
        output.diagnostics()
    );

    output.stdout
}

async fn commit_file(directory: &Path, name: &str, contents: &str, message: &str) {
    std::fs::write(directory.join(name), contents).expect("write file");
    run_git(directory, &["add", name]).await;
    run_git(directory, &["commit", "-m", message]).await;
}

/// Creates an upstream repository with one commit and one tag.
async fn create_upstream(root: &Path) -> String {
    let upstream = root.join("upstream");
    std::fs::create_dir_all(&upstream).expect("create upstream");

    run_git(&upstream, &["init", "--quiet"]).await;
    commit_file(&upstream, "README.md", "llama.cpp\n", "initial commit").await;
    run_git(&upstream, &["tag", "b0001"]).await;

    run_git(&upstream, &["symbolic-ref", "--short", "HEAD"])
        .await
        .trim()
        .to_string()
}

fn discard_progress() -> mpsc::UnboundedSender<OutputLine> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    tokio::spawn(async move { while receiver.recv().await.is_some() {} });
    sender
}

#[tokio::test]
async fn clone_reports_status_refs_and_remotes() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    let default_branch = create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone should succeed");

    let repository = Repository::new(&git, &workspace);
    repository
        .ensure_is_repository()
        .await
        .expect("clone produced a repository");

    let status = repository.status().await.expect("status");
    assert_eq!(status.branch.as_deref(), Some(default_branch.as_str()));
    assert!(!status.detached);
    assert!(!status.is_dirty());
    assert_eq!(
        status.upstream.as_deref(),
        Some(format!("origin/{default_branch}").as_str())
    );

    let head = repository
        .head_commit()
        .await
        .expect("head")
        .expect("a commit");
    assert_eq!(head.subject, "initial commit");
    assert_eq!(head.short_commit.len(), 7);

    let refs = repository.list_refs().await.expect("refs");
    assert!(refs
        .iter()
        .any(|reference| reference.name == default_branch && reference.is_head));
    assert!(refs
        .iter()
        .any(|reference| reference.name == format!("origin/{default_branch}")));
    assert!(refs.iter().any(|reference| reference.name == "b0001"));
    // `origin/HEAD` is a symbolic pointer and must not be offered as a branch.
    assert!(!refs
        .iter()
        .any(|reference| reference.name.ends_with("/HEAD")));

    let remotes = repository.list_remotes().await.expect("remotes");
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].name, "origin");

    let discovered = repository
        .default_branch("origin")
        .await
        .expect("default branch lookup");
    assert_eq!(discovered.as_deref(), Some(default_branch.as_str()));
}

#[tokio::test]
async fn fetch_then_fast_forward_advances_the_branch() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    commit_file(&upstream, "ggml.c", "// second\n", "second commit").await;

    let repository = Repository::new(&git, &workspace);
    repository
        .fetch(None, discard_progress())
        .await
        .expect("fetch");

    let before = repository.status().await.expect("status before update");
    assert_eq!(before.behind, 1);
    assert_eq!(before.ahead, 0);

    let outcome = repository.fast_forward().await.expect("fast forward");
    assert!(outcome.changed);
    assert_ne!(outcome.previous_commit, outcome.current_commit);

    let after = repository.status().await.expect("status after update");
    assert_eq!(after.behind, 0);

    let head = repository
        .head_commit()
        .await
        .expect("head")
        .expect("commit");
    assert_eq!(head.subject, "second commit");

    // A second update with nothing new must be a no-op rather than an error.
    let repeat = repository
        .fast_forward()
        .await
        .expect("second fast forward");
    assert!(!repeat.changed);
}

#[tokio::test]
async fn a_dirty_worktree_blocks_updates_and_ref_switches() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    // An untracked file is normal in a llama.cpp checkout and must not block anything.
    std::fs::write(workspace.join("scratch.txt"), "ignored\n").expect("write untracked");
    let with_untracked = Repository::new(&git, &workspace)
        .status()
        .await
        .expect("status");
    assert_eq!(with_untracked.untracked, 1);
    assert!(!with_untracked.has_tracked_changes());
    Repository::new(&git, &workspace)
        .fast_forward()
        .await
        .expect("untracked files do not block a fast-forward");

    // Modifying a tracked file must block both update and switch.
    std::fs::write(workspace.join("README.md"), "edited\n").expect("edit tracked file");
    let repository = Repository::new(&git, &workspace);

    let status = repository.status().await.expect("status");
    assert_eq!(status.unstaged, 1);
    assert!(status.has_tracked_changes());

    let update_error = repository
        .fast_forward()
        .await
        .expect_err("a dirty worktree must block updating");
    assert_eq!(update_error.code, ErrorCode::DirtyWorktree);

    let switch_error = repository
        .switch_ref("b0001")
        .await
        .expect_err("a dirty worktree must block switching");
    assert_eq!(switch_error.code, ErrorCode::DirtyWorktree);

    // The edit must still be there: nothing was stashed or discarded.
    let contents = std::fs::read_to_string(workspace.join("README.md")).expect("read");
    assert_eq!(contents, "edited\n");
}

#[tokio::test]
async fn switching_to_a_tag_detaches_and_switching_back_reattaches() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    let default_branch = create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    let repository = Repository::new(&git, &workspace);

    let checkout = repository.switch_ref("b0001").await.expect("switch to tag");
    assert_eq!(checkout, RefCheckout::Detach);
    assert!(repository.status().await.expect("status").detached);

    let checkout = repository
        .switch_ref(&default_branch)
        .await
        .expect("switch back to the branch");
    assert_eq!(checkout, RefCheckout::LocalBranch);

    let status = repository.status().await.expect("status");
    assert!(!status.detached);
    assert_eq!(status.branch.as_deref(), Some(default_branch.as_str()));
}

#[tokio::test]
async fn a_remote_only_branch_becomes_a_local_tracking_branch() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    let default_branch = create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    // Publish an experimental branch upstream after the clone.
    run_git(&upstream, &["switch", "-c", "experimental/dflash"]).await;
    commit_file(
        &upstream,
        "dflash.c",
        "// wip\n",
        "dflash: work in progress",
    )
    .await;
    run_git(&upstream, &["switch", &default_branch]).await;

    let repository = Repository::new(&git, &workspace);
    repository
        .fetch(None, discard_progress())
        .await
        .expect("fetch");

    let checkout = repository
        .switch_ref("origin/experimental/dflash")
        .await
        .expect("switch to the remote branch");
    assert_eq!(checkout, RefCheckout::TrackRemoteBranch);

    let status = repository.status().await.expect("status");
    assert!(!status.detached);
    assert_eq!(status.branch.as_deref(), Some("experimental/dflash"));
    assert_eq!(
        status.upstream.as_deref(),
        Some("origin/experimental/dflash")
    );
}

#[tokio::test]
async fn remotes_can_be_added_fetched_and_removed() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    // A second upstream stands in for an experimental fork.
    let fork = temp.path().join("fork");
    std::fs::create_dir_all(&fork).expect("create fork");
    run_git(&fork, &["init", "--quiet"]).await;
    commit_file(&fork, "fork.c", "// fork\n", "fork: initial").await;
    run_git(&fork, &["switch", "-c", "experimental/backend"]).await;
    commit_file(&fork, "backend.c", "// backend\n", "fork: new backend").await;

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    let repository = Repository::new(&git, &workspace);
    repository
        .add_remote("experimental", &fork.display().to_string())
        .await
        .expect("add remote");

    let remotes = repository.list_remotes().await.expect("remotes");
    assert_eq!(remotes.len(), 2);
    assert!(remotes.iter().any(|remote| remote.name == "experimental"));

    repository
        .fetch(Some("experimental"), discard_progress())
        .await
        .expect("fetch the fork");

    let refs = repository.list_refs().await.expect("refs");
    assert!(refs
        .iter()
        .any(|reference| reference.name == "experimental/experimental/backend"));

    repository
        .remove_remote("experimental")
        .await
        .expect("remove remote");
    assert_eq!(repository.list_remotes().await.expect("remotes").len(), 1);
}

#[tokio::test]
async fn unknown_refs_and_invalid_urls_are_rejected_before_anything_changes() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    let default_branch = create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");
    let workspace = temp.path().join("work");

    let git = Git::default();
    clone_repository(
        &git,
        &upstream.display().to_string(),
        &workspace,
        None,
        discard_progress(),
    )
    .await
    .expect("clone");

    let repository = Repository::new(&git, &workspace);

    let error = repository
        .switch_ref("does-not-exist")
        .await
        .expect_err("unknown refs must fail");
    assert_eq!(error.code, ErrorCode::UnknownRef);

    // The checkout is untouched after the rejected switch.
    assert_eq!(
        repository.status().await.expect("status").branch.as_deref(),
        Some(default_branch.as_str())
    );

    let error = repository
        .add_remote("../escape", "https://example.com/x.git")
        .await
        .expect_err("unsafe remote names must fail");
    assert_eq!(error.code, ErrorCode::InvalidPath);

    let error = discover_default_branch(&git, "--upload-pack=calc")
        .await
        .expect_err("option-like URLs must fail");
    assert_eq!(error.code, ErrorCode::InvalidPath);
}

#[tokio::test]
async fn cloning_into_a_non_empty_folder_is_refused() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");

    let occupied = temp.path().join("occupied");
    std::fs::create_dir_all(&occupied).expect("create folder");
    std::fs::write(occupied.join("existing.txt"), "keep me").expect("write file");

    let error = clone_repository(
        &Git::default(),
        &upstream.display().to_string(),
        &occupied,
        None,
        discard_progress(),
    )
    .await
    .expect_err("cloning into a non-empty folder must fail");

    assert_eq!(error.code, ErrorCode::DirectoryNotEmpty);
    // The pre-existing file survived.
    assert!(occupied.join("existing.txt").exists());
}

#[tokio::test]
async fn discovers_the_default_branch_of_a_remote_without_cloning() {
    if !git_is_available().await {
        eprintln!("skipping: git is not installed");
        return;
    }

    let temp = tempfile::tempdir().expect("temp dir");
    let default_branch = create_upstream(temp.path()).await;
    let upstream = temp.path().join("upstream");

    let discovered = discover_default_branch(&Git::default(), &upstream.display().to_string())
        .await
        .expect("ls-remote should succeed");

    assert_eq!(discovered.as_deref(), Some(default_branch.as_str()));
}
