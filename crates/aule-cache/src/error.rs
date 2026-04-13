use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Hook script not found: {0}")]
    HookNotFound(String),

    #[error("Hook execution failed: {0}")]
    HookExecution(String),
}
