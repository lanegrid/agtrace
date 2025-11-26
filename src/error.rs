use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Agent data not found at: {0}")]
    AgentDataNotFound(PathBuf),

    #[error("Execution not found: {0}")]
    ExecutionNotFound(String),

    #[error("Unknown agent: {0}")]
    UnknownAgent(String),

    #[error("Invalid sort field: {0}")]
    InvalidSortField(String),
}

impl Error {
    /// Get the exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::ExecutionNotFound(_) => 3,
            Error::AgentDataNotFound(_) => 2,
            Error::UnknownAgent(_) | Error::Parse(_) | Error::InvalidSortField(_) => 1,
            Error::Io(_) | Error::Json(_) => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
