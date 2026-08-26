use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

use super::record::RuntimeRecord;

pub const RUNTIMES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RuntimeRegistry {
    pub schema_version: u32,
    pub runtimes: Vec<RuntimeRecord>,
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self {
            schema_version: RUNTIMES_SCHEMA_VERSION,
            runtimes: Vec::new(),
        }
    }
}

impl RuntimeRegistry {
    pub fn find(&self, id: &str) -> Option<&RuntimeRecord> {
        self.runtimes.iter().find(|runtime| runtime.id == id)
    }

    /// Newest first, which is the order the build history is read in.
    pub fn sorted(&self) -> Vec<RuntimeRecord> {
        let mut runtimes = self.runtimes.clone();
        runtimes.sort_by(|left, right| right.build_date.cmp(&left.build_date));
        runtimes
    }

    pub fn insert(&mut self, runtime: RuntimeRecord) {
        self.runtimes.push(runtime);
    }

    pub fn insert_missing(&mut self, runtimes: impl IntoIterator<Item = RuntimeRecord>) -> usize {
        let mut inserted = 0;
        for runtime in runtimes {
            if self.find(&runtime.id).is_none() {
                self.insert(runtime);
                inserted += 1;
            }
        }
        inserted
    }

    /// Reconciles the central history with self-describing snapshot metadata. Existing records
    /// are replaced when their sidecar pointer changed after an interrupted re-inspection.
    pub fn upsert_discovered(
        &mut self,
        runtimes: impl IntoIterator<Item = RuntimeRecord>,
    ) -> usize {
        let mut changed = 0;
        for runtime in runtimes {
            match self
                .runtimes
                .iter_mut()
                .find(|existing| existing.id == runtime.id)
            {
                Some(existing) if existing != &runtime => {
                    *existing = runtime;
                    changed += 1;
                }
                Some(_) => {}
                None => {
                    self.runtimes.push(runtime);
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn replace(&mut self, runtime: RuntimeRecord) -> AppResult<RuntimeRecord> {
        let existing = self
            .runtimes
            .iter_mut()
            .find(|existing| existing.id == runtime.id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::RuntimeNotFound,
                    "That runtime is no longer registered.",
                )
                .with_details(runtime.id.clone())
            })?;
        Ok(std::mem::replace(existing, runtime))
    }

    pub fn remove(&mut self, id: &str) -> AppResult<RuntimeRecord> {
        let index = self
            .runtimes
            .iter()
            .position(|runtime| runtime.id == id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::RuntimeNotFound,
                    "That runtime is no longer registered.",
                )
                .with_details(id.to_string())
            })?;

        Ok(self.runtimes.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::build::profile::{BuildBackend, BuildConfiguration};

    fn record(id: &str, build_date: &str) -> RuntimeRecord {
        RuntimeRecord {
            id: id.into(),
            source_id: "s1".into(),
            source_name: "llama.cpp".into(),
            repository: "https://github.com/ggml-org/llama.cpp".into(),
            commit: format!("{id}0000000000000000000000000000000000000"),
            short_commit: id.into(),
            branch: "master".into(),
            backend: BuildBackend::Cuda,
            configuration: BuildConfiguration::Release,
            generator: "Ninja".into(),
            build_date: build_date.into(),
            directory: PathBuf::from("/data/runtimes"),
            executable: PathBuf::from("/data/runtimes/llama-server.exe"),
            size_bytes: 0,
            file_count: 0,
            capabilities: None,
        }
    }

    #[test]
    fn an_empty_document_deserializes_to_an_empty_registry() {
        let registry: RuntimeRegistry = serde_json::from_str("{}").expect("defaults");
        assert_eq!(registry, RuntimeRegistry::default());
    }

    #[test]
    fn runtimes_are_listed_newest_first() {
        let mut registry = RuntimeRegistry::default();
        registry.insert(record("aaa", "2026-08-20T10:00:00Z"));
        registry.insert(record("ccc", "2026-08-25T10:00:00Z"));
        registry.insert(record("bbb", "2026-08-22T10:00:00Z"));

        let ids: Vec<String> = registry.sorted().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, vec!["ccc", "bbb", "aaa"]);
    }

    #[test]
    fn rebuilding_the_same_commit_adds_a_snapshot_rather_than_replacing_one() {
        let mut registry = RuntimeRegistry::default();
        registry.insert(record("aaa", "2026-08-20T10:00:00Z"));
        registry.insert(record("bbb", "2026-08-21T10:00:00Z"));

        assert_eq!(registry.runtimes.len(), 2);
    }

    #[test]
    fn recovery_only_inserts_runtime_ids_missing_from_the_registry() {
        let mut registry = RuntimeRegistry::default();
        registry.insert(record("aaa", "2026-08-20T10:00:00Z"));

        let inserted = registry.insert_missing([
            record("aaa", "2026-08-20T10:00:00Z"),
            record("bbb", "2026-08-21T10:00:00Z"),
        ]);

        assert_eq!(inserted, 1);
        assert_eq!(registry.runtimes.len(), 2);
    }

    #[test]
    fn recovery_replaces_an_existing_record_when_snapshot_metadata_is_newer() {
        let mut registry = RuntimeRegistry::default();
        registry.insert(record("aaa", "2026-08-20T10:00:00Z"));
        let mut discovered = record("aaa", "2026-08-20T10:00:00Z");
        discovered.generator = "Visual Studio".into();

        assert_eq!(registry.upsert_discovered([discovered]), 1);
        assert_eq!(registry.runtimes[0].generator, "Visual Studio");
    }

    #[test]
    fn removing_an_unknown_runtime_reports_an_error() {
        let mut registry = RuntimeRegistry::default();
        assert_eq!(
            registry.remove("nope").expect_err("must fail").code,
            ErrorCode::RuntimeNotFound
        );
    }
}
