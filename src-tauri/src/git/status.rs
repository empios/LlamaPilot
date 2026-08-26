use serde::Serialize;

/// Working-tree state derived from `git status --porcelain=v2 --branch`.
///
/// Porcelain v2 is used because it is explicitly documented as stable for scripts, unlike the
/// human-readable output which changes between Git releases.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicted: u32,
}

impl GitStatus {
    /// Modifications to files Git tracks. These block updates and ref switches.
    pub fn has_tracked_changes(&self) -> bool {
        self.staged > 0 || self.unstaged > 0 || self.conflicted > 0
    }

    /// Any local deviation at all, including untracked files.
    ///
    /// Reported separately from [`Self::has_tracked_changes`] because a llama.cpp checkout
    /// routinely contains untracked scratch files that must not block a fast-forward.
    pub fn is_dirty(&self) -> bool {
        self.has_tracked_changes() || self.untracked > 0
    }

    pub fn short_commit(&self) -> Option<String> {
        self.commit
            .as_ref()
            .map(|commit| commit.chars().take(7).collect())
    }
}

pub fn parse_status(output: &str) -> GitStatus {
    let mut status = GitStatus::default();

    for line in output.lines() {
        if let Some(header) = line.strip_prefix("# ") {
            apply_header(&mut status, header);
        } else if let Some(entry) = line.strip_prefix("1 ").or_else(|| line.strip_prefix("2 ")) {
            apply_change(&mut status, entry);
        } else if line.starts_with("u ") {
            status.conflicted += 1;
        } else if line.starts_with("? ") {
            status.untracked += 1;
        }
    }

    status
}

fn apply_header(status: &mut GitStatus, header: &str) {
    let Some((key, value)) = header.split_once(' ') else {
        return;
    };

    match key {
        "branch.oid" if value != "(initial)" => status.commit = Some(value.to_string()),
        "branch.head" => {
            if value == "(detached)" {
                status.detached = true;
            } else {
                status.branch = Some(value.to_string());
            }
        }
        "branch.upstream" => status.upstream = Some(value.to_string()),
        "branch.ab" => {
            let (ahead, behind) = parse_ahead_behind(value);
            status.ahead = ahead;
            status.behind = behind;
        }
        _ => {}
    }
}

fn parse_ahead_behind(value: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    for token in value.split_whitespace() {
        let Some((sign, digits)) = token.split_at_checked(1) else {
            continue;
        };
        let Ok(count) = digits.parse::<u32>() else {
            continue;
        };

        match sign {
            "+" => ahead = count,
            "-" => behind = count,
            _ => {}
        }
    }

    (ahead, behind)
}

/// Entry format: `<XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>` where `X` is the staged state and
/// `Y` the unstaged state; `.` means unchanged in that position.
fn apply_change(status: &mut GitStatus, entry: &str) {
    let Some(codes) = entry.split_whitespace().next() else {
        return;
    };

    let mut characters = codes.chars();
    if matches!(characters.next(), Some(state) if state != '.') {
        status.staged += 1;
    }
    if matches!(characters.next(), Some(state) if state != '.') {
        status.unstaged += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN_TRACKING_BRANCH: &str = "\
# branch.oid a1b2c3d4e5f60718293a4b5c6d7e8f9012345678
# branch.head master
# branch.upstream origin/master
# branch.ab +0 -0
";

    const BEHIND_WITH_LOCAL_EDITS: &str = "\
# branch.oid a1b2c3d4e5f60718293a4b5c6d7e8f9012345678
# branch.head master
# branch.upstream origin/master
# branch.ab +2 -12
1 .M N... 100644 100644 100644 a1b2c3d a1b2c3d ggml/src/ggml.c
1 M. N... 100644 100644 100644 a1b2c3d a1b2c3d tools/server/server.cpp
? build/CMakeCache.txt
";

    const DETACHED_AT_TAG: &str = "\
# branch.oid 92fac1a0b1c2d3e4f50617283940a1b2c3d4e5f6
# branch.head (detached)
";

    const CONFLICTED: &str = "\
# branch.oid a1b2c3d4e5f60718293a4b5c6d7e8f9012345678
# branch.head master
u UU N... 100644 100644 100644 100644 aaa bbb ccc src/conflict.cpp
";

    #[test]
    fn parses_a_clean_tracking_branch() {
        let status = parse_status(CLEAN_TRACKING_BRANCH);

        assert_eq!(status.branch.as_deref(), Some("master"));
        assert_eq!(status.upstream.as_deref(), Some("origin/master"));
        assert_eq!(status.short_commit().as_deref(), Some("a1b2c3d"));
        assert_eq!((status.ahead, status.behind), (0, 0));
        assert!(!status.detached);
        assert!(!status.is_dirty());
    }

    #[test]
    fn counts_staged_unstaged_and_untracked_separately() {
        let status = parse_status(BEHIND_WITH_LOCAL_EDITS);

        assert_eq!(status.staged, 1);
        assert_eq!(status.unstaged, 1);
        assert_eq!(status.untracked, 1);
        assert_eq!((status.ahead, status.behind), (2, 12));
        assert!(status.has_tracked_changes());
    }

    #[test]
    fn untracked_files_alone_do_not_block_updates() {
        let status = parse_status("# branch.head master\n? build/CMakeCache.txt\n");

        assert_eq!(status.untracked, 1);
        assert!(status.is_dirty());
        assert!(!status.has_tracked_changes());
    }

    #[test]
    fn recognises_detached_head() {
        let status = parse_status(DETACHED_AT_TAG);

        assert!(status.detached);
        assert!(status.branch.is_none());
        assert_eq!(status.short_commit().as_deref(), Some("92fac1a"));
    }

    #[test]
    fn counts_unmerged_entries_as_conflicts() {
        let status = parse_status(CONFLICTED);

        assert_eq!(status.conflicted, 1);
        assert!(status.has_tracked_changes());
    }

    #[test]
    fn an_initial_repository_has_no_commit() {
        let status = parse_status("# branch.oid (initial)\n# branch.head main\n");

        assert!(status.commit.is_none());
        assert_eq!(status.branch.as_deref(), Some("main"));
    }

    #[test]
    fn empty_output_is_a_clean_default() {
        assert_eq!(parse_status(""), GitStatus::default());
    }
}
