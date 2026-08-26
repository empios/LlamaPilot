use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config::store::write_json_atomic;
use crate::error::{AppError, AppResult, ErrorCode};

use super::record::{normalize_input, LaunchProfile, ProfileInput, PROFILE_SCHEMA_VERSION};
use super::ResolvedProfileTarget;

pub struct ProfileRepository {
    directory: PathBuf,
    lock: Mutex<()>,
}

impl ProfileRepository {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            lock: Mutex::new(()),
        }
    }

    pub fn list(&self) -> AppResult<Vec<LaunchProfile>> {
        let _guard = self.lock();
        let mut profiles = Vec::new();
        for entry in fs::read_dir(&self.directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not read {}.", self.directory.display()),
            )
            .with_details(error.to_string())
        })? {
            let entry = entry.map_err(|error| {
                AppError::new(ErrorCode::Io, "Could not inspect a profile file.")
                    .with_details(error.to_string())
            })?;
            let path = entry.path();
            if !entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
                || path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_none_or(|extension| !extension.eq_ignore_ascii_case("json"))
            {
                continue;
            }
            profiles.push(read_profile(&path)?);
        }
        profiles.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(profiles)
    }

    pub fn find(&self, id: &str) -> AppResult<LaunchProfile> {
        validate_id(id)?;
        let _guard = self.lock();
        let path = self.path(id);
        if !path.is_file() {
            return Err(profile_not_found(id));
        }
        read_profile(&path)
    }

    pub fn create(
        &self,
        input: ProfileInput,
        target: ResolvedProfileTarget,
    ) -> AppResult<LaunchProfile> {
        let input = normalize_input(input)?;
        let _guard = self.lock();
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let profile = assemble_profile(id, now.clone(), now, input, target);
        write_json_atomic(&self.path(&profile.id), &profile)?;
        Ok(profile)
    }

    pub fn update(
        &self,
        id: &str,
        input: ProfileInput,
        target: ResolvedProfileTarget,
    ) -> AppResult<LaunchProfile> {
        validate_id(id)?;
        let input = normalize_input(input)?;
        let _guard = self.lock();
        let path = self.path(id);
        if !path.is_file() {
            return Err(profile_not_found(id));
        }
        let previous = read_profile(&path)?;
        let profile = assemble_profile(
            id.to_string(),
            previous.created_at,
            chrono::Utc::now().to_rfc3339(),
            input,
            target,
        );
        write_json_atomic(&path, &profile)?;
        Ok(profile)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        validate_id(id)?;
        let _guard = self.lock();
        let path = self.path(id);
        if !path.is_file() {
            return Err(profile_not_found(id));
        }
        fs::remove_file(&path).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not delete {}.", path.display()),
            )
            .with_details(error.to_string())
        })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{id}.json"))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn assemble_profile(
    id: String,
    created_at: String,
    updated_at: String,
    input: ProfileInput,
    target: ResolvedProfileTarget,
) -> LaunchProfile {
    LaunchProfile {
        schema_version: PROFILE_SCHEMA_VERSION,
        id,
        name: input.name,
        description: input.description,
        runtime_id: input.runtime_id,
        runtime_label: target.runtime_label,
        model_id: input.model_id,
        model_name: target.model_name,
        model_path: target.model_path,
        projector_path: target.projector_path,
        host: input.host,
        port: input.port,
        auto_select_port: input.auto_select_port,
        options: input.options,
        environment: input.environment,
        additional_arguments: input.additional_arguments,
        created_at,
        updated_at,
    }
}

fn read_profile(path: &Path) -> AppResult<LaunchProfile> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::new(ErrorCode::Io, format!("Could not read {}.", path.display()))
            .with_details(error.to_string())
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            ErrorCode::Config,
            format!("{} is not a valid launch profile.", path.display()),
        )
        .with_details(error.to_string())
    })
}

fn validate_id(id: &str) -> AppResult<()> {
    uuid::Uuid::parse_str(id).map_err(|_| {
        AppError::new(ErrorCode::InvalidProfile, "The profile id is invalid.")
            .with_details(id.to_string())
    })?;
    Ok(())
}

fn profile_not_found(id: &str) -> AppError {
    AppError::new(
        ErrorCode::ProfileNotFound,
        "That launch profile no longer exists.",
    )
    .with_details(id.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn input(name: &str) -> ProfileInput {
        ProfileInput {
            name: name.into(),
            description: None,
            runtime_id: "runtime-1".into(),
            model_id: "model-1".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            auto_select_port: true,
            options: BTreeMap::new(),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
        }
    }

    fn target() -> ResolvedProfileTarget {
        ResolvedProfileTarget {
            runtime_label: "master @ abc1234 · CUDA".into(),
            model_name: "Tiny".into(),
            model_path: PathBuf::from("/models/tiny.gguf"),
            projector_path: None,
        }
    }

    #[test]
    fn persists_updates_lists_and_deletes_one_json_file_per_profile() {
        let temp = tempfile::tempdir().expect("temp directory");
        let repository = ProfileRepository::new(temp.path().to_path_buf());
        let created = repository.create(input("First"), target()).expect("create");
        assert_eq!(repository.list().expect("list").len(), 1);
        assert!(temp.path().join(format!("{}.json", created.id)).is_file());

        let updated = repository
            .update(&created.id, input("Renamed"), target())
            .expect("update");
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.created_at, created.created_at);
        assert_eq!(repository.find(&created.id).expect("find").name, "Renamed");

        repository.delete(&created.id).expect("delete");
        assert!(repository.list().expect("list").is_empty());
        assert_eq!(
            repository.find(&created.id).expect_err("removed").code,
            ErrorCode::ProfileNotFound
        );
    }
}
