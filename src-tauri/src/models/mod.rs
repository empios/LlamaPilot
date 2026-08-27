mod catalog;
mod download;

pub use catalog::{ModelCatalog, ModelCatalogService, ProjectorSelection};
pub use download::{
    download_selection, inspect_hugging_face_repository, inspect_repository_revision,
    validate_destination, HuggingFaceRepository, ModelDownloadEvent, ModelDownloadRequest,
    ModelDownloadSupervisor,
};
