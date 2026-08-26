use serde::{Deserialize, Serialize};

/// Field separator requested from `git for-each-ref` via `%1f`.
///
/// A control character is used so branch names, tag names and commit subjects can never collide
/// with the separator.
pub const FIELD_SEPARATOR: char = '\u{1f}';

pub const FOR_EACH_REF_FORMAT: &str = concat!(
    "%(refname)%1f",
    "%(objectname)%1f",
    "%(*objectname)%1f",
    "%(creatordate:iso-strict)%1f",
    "%(upstream:short)%1f",
    "%(HEAD)"
);

pub const LOG_COMMIT_FORMAT: &str = "%H%x1f%h%x1f%cI%x1f%an%x1f%s";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitRefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRef {
    pub name: String,
    pub full_name: String,
    pub kind: GitRefKind,
    pub commit: String,
    pub short_commit: String,
    pub created_at: Option<String>,
    pub remote: Option<String>,
    pub upstream: Option<String>,
    pub is_head: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub commit: String,
    pub short_commit: String,
    pub committed_at: String,
    pub author: String,
    pub subject: String,
}

/// How a user-supplied ref should be checked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefCheckout {
    /// A local branch: stays attached to HEAD.
    LocalBranch,
    /// A remote-only branch: creates a local tracking branch.
    TrackRemoteBranch,
    /// A tag or commit: HEAD becomes detached, intentionally.
    Detach,
}

pub fn parse_refs(output: &str) -> Vec<GitRef> {
    output.lines().filter_map(parse_ref_line).collect()
}

fn parse_ref_line(line: &str) -> Option<GitRef> {
    if line.trim().is_empty() {
        return None;
    }

    let fields: Vec<&str> = line.split(FIELD_SEPARATOR).collect();
    let full_name = (*fields.first()?).to_string();
    let object_name = fields.get(1).copied().unwrap_or_default();
    let dereferenced = fields.get(2).copied().unwrap_or_default();
    let created_at = fields.get(3).copied().unwrap_or_default();
    let upstream = fields.get(4).copied().unwrap_or_default();
    let head_marker = fields.get(5).copied().unwrap_or_default();

    let (kind, name) = classify(&full_name)?;

    // `origin/HEAD` is a symbolic pointer at the remote default branch, not a branch itself.
    if kind == GitRefKind::RemoteBranch && name.ends_with("/HEAD") {
        return None;
    }

    // For annotated tags `objectname` is the tag object; the commit is the dereferenced object.
    let commit = if dereferenced.is_empty() {
        object_name
    } else {
        dereferenced
    };

    let remote = match kind {
        GitRefKind::RemoteBranch => name.split_once('/').map(|(remote, _)| remote.to_string()),
        GitRefKind::LocalBranch | GitRefKind::Tag => None,
    };

    Some(GitRef {
        short_commit: commit.chars().take(7).collect(),
        commit: commit.to_string(),
        created_at: non_empty(created_at),
        upstream: non_empty(upstream),
        is_head: head_marker.trim() == "*",
        remote,
        kind,
        name,
        full_name,
    })
}

fn classify(full_name: &str) -> Option<(GitRefKind, String)> {
    if let Some(name) = full_name.strip_prefix("refs/heads/") {
        return Some((GitRefKind::LocalBranch, name.to_string()));
    }
    if let Some(name) = full_name.strip_prefix("refs/remotes/") {
        return Some((GitRefKind::RemoteBranch, name.to_string()));
    }
    if let Some(name) = full_name.strip_prefix("refs/tags/") {
        return Some((GitRefKind::Tag, name.to_string()));
    }
    None
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn parse_commit(output: &str) -> Option<CommitInfo> {
    let line = output.lines().find(|line| !line.trim().is_empty())?;
    let fields: Vec<&str> = line.split(FIELD_SEPARATOR).collect();

    Some(CommitInfo {
        commit: (*fields.first()?).to_string(),
        short_commit: fields.get(1).copied().unwrap_or_default().to_string(),
        committed_at: fields.get(2).copied().unwrap_or_default().to_string(),
        author: fields.get(3).copied().unwrap_or_default().to_string(),
        subject: fields.get(4).copied().unwrap_or_default().to_string(),
    })
}

/// Reads the default branch out of `git ls-remote --symref <url> HEAD`.
pub fn parse_symref_default_branch(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.strip_prefix("ref: ")?;
        let reference = rest.split_whitespace().next()?;
        reference
            .strip_prefix("refs/heads/")
            .map(|branch| branch.to_string())
    })
}

/// Reads the default branch out of `git symbolic-ref --short refs/remotes/<remote>/HEAD`.
pub fn parse_remote_head(output: &str, remote: &str) -> Option<String> {
    let value = output.trim();
    if value.is_empty() {
        return None;
    }
    value
        .strip_prefix(&format!("{remote}/"))
        .map(|branch| branch.to_string())
}

/// Decides how a chosen ref must be checked out, given what the repository already contains.
pub fn plan_checkout(target: &str, refs: &[GitRef]) -> RefCheckout {
    let has_local_branch = refs
        .iter()
        .any(|reference| reference.kind == GitRefKind::LocalBranch && reference.name == target);
    if has_local_branch {
        return RefCheckout::LocalBranch;
    }

    let is_remote_branch = refs
        .iter()
        .any(|reference| reference.kind == GitRefKind::RemoteBranch && reference.name == target);
    if is_remote_branch {
        return RefCheckout::TrackRemoteBranch;
    }

    RefCheckout::Detach
}

/// Local branch name implied by tracking `origin/experimental/dflash` → `experimental/dflash`.
pub fn local_branch_for_remote(remote_branch: &str) -> Option<String> {
    remote_branch
        .split_once('/')
        .map(|(_, branch)| branch.to_string())
        .filter(|branch| !branch.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(fields: &[&str]) -> String {
        fields.join(&FIELD_SEPARATOR.to_string())
    }

    #[test]
    fn parses_local_branches_with_upstream_and_head_marker() {
        let output = line(&[
            "refs/heads/master",
            "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
            "",
            "2026-08-24T10:11:12+02:00",
            "origin/master",
            "*",
        ]);

        let refs = parse_refs(&output);
        let reference = refs.first().expect("one ref");

        assert_eq!(reference.kind, GitRefKind::LocalBranch);
        assert_eq!(reference.name, "master");
        assert_eq!(reference.short_commit, "a1b2c3d");
        assert_eq!(reference.upstream.as_deref(), Some("origin/master"));
        assert!(reference.is_head);
    }

    #[test]
    fn parses_remote_branches_and_extracts_the_remote_name() {
        let output = line(&[
            "refs/remotes/experimental/dflash-wip",
            "92fac1a0b1c2d3e4f50617283940a1b2c3d4e5f6",
            "",
            "2026-08-20T08:00:00+02:00",
            "",
            "",
        ]);

        let reference = parse_refs(&output).into_iter().next().expect("one ref");

        assert_eq!(reference.kind, GitRefKind::RemoteBranch);
        assert_eq!(reference.name, "experimental/dflash-wip");
        assert_eq!(reference.remote.as_deref(), Some("experimental"));
        assert!(!reference.is_head);
    }

    #[test]
    fn annotated_tags_resolve_to_the_dereferenced_commit() {
        let output = line(&[
            "refs/tags/b7104",
            "ffffffffffffffffffffffffffffffffffffffff",
            "812abc9d0e1f2a3b4c5d6e7f8091a2b3c4d5e6f7",
            "2026-08-20T08:00:00+02:00",
            "",
            "",
        ]);

        let reference = parse_refs(&output).into_iter().next().expect("one ref");

        assert_eq!(reference.kind, GitRefKind::Tag);
        assert_eq!(reference.commit, "812abc9d0e1f2a3b4c5d6e7f8091a2b3c4d5e6f7");
        assert_eq!(reference.short_commit, "812abc9");
    }

    #[test]
    fn lightweight_tags_use_the_object_name_directly() {
        let output = line(&[
            "refs/tags/b7105",
            "0123456789abcdef0123456789abcdef01234567",
            "",
            "2026-08-21T08:00:00+02:00",
            "",
            "",
        ]);

        let reference = parse_refs(&output).into_iter().next().expect("one ref");
        assert_eq!(reference.commit, "0123456789abcdef0123456789abcdef01234567");
    }

    #[test]
    fn the_remote_head_pointer_is_not_listed_as_a_branch() {
        let output = [
            line(&["refs/remotes/origin/HEAD", "aaa", "", "", "", ""]),
            line(&["refs/remotes/origin/master", "bbb", "", "", "", ""]),
        ]
        .join("\n");

        let refs = parse_refs(&output);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "origin/master");
    }

    #[test]
    fn unknown_ref_namespaces_are_ignored() {
        let output = line(&["refs/stash", "aaa", "", "", "", ""]);
        assert!(parse_refs(&output).is_empty());
    }

    #[test]
    fn parses_commit_metadata() {
        let output = format!(
            "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678{sep}a1b2c3d{sep}2026-08-25T09:30:00+02:00{sep}Some Author{sep}server : add DFlash speculative decoding",
            sep = FIELD_SEPARATOR
        );

        let commit = parse_commit(&output).expect("parses");

        assert_eq!(commit.short_commit, "a1b2c3d");
        assert_eq!(commit.author, "Some Author");
        assert_eq!(commit.subject, "server : add DFlash speculative decoding");
        assert_eq!(commit.committed_at, "2026-08-25T09:30:00+02:00");
    }

    #[test]
    fn reads_the_default_branch_from_ls_remote() {
        let output = "ref: refs/heads/master\tHEAD\na1b2c3d\tHEAD\n";
        assert_eq!(
            parse_symref_default_branch(output).as_deref(),
            Some("master")
        );
    }

    #[test]
    fn reads_the_default_branch_from_the_remote_head_pointer() {
        assert_eq!(
            parse_remote_head("origin/main\n", "origin").as_deref(),
            Some("main")
        );
        assert_eq!(parse_remote_head("", "origin"), None);
    }

    #[test]
    fn checkout_plan_prefers_an_existing_local_branch() {
        let refs = parse_refs(
            &[
                line(&["refs/heads/master", "aaa", "", "", "origin/master", "*"]),
                line(&["refs/remotes/origin/master", "aaa", "", "", "", ""]),
            ]
            .join("\n"),
        );

        assert_eq!(plan_checkout("master", &refs), RefCheckout::LocalBranch);
    }

    #[test]
    fn checkout_plan_tracks_a_remote_only_branch() {
        let refs = parse_refs(&line(&[
            "refs/remotes/origin/gg/experimental",
            "aaa",
            "",
            "",
            "",
            "",
        ]));

        assert_eq!(
            plan_checkout("origin/gg/experimental", &refs),
            RefCheckout::TrackRemoteBranch
        );
    }

    #[test]
    fn checkout_plan_detaches_for_tags_and_raw_shas() {
        let refs = parse_refs(&line(&["refs/tags/b7104", "aaa", "bbb", "", "", ""]));

        assert_eq!(plan_checkout("b7104", &refs), RefCheckout::Detach);
        assert_eq!(plan_checkout("812abc9", &refs), RefCheckout::Detach);
    }

    #[test]
    fn derives_the_local_branch_name_for_a_remote_branch() {
        assert_eq!(
            local_branch_for_remote("origin/experimental/dflash").as_deref(),
            Some("experimental/dflash")
        );
        assert_eq!(local_branch_for_remote("origin/"), None);
        assert_eq!(local_branch_for_remote("master"), None);
    }
}
