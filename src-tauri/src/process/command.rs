use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// A process invocation described as a program plus an argument array.
///
/// There is deliberately no way to express a shell string. Anything the user types is carried as
/// a discrete argument, so quoting and metacharacters can never change what gets executed.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub working_directory: Option<PathBuf>,
    pub environment: Vec<(OsString, OsString)>,
}

impl CommandSpec {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
            working_directory: None,
            environment: Vec::new(),
        }
    }

    pub fn arg(mut self, value: impl AsRef<OsStr>) -> Self {
        self.args.push(value.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args.extend(
            values
                .into_iter()
                .map(|value| value.as_ref().to_os_string()),
        );
        self
    }

    pub fn current_dir(mut self, directory: impl AsRef<Path>) -> Self {
        self.working_directory = Some(directory.as_ref().to_path_buf());
        self
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.environment
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    pub fn program_display(&self) -> String {
        self.program.to_string_lossy().into_owned()
    }

    pub fn args_display(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    /// Human-readable rendering for logs and the command preview. Never used for execution.
    pub fn to_display_string(&self) -> String {
        let mut rendered = quote_for_display(&self.program_display());
        for argument in self.args_display() {
            rendered.push(' ');
            rendered.push_str(&quote_for_display(&argument));
        }
        rendered
    }
}

fn quote_for_display(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }

    let needs_quotes = value
        .chars()
        .any(|character| character.is_whitespace() || matches!(character, '"' | '\''));

    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_kept_discrete() {
        let spec = CommandSpec::new("git").args(["clone", "--progress", "https://example/x.git"]);

        assert_eq!(spec.program_display(), "git");
        assert_eq!(
            spec.args_display(),
            vec!["clone", "--progress", "https://example/x.git"]
        );
    }

    #[test]
    fn display_quotes_only_what_needs_quoting() {
        let spec = CommandSpec::new(r"C:\Program Files\Git\git.exe").args([
            "-C",
            r"C:\repos\llama.cpp",
            "status",
        ]);

        assert_eq!(
            spec.to_display_string(),
            r#""C:\Program Files\Git\git.exe" -C C:\repos\llama.cpp status"#
        );
    }

    #[test]
    fn display_never_merges_arguments_containing_spaces() {
        let spec = CommandSpec::new("llama-server.exe").args(["-m", "my model.gguf"]);

        assert_eq!(spec.args.len(), 2);
        assert_eq!(
            spec.to_display_string(),
            r#"llama-server.exe -m "my model.gguf""#
        );
    }
}
