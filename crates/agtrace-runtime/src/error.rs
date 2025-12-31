use std::fmt;

/// Result type for agtrace-runtime operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in the runtime layer
#[derive(Debug)]
pub enum Error {
    /// Database/index layer error
    Index(agtrace_index::Error),

    /// Provider layer error
    Provider(agtrace_providers::Error),

    /// IO operation failed
    Io(std::io::Error),

    /// Configuration error
    Config(String),

    /// Workspace not initialized
    NotInitialized(String),

    /// Invalid operation or state
    InvalidOperation(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Index(err) => write!(f, "Index error: {}", err),
            Error::Provider(err) => write!(f, "Provider error: {}", err),
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::Config(msg) => write!(f, "Configuration error: {}", msg),
            Error::NotInitialized(msg) => write!(f, "Workspace not initialized: {}", msg),
            Error::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Index(err) => Some(err),
            Error::Provider(err) => Some(err),
            Error::Io(err) => Some(err),
            Error::Config(_) | Error::NotInitialized(_) | Error::InvalidOperation(_) => None,
        }
    }
}

impl From<agtrace_index::Error> for Error {
    fn from(err: agtrace_index::Error) -> Self {
        Error::Index(err)
    }
}

impl From<agtrace_providers::Error> for Error {
    fn from(err: agtrace_providers::Error) -> Self {
        Error::Provider(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Config(err.to_string())
    }
}
