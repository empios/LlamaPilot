use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

use super::record::LlamaSource;

pub const SOURCES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceRegistry {
    pub schema_version: u32,
    pub sources: Vec<LlamaSource>,
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self {
            schema_version: SOURCES_SCHEMA_VERSION,
            sources: Vec::new(),
        }
    }
}

impl SourceRegistry {
    pub fn find(&self, id: &str) -> Option<&LlamaSource> {
        self.sources.iter().find(|source| source.id == id)
    }

    pub fn find_by_directory(&self, directory: &Path) -> Option<&LlamaSource> {
        self.sources
            .iter()
            .find(|source| paths_match(&source.directory, directory))
    }

    pub fn names(&self) -> Vec<String> {
        self.sources
            .iter()
            .map(|source| source.name.clone())
            .collect()
    }

    pub fn insert(&mut self, source: LlamaSource) -> AppResult<()> {
        if self.find_by_directory(&source.directory).is_some() {
            return Err(AppError::new(
                ErrorCode::SourceExists,
                "That folder is already registered as a source.",
            )
            .with_details(source.directory.display().to_string()));
        }

        self.sources.push(source);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> AppResult<LlamaSource> {
        let index = self
            .sources
            .iter()
            .position(|source| source.id == id)
            .ok_or_else(|| missing_source(id))?;

        Ok(self.sources.remove(index))
    }

    pub fn update<F>(&mut self, id: &str, change: F) -> AppResult<LlamaSource>
    where
        F: FnOnce(&mut LlamaSource),
    {
        let source = self
            .sources
            .iter_mut()
            .find(|source| source.id == id)
            .ok_or_else(|| missing_source(id))?;

        change(source);
        Ok(source.clone())
    }
}

pub fn missing_source(id: &str) -> AppError {
    AppError::new(
        ErrorCode::SourceNotFound,
        "That llama.cpp source is no longer registered.",
    )
    .with_details(id.to_string())
}

/// Compares paths case-insensitively on Windows, where the filesystem is case-preserving but
/// case-insensitive.
fn paths_match(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn source(id: &str, directory: &str) -> LlamaSource {
        LlamaSource {
            id: id.to_string(),
            name: format!("source-{id}"),
            repository: "https://github.com/ggml-org/llama.cpp".to_string(),
            directory: PathBuf::from(directory),
            remote: "origin".to_string(),
            current_ref: "master".to_string(),
            current_commit: "a1b2c3d4".to_string(),
            added_at: "2026-08-25T10:00:00Z".to_string(),
            last_fetched_at: None,
        }
    }

    #[test]
    fn an_empty_document_deserializes_to_an_empty_registry() {
        let registry: SourceRegistry = serde_json::from_str("{}").expect("defaults");
        assert_eq!(registry, SourceRegistry::default());
    }

    #[test]
    fn registering_the_same_directory_twice_is_rejected() {
        let mut registry = SourceRegistry::default();
        registry
            .insert(source("a", "/repos/llama.cpp"))
            .expect("first insert");

        let error = registry
            .insert(source("b", "/repos/llama.cpp"))
            .expect_err("duplicate must fail");

        assert_eq!(error.code, ErrorCode::SourceExists);
        assert_eq!(registry.sources.len(), 1);
    }

    #[test]
    fn removing_an_unknown_source_reports_source_not_found() {
        let mut registry = SourceRegistry::default();
        let error = registry.remove("nope").expect_err("must fail");

        assert_eq!(error.code, ErrorCode::SourceNotFound);
    }

    #[test]
    fn updates_mutate_the_stored_record() {
        let mut registry = SourceRegistry::default();
        registry
            .insert(source("a", "/repos/llama.cpp"))
            .expect("insert");

        let updated = registry
            .update("a", |source| source.current_ref = "b7104".into())
            .expect("update");

        assert_eq!(updated.current_ref, "b7104");
        assert_eq!(registry.find("a").expect("present").current_ref, "b7104");
    }
}
