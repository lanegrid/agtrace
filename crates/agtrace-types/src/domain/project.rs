use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Project identifier computed from canonical project root path via SHA256
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectHash(String);

impl ProjectHash {
    /// Create a new ProjectHash from a string (typically hex digest)
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    /// Get the hash as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ProjectHash {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProjectHash {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ProjectHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Canonical project root path
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectRoot(PathBuf);

impl ProjectRoot {
    /// Create a new ProjectRoot from a path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Get the root path as a Path reference
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Get the path as a string (lossy UTF-8 conversion)
    pub fn to_string_lossy(&self) -> String {
        self.0.to_string_lossy().to_string()
    }
}

impl fmt::Display for ProjectRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl From<PathBuf> for ProjectRoot {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<&Path> for ProjectRoot {
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

impl AsRef<Path> for ProjectRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Project scope for indexing and filtering sessions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectScope {
    /// Scan all projects without filtering
    All,
    /// Scan specific project by hash
    Specific(ProjectHash),
}

impl ProjectScope {
    /// Get optional project hash for filtering
    pub fn hash(&self) -> Option<&ProjectHash> {
        match self {
            ProjectScope::All => None,
            ProjectScope::Specific(hash) => Some(hash),
        }
    }
}
