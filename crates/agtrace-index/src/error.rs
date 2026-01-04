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
            Error::Database(err) => {
                let msg = err.to_string();
                // Detect schema mismatch errors and provide actionable hint
                if msg.contains("no such column") || msg.contains("no such table") {
                    write!(
                        f,
                        "Database schema mismatch: {}. Please restart the CLI to auto-migrate.",
                        msg
                    )
                } else {
                    write!(f, "Database error: {}", err)
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_mismatch_error_message() {
        // Simulate a "no such column" error
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("no such column: parent_session_id".to_string()),
        );
        let err = Error::Database(sqlite_err);
        let msg = err.to_string();

        assert!(msg.contains("Database schema mismatch"));
        assert!(msg.contains("Please restart the CLI to auto-migrate"));
    }

    #[test]
    fn test_regular_database_error_message() {
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("UNIQUE constraint failed".to_string()),
        );
        let err = Error::Database(sqlite_err);
        let msg = err.to_string();

        assert!(msg.starts_with("Database error:"));
        assert!(!msg.contains("restart"));
    }
}
