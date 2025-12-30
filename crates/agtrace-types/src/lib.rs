pub mod models;
mod util;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

pub use models::*;
pub use util::*;

/// Git repository context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
}

/// Execution environment context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

/// Agent control policy and constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
}

/// Source of the agent log (provider-agnostic identifier)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Source(String);

impl Source {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

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

    /// Compute ProjectHash from a project root path
    pub fn from_root(project_root: &str) -> Self {
        Self(project_hash_from_root(project_root))
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

    /// Compute the project hash for this root
    pub fn compute_hash(&self) -> ProjectHash {
        ProjectHash::from_root(&self.0.to_string_lossy())
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

/// Tool execution status (used in Span API)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    InProgress,
    Unknown,
}

/// Session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub source: Source,
    pub project_hash: ProjectHash,
    pub start_ts: String,
    pub end_ts: String,
    pub event_count: usize,
    pub user_message_count: usize,
    pub tokens_input_total: u64,
    pub tokens_output_total: u64,
}
