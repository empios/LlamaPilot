use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::paths::validate_absolute_path;
use crate::config::JsonStore;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::gguf::{parse_split_filename, read_metadata, GgufMetadata};

const CACHE_SCHEMA_VERSION: u32 = 1;
const OVERRIDES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalog {
    pub scanned_at: DateTime<Utc>,
    pub roots: Vec<PathBuf>,
    pub models: Vec<ModelRecord>,
    pub projectors: Vec<ProjectorRecord>,
    pub issues: Vec<ModelScanIssue>,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRecord {
    pub id: String,
    pub display_name: String,
    pub role: ModelRole,
    /// The first shard. This is the only path that may later be passed to `llama-server -m`.
    pub primary_path: Option<PathBuf>,
    pub directory: PathBuf,
    pub shards: Vec<ModelShard>,
    pub expected_shards: u32,
    pub complete: bool,
    pub total_size_bytes: u64,
    pub modified_at: DateTime<Utc>,
    pub metadata: GgufMetadata,
    pub projector_status: ProjectorStatus,
    pub projector_id: Option<String>,
    pub projector_candidates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelRole {
    Main,
    Drafter,
}

impl ModelRecord {
    pub fn is_drafter(&self) -> bool {
        self.role == ModelRole::Drafter
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelShard {
    pub path: PathBuf,
    pub index: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectorRecord {
    pub id: String,
    pub display_name: String,
    pub path: PathBuf,
    pub directory: PathBuf,
    pub size_bytes: u64,
    pub modified_at: DateTime<Utc>,
    pub metadata: GgufMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectorStatus {
    None,
    Auto,
    Ambiguous,
    Overridden,
    Disabled,
    MissingOverride,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelScanIssue {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ProjectorSelection {
    Auto,
    Disabled,
    Custom { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
enum ProjectorOverride {
    Disabled,
    Custom { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ModelOverrides {
    schema_version: u32,
    projectors: BTreeMap<String, ProjectorOverride>,
}

impl Default for ModelOverrides {
    fn default() -> Self {
        Self {
            schema_version: OVERRIDES_SCHEMA_VERSION,
            projectors: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ModelMetadataCache {
    schema_version: u32,
    entries: BTreeMap<String, CachedMetadata>,
}

impl Default for ModelMetadataCache {
    fn default() -> Self {
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedMetadata {
    path: PathBuf,
    size_bytes: u64,
    modified_unix_nanos: u64,
    metadata: GgufMetadata,
}

#[derive(Debug, Clone)]
struct ParsedFile {
    path: PathBuf,
    directory: PathBuf,
    filename: String,
    size_bytes: u64,
    modified_at: DateTime<Utc>,
    metadata: GgufMetadata,
}

#[derive(Debug)]
struct ModelGroup {
    id: String,
    display_fallback: String,
    directory: PathBuf,
    expected: u32,
    shards: BTreeMap<u32, ParsedFile>,
}

/// Thread-safe scanner with independent persistent stores for disposable cache data and user
/// projector choices.
pub struct ModelCatalogService {
    cache: JsonStore<ModelMetadataCache>,
    overrides: JsonStore<ModelOverrides>,
    scan_lock: Mutex<()>,
}

impl ModelCatalogService {
    pub fn load(cache_file: PathBuf, overrides_file: PathBuf) -> AppResult<Self> {
        Ok(Self {
            cache: JsonStore::load(cache_file)?,
            overrides: JsonStore::load(overrides_file)?,
            scan_lock: Mutex::new(()),
        })
    }

    pub fn scan(&self, roots: &[PathBuf]) -> AppResult<ModelCatalog> {
        let _guard = self
            .scan_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut issues = Vec::new();
        let mut paths = discover_gguf_files(roots, &mut issues);
        let overrides = self.overrides.get();

        // A manually selected projector remains inspectable even when it lives outside a scan
        // root. This makes the override real instead of merely a choice among auto-detected files.
        for selection in overrides.projectors.values() {
            if let ProjectorOverride::Custom { path } = selection {
                if path.is_file() {
                    paths.insert(path.clone());
                }
            }
        }

        let cached = self.cache.get();
        let mut next_entries = BTreeMap::new();
        let mut parsed = Vec::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        for path in paths {
            match inspect_file(&path, &cached.entries) {
                Ok((file, entry, was_cached)) => {
                    if was_cached {
                        cache_hits += 1;
                    } else {
                        cache_misses += 1;
                    }
                    next_entries.insert(path_key(&path), entry);
                    parsed.push(file);
                }
                Err(error) => {
                    cache_misses += 1;
                    issues.push(ModelScanIssue {
                        path,
                        message: error.to_string(),
                    });
                }
            }
        }

        self.cache.replace(ModelMetadataCache {
            schema_version: CACHE_SCHEMA_VERSION,
            entries: next_entries,
        })?;

        let mut projectors: Vec<ProjectorRecord> = parsed
            .iter()
            .filter(|file| is_projector(file))
            .map(projector_record)
            .collect();
        projectors.sort_by(|left, right| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
                .then_with(|| left.path.cmp(&right.path))
        });

        let model_files = parsed
            .into_iter()
            .filter(is_regular_model)
            .collect::<Vec<_>>();
        let mut models = group_models(model_files, &mut issues);
        pair_projectors(&mut models, &projectors, &overrides, &mut issues);
        models.sort_by(|left, right| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(ModelCatalog {
            scanned_at: Utc::now(),
            roots: roots.to_vec(),
            models,
            projectors,
            issues,
            cache_hits,
            cache_misses,
        })
    }

    pub fn set_projector(&self, model_id: String, selection: ProjectorSelection) -> AppResult<()> {
        let model_path = PathBuf::from(&model_id);
        validate_absolute_path(&model_path, "The model")?;

        let persisted = match selection {
            ProjectorSelection::Auto => None,
            ProjectorSelection::Disabled => Some(ProjectorOverride::Disabled),
            ProjectorSelection::Custom { path } => {
                let path = validate_projector_path(&path)?;
                Some(ProjectorOverride::Custom { path })
            }
        };

        self.overrides.update(|overrides| {
            overrides.schema_version = OVERRIDES_SCHEMA_VERSION;
            if let Some(selection) = persisted {
                overrides.projectors.insert(model_id, selection);
            } else {
                overrides.projectors.remove(&model_id);
            }
        })?;
        Ok(())
    }
}

fn validate_projector_path(path: &Path) -> AppResult<PathBuf> {
    let path = validate_absolute_path(path, "The projector")?;
    if !path.is_file() {
        return Err(AppError::invalid_path(
            "The selected projector does not exist or is not a file.",
        )
        .with_details(path.display().to_string()));
    }
    if !has_gguf_extension(&path) {
        return Err(AppError::new(
            ErrorCode::InvalidGguf,
            "The selected projector is not a .gguf file.",
        ));
    }
    let metadata = read_metadata(&path).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidGguf,
            "The selected projector has invalid GGUF metadata.",
        )
        .with_details(error.to_string())
    })?;
    let file = ParsedFile {
        directory: path.parent().unwrap_or(Path::new("")).to_path_buf(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        path: path.clone(),
        size_bytes: 0,
        modified_at: DateTime::<Utc>::from(UNIX_EPOCH),
        metadata,
    };
    if !is_projector(&file) {
        return Err(AppError::new(
            ErrorCode::InvalidGguf,
            "The selected GGUF is a model, not a multimodal projector.",
        )
        .with_hint("Choose a file identified as mmproj."));
    }
    Ok(path)
}

fn discover_gguf_files(roots: &[PathBuf], issues: &mut Vec<ModelScanIssue>) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    for root in roots {
        if validate_absolute_path(root, "Model directory").is_err() || !root.is_dir() {
            issues.push(ModelScanIssue {
                path: root.clone(),
                message: "Model directory does not exist or is not accessible.".to_string(),
            });
            continue;
        }

        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    issues.push(ModelScanIssue {
                        path: directory,
                        message: format!("Could not read directory: {error}"),
                    });
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        issues.push(ModelScanIssue {
                            path: directory.clone(),
                            message: format!("Could not inspect a directory entry: {error}"),
                        });
                        continue;
                    }
                };
                let path = entry.path();
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() && !kind.is_symlink() => pending.push(path),
                    Ok(kind) if kind.is_file() && has_gguf_extension(&path) => {
                        files.insert(path);
                    }
                    Ok(_) => {}
                    Err(error) => issues.push(ModelScanIssue {
                        path,
                        message: format!("Could not inspect file type: {error}"),
                    }),
                }
            }
        }
    }
    files
}

fn inspect_file(
    path: &Path,
    cache: &BTreeMap<String, CachedMetadata>,
) -> Result<(ParsedFile, CachedMetadata, bool), AppError> {
    let file_metadata = fs::metadata(path).map_err(|error| {
        AppError::new(ErrorCode::Io, "Could not inspect GGUF file.").with_details(error.to_string())
    })?;
    let size_bytes = file_metadata.len();
    let modified = file_metadata.modified().unwrap_or(UNIX_EPOCH);
    let modified_unix_nanos = system_time_nanos(modified);
    let key = path_key(path);
    let (metadata, was_cached) = match cache.get(&key) {
        Some(entry)
            if entry.size_bytes == size_bytes
                && entry.modified_unix_nanos == modified_unix_nanos =>
        {
            (entry.metadata.clone(), true)
        }
        _ => (
            read_metadata(path).map_err(|error| {
                AppError::new(ErrorCode::InvalidGguf, "Could not read GGUF metadata.")
                    .with_details(error.to_string())
            })?,
            false,
        ),
    };

    let entry = CachedMetadata {
        path: path.to_path_buf(),
        size_bytes,
        modified_unix_nanos,
        metadata: metadata.clone(),
    };
    let parsed = ParsedFile {
        path: path.to_path_buf(),
        directory: path.parent().unwrap_or(Path::new("")).to_path_buf(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        size_bytes,
        modified_at: DateTime::<Utc>::from(modified),
        metadata,
    };
    Ok((parsed, entry, was_cached))
}

fn is_projector(file: &ParsedFile) -> bool {
    file.metadata
        .general_type
        .as_deref()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("mmproj"))
        || file.filename.to_ascii_lowercase().contains("mmproj")
        || file.metadata.projector_type.is_some()
        || file.metadata.has_vision_encoder.is_some()
        || file.metadata.has_audio_encoder.is_some()
}

fn is_regular_model(file: &ParsedFile) -> bool {
    if is_projector(file) {
        return false;
    }
    file.metadata
        .general_type
        .as_deref()
        .is_none_or(|kind| kind.eq_ignore_ascii_case("model"))
}

fn projector_record(file: &ParsedFile) -> ProjectorRecord {
    ProjectorRecord {
        id: path_key(&file.path),
        display_name: file
            .metadata
            .name
            .clone()
            .unwrap_or_else(|| file.filename.clone()),
        path: file.path.clone(),
        directory: file.directory.clone(),
        size_bytes: file.size_bytes,
        modified_at: file.modified_at,
        metadata: file.metadata.clone(),
    }
}

fn group_models(files: Vec<ParsedFile>, issues: &mut Vec<ModelScanIssue>) -> Vec<ModelRecord> {
    let mut groups = BTreeMap::<String, ModelGroup>::new();
    for file in files {
        let split = parse_split_filename(&file.filename);
        let (logical_name, index, expected) = match split {
            Some(split) => {
                if file
                    .metadata
                    .split_count
                    .is_some_and(|count| count != split.count)
                {
                    issues.push(ModelScanIssue {
                        path: file.path.clone(),
                        message: "Shard count in the filename disagrees with GGUF metadata."
                            .to_string(),
                    });
                }
                if file
                    .metadata
                    .split_index
                    .is_some_and(|index| index.saturating_add(1) != split.index)
                {
                    issues.push(ModelScanIssue {
                        path: file.path.clone(),
                        message: "Shard index in the filename disagrees with GGUF metadata."
                            .to_string(),
                    });
                }
                (split.prefix, split.index, split.count)
            }
            None => {
                let expected = file.metadata.split_count.unwrap_or(1).max(1);
                let index = file
                    .metadata
                    .split_index
                    .map(|index| index.saturating_add(1))
                    .unwrap_or(1);
                let name = file
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                (name, index, expected)
            }
        };
        let logical_path = file.directory.join(format!("{logical_name}.gguf"));
        let id = logical_path.to_string_lossy().into_owned();
        let key = comparison_key(&logical_path);
        let group = groups.entry(key).or_insert_with(|| ModelGroup {
            id,
            display_fallback: logical_name,
            directory: file.directory.clone(),
            expected,
            shards: BTreeMap::new(),
        });
        group.expected = group.expected.max(expected);
        if group.shards.insert(index, file).is_some() {
            issues.push(ModelScanIssue {
                path: logical_path,
                message: format!("More than one file claims to be shard {index}."),
            });
        }
    }

    groups
        .into_values()
        .map(|group| {
            let metadata_source = group
                .shards
                .get(&1)
                .or_else(|| group.shards.values().next())
                .expect("model groups are never empty");
            let metadata = metadata_source.metadata.clone();
            let display_name = metadata
                .name
                .clone()
                .unwrap_or_else(|| group.display_fallback.clone());
            let total_size_bytes = group.shards.values().map(|file| file.size_bytes).sum();
            let modified_at = group
                .shards
                .values()
                .map(|file| file.modified_at)
                .max()
                .unwrap_or_else(Utc::now);
            let primary_path = group.shards.get(&1).map(|file| file.path.clone());
            let complete = primary_path.is_some()
                && (1..=group.expected).all(|index| group.shards.contains_key(&index));
            let shards = group
                .shards
                .into_iter()
                .map(|(index, file)| ModelShard {
                    path: file.path,
                    index,
                    size_bytes: file.size_bytes,
                })
                .collect();
            ModelRecord {
                id: group.id,
                display_name,
                role: classify_model_role(&metadata, &group.display_fallback),
                primary_path,
                directory: group.directory,
                shards,
                expected_shards: group.expected,
                complete,
                total_size_bytes,
                modified_at,
                metadata,
                projector_status: ProjectorStatus::None,
                projector_id: None,
                projector_candidates: Vec::new(),
            }
        })
        .collect()
}

fn pair_projectors(
    models: &mut [ModelRecord],
    projectors: &[ProjectorRecord],
    overrides: &ModelOverrides,
    issues: &mut Vec<ModelScanIssue>,
) {
    for model in models.iter_mut().filter(|model| !model.is_drafter()) {
        if let Some(selection) = overrides.projectors.get(&model.id) {
            match selection {
                ProjectorOverride::Disabled => {
                    model.projector_status = ProjectorStatus::Disabled;
                    continue;
                }
                ProjectorOverride::Custom { path } => {
                    let selected = projectors
                        .iter()
                        .find(|projector| comparison_key(&projector.path) == comparison_key(path));
                    if let Some(projector) = selected {
                        model.projector_status = ProjectorStatus::Overridden;
                        model.projector_id = Some(projector.id.clone());
                    } else {
                        model.projector_status = ProjectorStatus::MissingOverride;
                        issues.push(ModelScanIssue {
                            path: path.clone(),
                            message: format!(
                                "The saved projector for {} is missing or no longer identifies as mmproj.",
                                model.display_name
                            ),
                        });
                    }
                    continue;
                }
            }
        }

        let candidates: Vec<_> = projectors
            .iter()
            .filter(|projector| {
                comparison_key(&projector.directory) == comparison_key(&model.directory)
                    && identities_compatible(model, projector)
            })
            .collect();
        model.projector_candidates = candidates
            .iter()
            .map(|projector| projector.id.clone())
            .collect();
        match candidates.as_slice() {
            [projector] => {
                model.projector_status = ProjectorStatus::Auto;
                model.projector_id = Some(projector.id.clone());
            }
            [] => model.projector_status = ProjectorStatus::None,
            _ => model.projector_status = ProjectorStatus::Ambiguous,
        }
    }
}

fn classify_model_role(metadata: &GgufMetadata, filename: &str) -> ModelRole {
    if metadata
        .architecture
        .as_deref()
        .is_some_and(|architecture| {
            let architecture = architecture.to_ascii_lowercase();
            architecture.ends_with("-assistant") || architecture.ends_with("_assistant")
        })
    {
        return ModelRole::Drafter;
    }

    let filename = filename.to_ascii_lowercase();
    if filename.starts_with("mtp-")
        || filename.starts_with("draft-")
        || filename.starts_with("drafter-")
        || filename.contains("-drafter-")
        || filename.contains("dflash")
        || filename.contains("dspark")
        || filename.contains("eagle3")
        || filename.contains("eagle-3")
    {
        ModelRole::Drafter
    } else {
        ModelRole::Main
    }
}

fn identities_compatible(model: &ModelRecord, projector: &ProjectorRecord) -> bool {
    let model_metadata_identities = identities(
        model.metadata.basename.as_deref(),
        model.metadata.name.as_deref(),
        None,
    );
    let projector_metadata_identities = identities(
        projector.metadata.basename.as_deref(),
        projector.metadata.name.as_deref(),
        None,
    );
    if !model_metadata_identities.is_empty() && !projector_metadata_identities.is_empty() {
        return any_identity_matches(&model_metadata_identities, &projector_metadata_identities);
    }

    let model_identities = identities(
        model.metadata.basename.as_deref(),
        model.metadata.name.as_deref(),
        model.primary_path.as_deref(),
    );
    let projector_identities = identities(
        projector.metadata.basename.as_deref(),
        projector.metadata.name.as_deref(),
        Some(&projector.path),
    );
    any_identity_matches(&model_identities, &projector_identities)
}

fn any_identity_matches(left: &[String], right: &[String]) -> bool {
    left.iter().any(|model_identity| {
        right.iter().any(|projector_identity| {
            model_identity == projector_identity
                || (model_identity.len().min(projector_identity.len()) >= 6
                    && (model_identity.contains(projector_identity)
                        || projector_identity.contains(model_identity)))
        })
    })
}

fn identities(basename: Option<&str>, name: Option<&str>, path: Option<&Path>) -> Vec<String> {
    let mut values = Vec::new();
    for raw in basename.into_iter().chain(name).chain(
        path.and_then(Path::file_stem)
            .and_then(|stem| stem.to_str()),
    ) {
        let normalized = normalize_identity(raw);
        if !normalized.is_empty() && !values.contains(&normalized) {
            values.push(normalized);
        }
    }
    values
}

fn normalize_identity(value: &str) -> String {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| {
            let lower = token.to_ascii_lowercase();
            lower != "mmproj" && !is_quantization_token(&lower) && !looks_like_shard_number(&lower)
        })
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("")
}

fn is_quantization_token(token: &str) -> bool {
    matches!(token, "f16" | "f32" | "bf16" | "fp16" | "fp32" | "gguf")
        || token.strip_prefix('q').is_some_and(|rest| {
            rest.chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit())
        })
        || token.strip_prefix("iq").is_some_and(|rest| {
            rest.chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit())
        })
        || token.strip_prefix("tq").is_some_and(|rest| {
            rest.chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit())
        })
}

fn looks_like_shard_number(token: &str) -> bool {
    token.len() == 5 && token.bytes().all(|byte| byte.is_ascii_digit())
}

fn has_gguf_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn comparison_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn system_time_nanos(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| duration.as_nanos().try_into().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn write_string(buffer: &mut Vec<u8>, value: &str) {
        buffer.extend_from_slice(&(value.len() as u64).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
    }

    fn write_gguf(path: &Path, entries: &[(&str, &str)]) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i64.to_le_bytes());
        bytes.extend_from_slice(&(entries.len() as i64).to_le_bytes());
        for (key, value) in entries {
            write_string(&mut bytes, key);
            bytes.extend_from_slice(&8_u32.to_le_bytes());
            write_string(&mut bytes, value);
        }
        let mut file = fs::File::create(path).expect("create fixture");
        file.write_all(&bytes).expect("write fixture");
    }

    fn service(temp: &TempDir) -> ModelCatalogService {
        ModelCatalogService::load(
            temp.path().join("cache.json"),
            temp.path().join("overrides.json"),
        )
        .expect("service")
    }

    #[test]
    fn recursively_scans_groups_shards_and_reuses_cache() {
        let temp = tempfile::tempdir().expect("temp directory");
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).expect("nested directory");
        write_gguf(
            &nested.join("vision-model-00001-of-00002.gguf"),
            &[("general.type", "model"), ("general.name", "Vision Model")],
        );
        write_gguf(
            &nested.join("vision-model-00002-of-00002.gguf"),
            &[("general.type", "model"), ("general.name", "Vision Model")],
        );
        write_gguf(
            &nested.join("mmproj-vision-model-f16.gguf"),
            &[("general.type", "mmproj"), ("general.name", "Vision Model")],
        );
        fs::write(nested.join("broken.gguf"), b"broken").expect("broken fixture");

        let service = service(&temp);
        let first = service.scan(&[temp.path().to_path_buf()]).expect("scan");
        assert_eq!(first.models.len(), 1);
        assert_eq!(first.models[0].role, ModelRole::Main);
        assert_eq!(first.models[0].shards.len(), 2);
        assert_eq!(first.models[0].expected_shards, 2);
        assert!(first.models[0].complete);
        assert!(first.models[0]
            .primary_path
            .as_ref()
            .is_some_and(|path| path.ends_with("vision-model-00001-of-00002.gguf")));
        assert_eq!(first.models[0].projector_status, ProjectorStatus::Auto);
        assert_eq!(first.projectors.len(), 1);
        assert_eq!(first.issues.len(), 1);
        assert_eq!(first.cache_hits, 0);
        assert_eq!(first.cache_misses, 4);

        let second = service.scan(&[temp.path().to_path_buf()]).expect("rescan");
        assert_eq!(second.cache_hits, 3);
        assert_eq!(second.cache_misses, 1);
    }

    #[test]
    fn classifies_assistant_architectures_as_drafters() {
        let temp = tempfile::tempdir().expect("temp directory");
        write_gguf(
            &temp.path().join("mtp-gemma-4-31b.gguf"),
            &[
                ("general.type", "model"),
                ("general.name", "31B Assistant"),
                ("general.architecture", "gemma4-assistant"),
            ],
        );

        let catalog = service(&temp)
            .scan(&[temp.path().to_path_buf()])
            .expect("scan");
        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].role, ModelRole::Drafter);
        assert_eq!(catalog.models[0].projector_status, ProjectorStatus::None);
    }

    #[test]
    fn classifies_named_dflash_models_as_drafters() {
        let temp = tempfile::tempdir().expect("temp directory");
        write_gguf(
            &temp.path().join("DFlash-Gemma-4-31B.gguf"),
            &[
                ("general.type", "model"),
                ("general.name", "Gemma 4 speculative model"),
                ("general.architecture", "gemma4"),
            ],
        );

        let catalog = service(&temp)
            .scan(&[temp.path().to_path_buf()])
            .expect("scan");
        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].role, ModelRole::Drafter);
    }

    #[test]
    fn does_not_auto_pair_an_obviously_different_projector() {
        let temp = tempfile::tempdir().expect("temp directory");
        write_gguf(
            &temp.path().join("alpha-q4.gguf"),
            &[("general.type", "model"), ("general.name", "Alpha")],
        );
        write_gguf(
            &temp.path().join("mmproj-beta-f16.gguf"),
            &[("general.type", "mmproj"), ("general.name", "Beta")],
        );

        let catalog = service(&temp)
            .scan(&[temp.path().to_path_buf()])
            .expect("scan");
        assert_eq!(catalog.models[0].projector_status, ProjectorStatus::None);
        assert!(catalog.models[0].projector_id.is_none());
    }

    #[test]
    fn incompatible_metadata_wins_over_generic_matching_filenames() {
        let temp = tempfile::tempdir().expect("temp directory");
        write_gguf(
            &temp.path().join("model-q4.gguf"),
            &[("general.type", "model"), ("general.name", "Alpha")],
        );
        write_gguf(
            &temp.path().join("mmproj-model-f16.gguf"),
            &[("general.type", "mmproj"), ("general.name", "Beta")],
        );

        let catalog = service(&temp)
            .scan(&[temp.path().to_path_buf()])
            .expect("scan");
        assert_eq!(catalog.models[0].projector_status, ProjectorStatus::None);
    }

    #[test]
    fn missing_shards_stay_visible_and_multiple_projectors_stay_ambiguous() {
        let temp = tempfile::tempdir().expect("temp directory");
        write_gguf(
            &temp.path().join("alpha-00001-of-00002.gguf"),
            &[("general.type", "model"), ("general.name", "Alpha")],
        );
        for suffix in ["f16", "bf16"] {
            write_gguf(
                &temp.path().join(format!("mmproj-alpha-{suffix}.gguf")),
                &[("general.type", "mmproj"), ("general.name", "Alpha")],
            );
        }

        let catalog = service(&temp)
            .scan(&[temp.path().to_path_buf()])
            .expect("scan");
        let model = &catalog.models[0];
        assert!(!model.complete);
        assert_eq!(model.shards.len(), 1);
        assert!(model.primary_path.is_some());
        assert_eq!(model.projector_status, ProjectorStatus::Ambiguous);
        assert_eq!(model.projector_candidates.len(), 2);
        assert!(model.projector_id.is_none());
    }

    #[test]
    fn a_manual_projector_can_live_outside_scan_roots_and_can_be_disabled() {
        let temp = tempfile::tempdir().expect("temp directory");
        let root = temp.path().join("models");
        let other = temp.path().join("other");
        fs::create_dir(&root).expect("model root");
        fs::create_dir(&other).expect("other root");
        let model = root.join("alpha.gguf");
        let projector = other.join("mmproj-alpha.gguf");
        write_gguf(
            &model,
            &[("general.type", "model"), ("general.name", "Alpha")],
        );
        write_gguf(
            &projector,
            &[("general.type", "mmproj"), ("general.name", "Alpha")],
        );

        let service = service(&temp);
        service
            .set_projector(
                model.to_string_lossy().into_owned(),
                ProjectorSelection::Custom {
                    path: projector.clone(),
                },
            )
            .expect("override");
        let selected = service.scan(std::slice::from_ref(&root)).expect("scan");
        assert_eq!(
            selected.models[0].projector_status,
            ProjectorStatus::Overridden
        );
        assert_eq!(selected.projectors[0].path, projector);

        service
            .set_projector(
                model.to_string_lossy().into_owned(),
                ProjectorSelection::Disabled,
            )
            .expect("disable");
        let disabled = service.scan(&[root]).expect("scan");
        assert_eq!(
            disabled.models[0].projector_status,
            ProjectorStatus::Disabled
        );
    }
}
