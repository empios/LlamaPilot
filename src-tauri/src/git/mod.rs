pub mod refs;
pub mod remote;
pub mod repository;
pub mod runner;
pub mod status;

pub use refs::{CommitInfo, GitRef, GitRefKind, RefCheckout};
pub use remote::GitRemote;
pub use repository::{clone_repository, discover_default_branch, Repository, UpdateOutcome};
pub use runner::Git;
pub use status::GitStatus;
