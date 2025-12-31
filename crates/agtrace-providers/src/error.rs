use std::fmt;

/// Result type for agtrace-providers operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in the providers layer
#[derive(Debug)]
pub enum Error {
    /// IO operation failed
    Io(std::io::Error),

    /// JSON parsing failed
    Json(serde_json::Error),

    /// Provider not found or detection failed
    Provider(String),

    /// Session parsing failed (missing required fields, invalid format, etc.)
    Parse(String),

    /// Walkdir error
    WalkDir(walkdir::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::Json(err) => write!(f, "JSON error: {}", err),
            Error::Provider(msg) => write!(f, "Provider error: {}", msg),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::WalkDir(err) => write!(f, "Directory traversal error: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Json(err) => Some(err),
            Error::WalkDir(err) => Some(err),
            Error::Provider(_) | Error::Parse(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        Error::WalkDir(err)
    }
}
