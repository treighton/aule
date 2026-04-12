use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("no matching version for skill \"{name}\" with constraint \"{constraint}\"")]
    NoMatchingVersion { name: String, constraint: String },

    #[error("no compatible adapter for skill \"{name}\" targeting \"{target}\"")]
    NoCompatibleAdapter { name: String, target: String },

    #[error("permission blocked: {permission}")]
    PermissionBlocked { permission: String },

    #[error("manifest error: {0}")]
    ManifestError(String),

    #[error("git clone failed for \"{url}\": {reason}")]
    GitCloneFailed { url: String, reason: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
