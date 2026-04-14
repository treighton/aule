pub mod activate;
pub mod adapters;
pub mod build;
pub mod infer;
pub mod init;
pub mod install;
pub mod list;
pub mod login;
pub mod logout;
pub mod migrate;
pub mod publish;
pub mod search;
pub mod validate;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    User(String),

    #[error("{0}")]
    Internal(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::User(_) => 1,
            CliError::Internal(_) => 2,
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Internal(e.to_string())
    }
}
