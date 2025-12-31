use std::fmt;

/// Result type alias for SDK operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for SDK operations.
#[derive(Debug)]
pub enum Error {
    /// Resource not found (session, project, etc.).
    NotFound(String),
    /// Invalid input parameters or configuration.
    InvalidInput(String),
    /// Error from the underlying runtime layer.
    Runtime(agtrace_runtime::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(msg) => write!(f, "Not found: {}", msg),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Error::Runtime(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Runtime(err) => Some(err),
            _ => None,
        }
    }
}

impl From<agtrace_runtime::Error> for Error {
    fn from(err: agtrace_runtime::Error) -> Self {
        Error::Runtime(err)
    }
}
