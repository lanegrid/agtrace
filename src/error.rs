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
}

pub type Result<T> = std::result::Result<T, Error>;
