use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

/// Reads a JSON document, falling back to the type's default when the file does not exist.
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> AppResult<T> {
    match std::fs::read_to_string(path) {
        Ok(contents) if contents.trim().is_empty() => Ok(T::default()),
        Ok(contents) => serde_json::from_str(&contents).map_err(|error| {
            AppError::new(
                ErrorCode::Config,
                format!("{} is not valid JSON for this application.", path.display()),
            )
            .with_hint("Fix or delete the file, then restart.")
            .with_details(error.to_string())
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(AppError::new(
            ErrorCode::Io,
            format!("Could not read {}.", path.display()),
        )
        .with_details(error.to_string())),
    }
}

/// Writes a JSON document via a temporary file and a rename so an interrupted write cannot
/// leave a truncated document behind.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not create {}.", parent.display()),
            )
            .with_details(error.to_string())
        })?;
    }

    let serialized = serde_json::to_string_pretty(value)?;
    let temporary = path.with_extension(format!("json.{}.tmp", uuid::Uuid::new_v4()));

    std::fs::write(&temporary, serialized.as_bytes()).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not write {}.", temporary.display()),
        )
        .with_details(error.to_string())
    })?;

    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        AppError::new(ErrorCode::Io, format!("Could not save {}.", path.display()))
            .with_details(error.to_string())
    })
}

/// An in-memory cached JSON document backed by a single file on disk.
#[derive(Debug)]
pub struct JsonStore<T> {
    path: PathBuf,
    value: Mutex<T>,
}

impl<T: Clone + Default + Serialize + DeserializeOwned> JsonStore<T> {
    pub fn load(path: PathBuf) -> AppResult<Self> {
        let value = read_json::<T>(&path)?;
        Ok(Self {
            path,
            value: Mutex::new(value),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> T {
        self.lock().clone()
    }

    pub fn replace(&self, next: T) -> AppResult<T> {
        let mut guard = self.lock();
        write_json_atomic(&self.path, &next)?;
        *guard = next.clone();
        Ok(next)
    }

    /// Applies `change` to the document and persists the result.
    ///
    /// The candidate is persisted before it replaces the cached value. Keeping the lock for the
    /// write serializes concurrent commands and guarantees that a failed write cannot leave the
    /// in-memory state ahead of the document on disk.
    pub fn update<F, R>(&self, change: F) -> AppResult<(T, R)>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        let mut next = guard.clone();
        let outcome = change(&mut next);

        write_json_atomic(&self.path, &next)?;
        *guard = next.clone();

        Ok((next, outcome))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        self.value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    struct Document {
        counter: u32,
        label: String,
    }

    #[test]
    fn missing_file_yields_the_default_document() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store: JsonStore<Document> =
            JsonStore::load(dir.path().join("missing.json")).expect("loads");

        assert_eq!(store.get(), Document::default());
    }

    #[test]
    fn updates_are_persisted_and_reloadable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("doc.json");

        let store: JsonStore<Document> = JsonStore::load(path.clone()).expect("loads");
        store
            .update(|document| {
                document.counter = 7;
                document.label = "seven".into();
            })
            .expect("updates");

        let reloaded: JsonStore<Document> = JsonStore::load(path).expect("reloads");
        assert_eq!(reloaded.get().counter, 7);
        assert_eq!(reloaded.get().label, "seven");
    }

    #[test]
    fn invalid_json_reports_a_config_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("broken.json");
        std::fs::write(&path, "{ not json").expect("write");

        let error = read_json::<Document>(&path).expect_err("must fail");
        assert_eq!(error.code, ErrorCode::Config);
        assert!(error.details.is_some());
    }

    #[test]
    fn atomic_write_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("doc.json");

        write_json_atomic(&path, &Document::default()).expect("writes");

        assert!(path.exists());
        let leftovers = std::fs::read_dir(dir.path())
            .expect("read temp directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn a_failed_replace_keeps_the_cached_value_unchanged() {
        let dir = tempfile::tempdir().expect("temp dir");
        let blocker = dir.path().join("not-a-directory");
        std::fs::write(&blocker, b"file").expect("create blocker");

        let store = JsonStore {
            path: blocker.join("doc.json"),
            value: Mutex::new(Document {
                counter: 1,
                label: "original".into(),
            }),
        };

        let error = store
            .replace(Document {
                counter: 2,
                label: "replacement".into(),
            })
            .expect_err("the write must fail");

        assert_eq!(error.code, ErrorCode::Io);
        assert_eq!(store.get().counter, 1);
        assert_eq!(store.get().label, "original");
    }

    #[test]
    fn concurrent_updates_are_serialized_in_memory_and_on_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("doc.json");
        let store = std::sync::Arc::new(
            JsonStore::<Document>::load(path.clone()).expect("load shared store"),
        );

        let workers: Vec<_> = (0..16)
            .map(|_| {
                let store = std::sync::Arc::clone(&store);
                std::thread::spawn(move || {
                    store
                        .update(|document| document.counter += 1)
                        .expect("persist concurrent update");
                })
            })
            .collect();

        for worker in workers {
            worker.join().expect("worker completes");
        }

        let persisted: Document = read_json(&path).expect("read persisted document");
        assert_eq!(store.get().counter, 16);
        assert_eq!(persisted.counter, 16);
    }
}
