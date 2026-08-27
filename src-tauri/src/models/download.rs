use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::gguf::parse_split_filename;

const HUB_ENDPOINT: &str = "https://huggingface.co";
const DEFAULT_REVISION: &str = "main";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceRepository {
    pub repository_id: String,
    pub revision: String,
    pub selections: Vec<HuggingFaceGgufSelection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceGgufSelection {
    pub id: String,
    pub display_name: String,
    pub files: Vec<HuggingFaceFile>,
    pub total_size_bytes: u64,
    pub expected_files: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadRequest {
    pub repository_id: String,
    pub revision: String,
    pub selection_id: String,
    pub destination_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ModelDownloadEvent {
    Started {
        total_files: usize,
        total_bytes: u64,
    },
    FileStarted {
        path: String,
        index: usize,
        total_files: usize,
    },
    Progress {
        downloaded_bytes: u64,
        total_bytes: u64,
        file_downloaded_bytes: u64,
        file_size_bytes: u64,
    },
    Finished {
        total_files: usize,
        total_bytes: u64,
    },
}

#[derive(Debug, Deserialize)]
struct HubModelInfo {
    sha: Option<String>,
    #[serde(default)]
    siblings: Vec<HubSibling>,
}

#[derive(Debug, Deserialize)]
struct HubSibling {
    rfilename: String,
    size: Option<u64>,
    lfs: Option<HubLfs>,
}

#[derive(Debug, Deserialize)]
struct HubLfs {
    size: Option<u64>,
}

#[derive(Default)]
pub struct ModelDownloadSupervisor {
    active: std::sync::Mutex<Option<ActiveDownload>>,
}

struct ActiveDownload {
    id: uuid::Uuid,
    cancelled: Arc<AtomicBool>,
}

pub struct ModelDownloadPermit<'a> {
    supervisor: &'a ModelDownloadSupervisor,
    id: uuid::Uuid,
    cancelled: Arc<AtomicBool>,
}

impl ModelDownloadSupervisor {
    pub fn begin(&self) -> AppResult<ModelDownloadPermit<'_>> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err(AppError::new(
                ErrorCode::ModelDownloadInProgress,
                "A model download is already running.",
            )
            .with_hint("Wait for it to finish, or cancel it first."));
        }

        let id = uuid::Uuid::new_v4();
        let cancelled = Arc::new(AtomicBool::new(false));
        *slot = Some(ActiveDownload {
            id,
            cancelled: Arc::clone(&cancelled),
        });
        Ok(ModelDownloadPermit {
            supervisor: self,
            id,
            cancelled,
        })
    }

    pub fn cancel(&self) -> bool {
        let cancelled = self
            .lock()
            .as_ref()
            .map(|active| Arc::clone(&active.cancelled));
        let Some(cancelled) = cancelled else {
            return false;
        };
        cancelled.store(true, Ordering::Release);
        true
    }

    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    fn finish(&self, id: uuid::Uuid) {
        let mut slot = self.lock();
        if slot.as_ref().is_some_and(|active| active.id == id) {
            *slot = None;
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ActiveDownload>> {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl ModelDownloadPermit<'_> {
    pub fn ensure_not_cancelled(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::ModelDownloadCancelled,
                "The model download was cancelled.",
            ));
        }
        Ok(())
    }
}

impl Drop for ModelDownloadPermit<'_> {
    fn drop(&mut self) {
        self.supervisor.finish(self.id);
    }
}

pub async fn inspect_hugging_face_repository(repository: &str) -> AppResult<HuggingFaceRepository> {
    let repository_id = normalize_repository_id(repository)?;
    inspect_repository_revision(&repository_id, DEFAULT_REVISION).await
}

pub async fn inspect_repository_revision(
    repository_id: &str,
    revision: &str,
) -> AppResult<HuggingFaceRepository> {
    let repository_id = normalize_repository_id(repository_id)?;
    let client = hub_client()?;
    let response = client
        .get(model_info_url(&repository_id, revision)?)
        .send()
        .await
        .map_err(hub_request_error)?;
    let response = checked_hub_response(response, &repository_id)?;
    let info: HubModelInfo = response.json().await.map_err(|error| {
        AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Hugging Face returned an unreadable repository response.",
        )
        .with_details(error.to_string())
    })?;
    let resolved_revision = info.sha.filter(|sha| is_commit_sha(sha)).ok_or_else(|| {
        AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Hugging Face did not return a pinned repository revision.",
        )
    })?;

    Ok(HuggingFaceRepository {
        repository_id,
        revision: resolved_revision,
        selections: group_gguf_files(info.siblings),
    })
}

pub fn validate_destination(
    destination: &Path,
    configured_roots: &[PathBuf],
) -> AppResult<PathBuf> {
    let destination = std::fs::canonicalize(destination).map_err(|error| {
        AppError::invalid_path("The selected model folder does not exist or cannot be opened.")
            .with_details(error.to_string())
    })?;
    if !destination.is_dir() {
        return Err(AppError::invalid_path(
            "The selected model destination is not a folder.",
        ));
    }

    let configured = configured_roots.iter().any(|root| {
        std::fs::canonicalize(root)
            .map(|root| root == destination)
            .unwrap_or(false)
    });
    if !configured {
        return Err(AppError::invalid_path(
            "Choose one of the model folders configured in Settings.",
        ));
    }
    Ok(destination)
}

pub async fn download_selection<F>(
    repository: &HuggingFaceRepository,
    selection_id: &str,
    destination: &Path,
    permit: &ModelDownloadPermit<'_>,
    mut send_event: F,
) -> AppResult<Vec<PathBuf>>
where
    F: FnMut(ModelDownloadEvent),
{
    permit.ensure_not_cancelled()?;
    let selection = repository
        .selections
        .iter()
        .find(|selection| selection.id == selection_id)
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::ModelDownloadFailed,
                "The selected GGUF file is no longer available in this repository revision.",
            )
            .with_hint("Load the repository files again and retry.")
        })?;
    if !selection.complete {
        return Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "The selected split GGUF is incomplete in the repository.",
        ));
    }

    let targets = target_paths(destination, &selection.files)?;
    ensure_targets_available(&targets)?;
    let staging = destination.join(format!(".llamapilot-download-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir(&staging).await.map_err(|error| {
        AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Could not prepare the model download folder.",
        )
        .with_details(error.to_string())
    })?;

    let result = download_to_staging(
        repository,
        selection,
        &staging,
        &targets,
        permit,
        &mut send_event,
    )
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_dir_all(&staging).await;
    }
    result
}

async fn download_to_staging<F>(
    repository: &HuggingFaceRepository,
    selection: &HuggingFaceGgufSelection,
    staging: &Path,
    targets: &[PathBuf],
    permit: &ModelDownloadPermit<'_>,
    send_event: &mut F,
) -> AppResult<Vec<PathBuf>>
where
    F: FnMut(ModelDownloadEvent),
{
    let total_bytes = selection.total_size_bytes;
    send_event(ModelDownloadEvent::Started {
        total_files: selection.files.len(),
        total_bytes,
    });
    let client = hub_client()?;
    let mut downloaded_bytes = 0_u64;
    let mut staged_files = Vec::with_capacity(selection.files.len());

    for (position, file) in selection.files.iter().enumerate() {
        permit.ensure_not_cancelled()?;
        send_event(ModelDownloadEvent::FileStarted {
            path: file.path.clone(),
            index: position + 1,
            total_files: selection.files.len(),
        });
        let filename = safe_filename(&file.path)?;
        let staged_path = staging.join(filename);
        let response = client
            .get(download_url(
                &repository.repository_id,
                &repository.revision,
                &file.path,
            )?)
            .send()
            .await
            .map_err(hub_request_error)?;
        let mut response = checked_download_response(response, &file.path)?;
        let mut output = tokio::fs::File::create(&staged_path)
            .await
            .map_err(|error| {
                AppError::new(
                    ErrorCode::ModelDownloadFailed,
                    "Could not create the destination model file.",
                )
                .with_details(error.to_string())
            })?;
        let mut file_downloaded_bytes = 0_u64;

        loop {
            permit.ensure_not_cancelled()?;
            let next_chunk = loop {
                tokio::select! {
                    chunk = response.chunk() => break chunk,
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {
                        permit.ensure_not_cancelled()?;
                    }
                }
            }
            .map_err(hub_request_error)?;
            let Some(chunk) = next_chunk else {
                break;
            };
            output.write_all(&chunk).await.map_err(|error| {
                AppError::new(
                    ErrorCode::ModelDownloadFailed,
                    "Writing the model file failed.",
                )
                .with_details(error.to_string())
            })?;
            let chunk_size = chunk.len() as u64;
            file_downloaded_bytes = file_downloaded_bytes.saturating_add(chunk_size);
            downloaded_bytes = downloaded_bytes.saturating_add(chunk_size);
            send_event(ModelDownloadEvent::Progress {
                downloaded_bytes,
                total_bytes,
                file_downloaded_bytes,
                file_size_bytes: file.size_bytes,
            });
        }
        output.flush().await.map_err(|error| {
            AppError::new(
                ErrorCode::ModelDownloadFailed,
                "Finalizing the downloaded model file failed.",
            )
            .with_details(error.to_string())
        })?;
        drop(output);
        if file.size_bytes > 0 && file_downloaded_bytes != file.size_bytes {
            return Err(AppError::new(
                ErrorCode::ModelDownloadFailed,
                format!("Downloaded size did not match for '{}'.", file.path),
            )
            .with_details(format!(
                "Expected {} bytes, received {} bytes.",
                file.size_bytes, file_downloaded_bytes
            )));
        }
        staged_files.push(staged_path);
    }

    permit.ensure_not_cancelled()?;
    ensure_targets_available(targets)?;
    let mut committed = Vec::with_capacity(targets.len());
    for (staged, target) in staged_files.iter().zip(targets) {
        if let Err(error) = tokio::fs::rename(staged, target).await {
            for path in &committed {
                let _ = tokio::fs::remove_file(path).await;
            }
            return Err(AppError::new(
                ErrorCode::ModelDownloadFailed,
                "Could not move the completed model into its destination folder.",
            )
            .with_details(error.to_string()));
        }
        committed.push(target.clone());
    }
    let _ = tokio::fs::remove_dir(staging).await;
    send_event(ModelDownloadEvent::Finished {
        total_files: committed.len(),
        total_bytes: downloaded_bytes,
    });
    Ok(committed)
}

fn group_gguf_files(siblings: Vec<HubSibling>) -> Vec<HuggingFaceGgufSelection> {
    #[derive(Default)]
    struct SplitGroup {
        display_name: String,
        expected: usize,
        files: Vec<(u32, HuggingFaceFile)>,
    }

    let mut standalone = Vec::new();
    let mut splits: BTreeMap<String, SplitGroup> = BTreeMap::new();
    for sibling in siblings {
        if !sibling.rfilename.to_ascii_lowercase().ends_with(".gguf") {
            continue;
        }
        let size_bytes = sibling
            .size
            .or_else(|| sibling.lfs.and_then(|lfs| lfs.size))
            .unwrap_or(0);
        let file = HuggingFaceFile {
            path: sibling.rfilename,
            size_bytes,
        };
        let filename = file.path.rsplit('/').next().unwrap_or(&file.path);
        let Some(split) = parse_split_filename(filename) else {
            standalone.push(HuggingFaceGgufSelection {
                id: file.path.clone(),
                display_name: filename.to_string(),
                total_size_bytes: file.size_bytes,
                files: vec![file],
                expected_files: 1,
                complete: true,
            });
            continue;
        };
        let directory = file.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        let key = format!("{directory}\u{1f}{}\u{1f}{}", split.prefix, split.count);
        let group = splits.entry(key).or_insert_with(|| SplitGroup {
            display_name: split.prefix.clone(),
            expected: split.count as usize,
            files: Vec::new(),
        });
        group.files.push((split.index, file));
    }

    for (_, mut group) in splits {
        group.files.sort_by_key(|(index, _)| *index);
        let indexes: BTreeSet<_> = group.files.iter().map(|(index, _)| *index).collect();
        let complete = group.files.len() == group.expected
            && (1..=group.expected as u32).all(|index| indexes.contains(&index));
        let files: Vec<_> = group.files.into_iter().map(|(_, file)| file).collect();
        let Some(id) = files.first().map(|file| file.path.clone()) else {
            continue;
        };
        standalone.push(HuggingFaceGgufSelection {
            id,
            display_name: group.display_name,
            total_size_bytes: files.iter().map(|file| file.size_bytes).sum(),
            files,
            expected_files: group.expected,
            complete,
        });
    }
    standalone.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    standalone
}

fn target_paths(destination: &Path, files: &[HuggingFaceFile]) -> AppResult<Vec<PathBuf>> {
    let mut names = BTreeSet::new();
    files
        .iter()
        .map(|file| {
            let filename = safe_filename(&file.path)?;
            if !names.insert(filename.to_ascii_lowercase()) {
                return Err(AppError::new(
                    ErrorCode::ModelDownloadFailed,
                    "The selected repository files contain duplicate destination names.",
                ));
            }
            Ok(destination.join(filename))
        })
        .collect()
}

fn safe_filename(path: &str) -> AppResult<&str> {
    if path.is_empty() || path.contains('\\') || path.split('/').any(|part| part == "..") {
        return Err(unsafe_filename_error(path));
    }
    let filename = path.rsplit('/').next().unwrap_or(path);
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.ends_with(['.', ' '])
        || filename
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
    {
        return Err(unsafe_filename_error(path));
    }
    Ok(filename)
}

fn unsafe_filename_error(path: &str) -> AppError {
    AppError::new(
        ErrorCode::ModelDownloadFailed,
        "The repository contains a GGUF filename that is unsafe on this system.",
    )
    .with_details(path.to_string())
}

fn ensure_targets_available(targets: &[PathBuf]) -> AppResult<()> {
    if let Some(existing) = targets.iter().find(|path| path.exists()) {
        return Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "A model file with the same name already exists in the destination folder.",
        )
        .with_hint("Choose another model folder or move the existing file first.")
        .with_details(existing.display().to_string()));
    }
    Ok(())
}

fn normalize_repository_id(input: &str) -> AppResult<String> {
    let trimmed = input.trim().trim_end_matches('/');
    let candidate = if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        let url = Url::parse(trimmed).map_err(|error| {
            AppError::new(
                ErrorCode::ModelDownloadFailed,
                "Enter a valid Hugging Face model URL or repository ID.",
            )
            .with_details(error.to_string())
        })?;
        if url.scheme() != "https" || url.host_str() != Some("huggingface.co") {
            return Err(AppError::new(
                ErrorCode::ModelDownloadFailed,
                "Only public model repositories on huggingface.co are supported.",
            ));
        }
        let segments: Vec<_> = url
            .path_segments()
            .map(|segments| segments.filter(|part| !part.is_empty()).collect())
            .unwrap_or_default();
        if segments.len() != 2 {
            return Err(AppError::new(
                ErrorCode::ModelDownloadFailed,
                "Use the main Hugging Face model repository URL, not a file or collection URL.",
            ));
        }
        format!("{}/{}", segments[0], segments[1])
    } else {
        trimmed.to_string()
    };

    let parts: Vec<_> = candidate.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|part| !valid_repo_part(part)) {
        return Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Enter a Hugging Face repository as owner/model-name.",
        ));
    }
    Ok(candidate)
}

fn valid_repo_part(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= 96
        && !part.starts_with(['.', '-'])
        && !part.ends_with(['.', '-'])
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_commit_sha(revision: &str) -> bool {
    revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hub_client() -> AppResult<Client> {
    Client::builder()
        .user_agent(concat!("LlamaPilot/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| {
            AppError::internal("Could not initialize HTTPS.").with_details(error.to_string())
        })
}

fn model_info_url(repository_id: &str, revision: &str) -> AppResult<Url> {
    let (owner, name) = repository_id
        .split_once('/')
        .ok_or_else(|| AppError::internal("Invalid normalized Hugging Face repository ID."))?;
    let mut url = Url::parse(HUB_ENDPOINT).expect("static Hugging Face endpoint is valid");
    url.path_segments_mut()
        .map_err(|_| AppError::internal("Could not construct the Hugging Face API URL."))?
        .extend(["api", "models", owner, name, "revision", revision]);
    url.query_pairs_mut().append_pair("blobs", "true");
    Ok(url)
}

fn download_url(repository_id: &str, revision: &str, path: &str) -> AppResult<Url> {
    let (owner, name) = repository_id
        .split_once('/')
        .ok_or_else(|| AppError::internal("Invalid normalized Hugging Face repository ID."))?;
    let mut url = Url::parse(HUB_ENDPOINT).expect("static Hugging Face endpoint is valid");
    let mut segments = url
        .path_segments_mut()
        .map_err(|_| AppError::internal("Could not construct the Hugging Face download URL."))?;
    segments.extend([owner, name, "resolve", revision]);
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(unsafe_filename_error(path));
        }
        segments.push(part);
    }
    drop(segments);
    url.query_pairs_mut().append_pair("download", "true");
    Ok(url)
}

fn checked_hub_response(
    response: reqwest::Response,
    repository_id: &str,
) -> AppResult<reqwest::Response> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "This Hugging Face repository requires access or authentication.",
        )
        .with_hint("This version currently supports public, ungated repositories only.")),
        StatusCode::NOT_FOUND => Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            format!("Hugging Face repository '{repository_id}' was not found."),
        )
        .with_hint("Check the repository ID and make sure it is public.")),
        status if !status.is_success() => Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Hugging Face could not list this repository.",
        )
        .with_details(format!("HTTP {status}"))),
        _ => Ok(response),
    }
}

fn checked_download_response(
    response: reqwest::Response,
    path: &str,
) -> AppResult<reqwest::Response> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            "Hugging Face denied access to the selected model file.",
        )
        .with_hint("This version currently supports public, ungated repositories only.")),
        status if !status.is_success() => Err(AppError::new(
            ErrorCode::ModelDownloadFailed,
            format!("Hugging Face could not download '{path}'."),
        )
        .with_details(format!("HTTP {status}"))),
        _ => Ok(response),
    }
}

fn hub_request_error(error: reqwest::Error) -> AppError {
    AppError::new(
        ErrorCode::ModelDownloadFailed,
        "The connection to Hugging Face failed.",
    )
    .with_hint("Check the network connection and try again.")
    .with_details(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sibling(path: &str, size: u64) -> HubSibling {
        HubSibling {
            rfilename: path.into(),
            size: Some(size),
            lfs: None,
        }
    }

    #[test]
    fn accepts_repository_ids_and_main_repository_urls_only() {
        assert_eq!(
            normalize_repository_id("bartowski/Qwen3-Coder-GGUF").unwrap(),
            "bartowski/Qwen3-Coder-GGUF"
        );
        assert_eq!(
            normalize_repository_id("https://huggingface.co/bartowski/Qwen3-Coder-GGUF/").unwrap(),
            "bartowski/Qwen3-Coder-GGUF"
        );
        assert!(normalize_repository_id("https://example.com/owner/model").is_err());
        assert!(normalize_repository_id("https://huggingface.co/owner/model/tree/main").is_err());
        assert!(normalize_repository_id("owner").is_err());
    }

    #[test]
    fn groups_complete_split_files_and_keeps_regular_gguf_files_separate() {
        let selections = group_gguf_files(vec![
            sibling("Q4/model-00002-of-00002.gguf", 20),
            sibling("README.md", 5),
            sibling("single.gguf", 7),
            sibling("Q4/model-00001-of-00002.gguf", 10),
        ]);

        assert_eq!(selections.len(), 2);
        let split = selections
            .iter()
            .find(|selection| selection.display_name == "model")
            .unwrap();
        assert!(split.complete);
        assert_eq!(split.expected_files, 2);
        assert_eq!(split.total_size_bytes, 30);
        assert!(split.files[0].path.ends_with("00001-of-00002.gguf"));
    }

    #[test]
    fn marks_a_split_selection_incomplete_when_a_shard_is_missing() {
        let selections = group_gguf_files(vec![sibling("model-00002-of-00003.gguf", 20)]);
        assert_eq!(selections.len(), 1);
        assert!(!selections[0].complete);
        assert_eq!(selections[0].expected_files, 3);
    }

    #[test]
    fn destination_must_be_one_of_the_configured_model_roots() {
        let configured = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        assert_eq!(
            validate_destination(configured.path(), &[configured.path().to_path_buf()]).unwrap(),
            std::fs::canonicalize(configured.path()).unwrap()
        );
        assert!(validate_destination(other.path(), &[configured.path().to_path_buf()]).is_err());
    }

    #[test]
    fn rejects_remote_paths_that_could_escape_or_break_windows_filenames() {
        for path in [
            "../model.gguf",
            "folder\\model.gguf",
            "folder/bad:name.gguf",
        ] {
            assert!(safe_filename(path).is_err(), "{path}");
        }
        assert_eq!(safe_filename("quant/model.gguf").unwrap(), "model.gguf");
    }

    #[test]
    fn cancellation_holds_the_single_download_slot_until_drop() {
        let supervisor = ModelDownloadSupervisor::default();
        let permit = supervisor.begin().unwrap();
        assert!(supervisor.cancel());
        assert_eq!(
            permit.ensure_not_cancelled().unwrap_err().code,
            ErrorCode::ModelDownloadCancelled
        );
        assert!(supervisor.begin().is_err());
        drop(permit);
        assert!(!supervisor.is_running());
    }
}
