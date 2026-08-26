use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::path::PathBuf;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult, ErrorCode};

use super::model::{
    ServerLogEntry, ServerLogFact, ServerLogLevel, ServerLogStream, ServerLogsSnapshot,
};

const MAX_LOG_ENTRIES: usize = 10_000;

pub struct ServerLogBuffer {
    directory: PathBuf,
    entries: VecDeque<ServerLogEntry>,
    dropped_entries: u64,
    next_sequence: u64,
    current_file: Option<PathBuf>,
    writer: Option<mpsc::UnboundedSender<String>>,
}

impl ServerLogBuffer {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            entries: VecDeque::new(),
            dropped_entries: 0,
            next_sequence: 1,
            current_file: None,
            writer: None,
        }
    }

    pub fn begin_run(&mut self, profile_name: &str, generation: u64) -> AppResult<PathBuf> {
        std::fs::create_dir_all(&self.directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not create {}.", self.directory.display()),
            )
            .with_details(error.to_string())
        })?;

        self.writer.take();
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let safe_profile = safe_segment(profile_name);
        let path = self.directory.join(format!(
            "llama-server-{timestamp}-{generation}-{safe_profile}.log"
        ));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                AppError::new(
                    ErrorCode::Io,
                    format!("Could not create {}.", path.display()),
                )
                .with_details(error.to_string())
            })?;
        let (sender, mut receiver) = mpsc::unbounded_channel::<String>();
        let mut file = tokio::fs::File::from_std(file);
        tokio::spawn(async move {
            while let Some(line) = receiver.recv().await {
                if file.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
            let _ = file.flush().await;
        });
        self.current_file = Some(path.clone());
        self.writer = Some(sender);
        Ok(path)
    }

    pub fn finish_run(&mut self) {
        self.writer.take();
    }

    pub fn append(&mut self, stream: ServerLogStream, text: String) -> ServerLogEntry {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let entry = ServerLogEntry {
            sequence: self.next_sequence,
            timestamp,
            stream,
            level: classify_level(stream, &text),
            fact: classify_fact(&text),
            text,
        };
        self.next_sequence += 1;

        if self.entries.len() == MAX_LOG_ENTRIES {
            self.entries.pop_front();
            self.dropped_entries += 1;
        }
        self.entries.push_back(entry.clone());

        if let Some(writer) = &self.writer {
            // The file is a raw transcript. Timestamp, stream, level, and parsed facts remain
            // available beside the line in memory, but are never injected into child output.
            let rendered = format!("{}\n", entry.text);
            let _ = writer.send(rendered);
        }
        entry
    }

    pub fn clear(&mut self) -> u64 {
        self.entries.clear();
        self.dropped_entries = 0;
        self.next_sequence
    }

    pub fn snapshot(&self) -> ServerLogsSnapshot {
        ServerLogsSnapshot {
            entries: self.entries.iter().cloned().collect(),
            dropped_entries: self.dropped_entries,
            next_sequence: self.next_sequence,
            current_file: self.current_file.clone(),
        }
    }
}

fn safe_segment(value: &str) -> String {
    let mut result: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    let trimmed = result.trim_matches('-');
    if trimmed.is_empty() {
        "profile".into()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn classify_level(stream: ServerLogStream, text: &str) -> ServerLogLevel {
    let lower = text.to_ascii_lowercase();
    if ["fatal", "error", "out of memory", "exception", "failed"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        ServerLogLevel::Error
    } else if lower.contains("warn") {
        ServerLogLevel::Warn
    } else if lower.contains("debug") {
        ServerLogLevel::Debug
    } else if lower.contains("trace") {
        ServerLogLevel::Trace
    } else if stream == ServerLogStream::Stderr && lower.contains("cuda error") {
        ServerLogLevel::Error
    } else {
        ServerLogLevel::Info
    }
}

fn classify_fact(text: &str) -> Option<ServerLogFact> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("token")
        && (lower.contains("t/s")
            || lower.contains("tokens per second")
            || lower.contains("tokens/s"))
    {
        Some(ServerLogFact::Throughput)
    } else if lower.contains("kv") && lower.contains("cache") {
        Some(ServerLogFact::KvCache)
    } else if lower.contains("offload") && (lower.contains("gpu") || lower.contains("layer")) {
        Some(ServerLogFact::GpuOffload)
    } else if lower.contains("listening") || lower.contains("http server") {
        Some(ServerLogFact::Listening)
    } else if lower.contains("load") && (lower.contains("model") || lower.contains("tensor")) {
        Some(ServerLogFact::ModelLoad)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_startup_facts_without_changing_raw_text() {
        let raw = "llama_kv_cache: CUDA0 KV buffer size = 512 MiB";
        assert_eq!(classify_fact(raw), Some(ServerLogFact::KvCache));
        assert_eq!(
            classify_fact("offloaded 33/33 layers to GPU"),
            Some(ServerLogFact::GpuOffload)
        );
        assert_eq!(
            classify_fact("prompt eval time = 42 ms / 128 tokens (3047 tokens/s)"),
            Some(ServerLogFact::Throughput)
        );
    }

    #[test]
    fn file_names_are_bounded_and_windows_safe() {
        assert_eq!(
            safe_segment("My CUDA: profile / test"),
            "My-CUDA-profile-test"
        );
        assert_eq!(safe_segment("  "), "profile");
        assert!(safe_segment(&"x".repeat(100)).len() <= 48);
    }

    #[test]
    fn bounded_buffer_reports_dropped_entries() {
        let temp = tempfile::tempdir().expect("temp");
        let mut buffer = ServerLogBuffer::new(temp.path().to_path_buf());
        for index in 0..=MAX_LOG_ENTRIES {
            buffer.append(ServerLogStream::Stdout, format!("line {index}"));
        }
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.entries.len(), MAX_LOG_ENTRIES);
        assert_eq!(snapshot.dropped_entries, 1);
        assert_eq!(snapshot.entries[0].text, "line 1");
    }
}
