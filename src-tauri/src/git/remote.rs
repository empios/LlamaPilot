use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

/// Parses `git remote -v`, whose lines look like `origin<TAB><url> (fetch)`.
pub fn parse_remotes(output: &str) -> Vec<GitRemote> {
    let mut ordered: Vec<String> = Vec::new();
    let mut collected: BTreeMap<String, GitRemote> = BTreeMap::new();

    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let (Some(name), Some(url)) = (fields.next(), fields.next()) else {
            continue;
        };
        let direction = fields.next().unwrap_or("(fetch)");

        let entry = collected.entry(name.to_string()).or_insert_with(|| {
            ordered.push(name.to_string());
            GitRemote {
                name: name.to_string(),
                fetch_url: None,
                push_url: None,
            }
        });

        match direction {
            "(push)" => entry.push_url = Some(url.to_string()),
            _ => entry.fetch_url = Some(url.to_string()),
        }
    }

    ordered
        .into_iter()
        .filter_map(|name| collected.remove(&name))
        .collect()
}

/// Derives a display name such as `ggml-org/llama.cpp` from a clone URL.
pub fn describe_repository(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    let without_suffix = trimmed.strip_suffix(".git").unwrap_or(trimmed);

    let path = match without_suffix.split_once("://") {
        Some((_, authority_and_path)) => strip_credentials(authority_and_path),
        // Only scp-style URLs (`git@host:owner/repo`) use a colon as a path separator; doing this
        // unconditionally would mangle a colon that legitimately appears in a path.
        None => {
            let value = strip_credentials(without_suffix);
            match value.split_once(':') {
                Some((host, path)) => format!("{host}/{path}"),
                None => value,
            }
        }
    };

    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    match segments.as_slice() {
        [.., owner, repository] => format!("{owner}/{repository}"),
        [repository] => (*repository).to_string(),
        [] => without_suffix.to_string(),
    }
}

/// Removes `user@` from the authority only, leaving any `@` inside the path untouched.
fn strip_credentials(value: &str) -> String {
    match value.split_once('/') {
        Some((authority, path)) => {
            let host = authority
                .split_once('@')
                .map(|(_, host)| host)
                .unwrap_or(authority);
            format!("{host}/{path}")
        }
        None => value
            .split_once('@')
            .map(|(_, host)| host.to_string())
            .unwrap_or_else(|| value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REMOTE_OUTPUT: &str = "\
origin\thttps://github.com/ggml-org/llama.cpp (fetch)
origin\thttps://github.com/ggml-org/llama.cpp (push)
experimental\thttps://github.com/some-user/llama.cpp (fetch)
experimental\thttps://github.com/some-user/llama.cpp (push)
";

    #[test]
    fn groups_fetch_and_push_urls_per_remote() {
        let remotes = parse_remotes(REMOTE_OUTPUT);

        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(
            remotes[0].fetch_url.as_deref(),
            Some("https://github.com/ggml-org/llama.cpp")
        );
        assert_eq!(
            remotes[0].push_url.as_deref(),
            Some("https://github.com/ggml-org/llama.cpp")
        );
        assert_eq!(remotes[1].name, "experimental");
    }

    #[test]
    fn preserves_the_order_git_reported() {
        let remotes = parse_remotes("zeta\thttps://z (fetch)\nalpha\thttps://a (fetch)\n");
        let names: Vec<&str> = remotes.iter().map(|remote| remote.name.as_str()).collect();

        assert_eq!(names, vec!["zeta", "alpha"]);
    }

    #[test]
    fn empty_output_yields_no_remotes() {
        assert!(parse_remotes("").is_empty());
    }

    #[test]
    fn describes_https_ssh_and_local_repositories() {
        assert_eq!(
            describe_repository("https://github.com/ggml-org/llama.cpp"),
            "ggml-org/llama.cpp"
        );
        assert_eq!(
            describe_repository("https://github.com/ggml-org/llama.cpp.git"),
            "ggml-org/llama.cpp"
        );
        assert_eq!(
            describe_repository("git@github.com:some-user/llama.cpp.git"),
            "some-user/llama.cpp"
        );
        assert_eq!(
            describe_repository("https://example.com/llama.cpp/"),
            "example.com/llama.cpp"
        );
        assert_eq!(
            describe_repository("ssh://git@github.com/ggml-org/llama.cpp.git"),
            "ggml-org/llama.cpp"
        );
    }

    #[test]
    fn a_colon_inside_a_path_is_not_treated_as_a_separator() {
        assert_eq!(
            describe_repository("https://example.com/team/llama:cpp"),
            "team/llama:cpp"
        );
    }
}
