use std::fmt;

/// Result type for agtrace-index operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in the index layer
#[derive(Debug)]
pub enum Error {
    /// Database operation failed
    Database(rusqlite::Error),

    /// IO operation failed
    Io(std::io::Error),

    /// Query-specific error (invalid input, not found, etc.)
    Query(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Database(err) => write!(f, "Database error: {}", err),
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::Query(msg) => write!(f, "Query error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(err) => Some(err),
            Error::Io(err) => Some(err),
            Error::Query(_) => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Database(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
