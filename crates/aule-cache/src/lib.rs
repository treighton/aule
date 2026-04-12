pub mod activation;
pub mod artifact;
pub mod config;
pub mod error;
pub mod integrity;
pub mod manager;
pub mod metadata;

pub use activation::{ActivationRecord, ActivationState};
pub use artifact::{artifact_path, install_artifact, remove_artifact};
pub use config::{PolicyConfig, UserConfig};
pub use error::CacheError;
pub use integrity::{check_integrity, IntegrityReport};
pub use manager::CacheManager;
pub use metadata::{IndexEntry, MetadataIndex};
