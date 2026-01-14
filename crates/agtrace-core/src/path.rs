use agtrace_types::{ProjectHash, RepositoryHash};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::Config(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Config(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// Resolve the workspace data directory path based on priority:
/// 1. Explicit path (with tilde expansion)
/// 2. AGTRACE_PATH environment variable (with tilde expansion)
/// 3. System data directory (recommended default)
/// 4. ~/.agtrace (fallback for systems without standard data directory)
pub fn resolve_workspace_path(explicit_path: Option<&str>) -> Result<PathBuf> {
    // Priority 1: Explicit path
    if let Some(path) = explicit_path {
        return Ok(expand_tilde(path));
    }

    // Priority 2: AGTRACE_PATH environment variable
    if let Ok(env_path) = std::env::var("AGTRACE_PATH") {
        return Ok(expand_tilde(&env_path));
    }

    // Priority 3: System data directory (recommended default)
    if let Some(data_dir) = dirs::data_dir() {
        return Ok(data_dir.join("agtrace"));
    }

    // Priority 4: Fallback to ~/.agtrace (last resort for systems without standard data directory)
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".agtrace"));
    }

    // This should never happen, but provide a working directory fallback
    Err(Error::Config(
        "Could not determine workspace path: no HOME directory or system data directory found"
            .to_string(),
    ))
}

/// Expand tilde (~) in paths to the user's home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }
    PathBuf::from(path)
}

/// Calculate project_hash from project_root using SHA256
///
/// This function canonicalizes the path before hashing to ensure consistency
/// across symlinks and different path representations.
/// For example, `/var/folders/...` and `/private/var/folders/...` will produce
/// the same hash on macOS where `/var` is a symlink to `/private/var`.
pub fn project_hash_from_root(project_root: &str) -> ProjectHash {
    // Normalize path to resolve symlinks and relative paths
    let normalized = normalize_path(Path::new(project_root));
    let path_str = normalized.to_string_lossy();

    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    ProjectHash::new(format!("{:x}", hasher.finalize()))
}

/// Normalize a path for comparison (resolve to absolute, canonicalize if possible)
pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    })
}

/// Check if two paths are equivalent after normalization
pub fn paths_equal(path1: &Path, path2: &Path) -> bool {
    normalize_path(path1) == normalize_path(path2)
}

/// Discover project root based on priority:
/// 1. explicit_project_root (--project-root flag)
/// 2. AGTRACE_PROJECT_ROOT environment variable
/// 3. Current working directory
pub fn discover_project_root(explicit_project_root: Option<&str>) -> Result<PathBuf> {
    if let Some(root) = explicit_project_root {
        return Ok(PathBuf::from(root));
    }

    if let Ok(env_root) = std::env::var("AGTRACE_PROJECT_ROOT") {
        return Ok(PathBuf::from(env_root));
    }

    let cwd = std::env::current_dir()?;
    Ok(cwd)
}

/// Resolve effective project hash based on explicit hash or all_projects flag
pub fn resolve_effective_project_hash(
    explicit_hash: Option<&ProjectHash>,
    all_projects: bool,
) -> Result<(Option<ProjectHash>, bool)> {
    if let Some(hash) = explicit_hash {
        Ok((Some(hash.clone()), false))
    } else if all_projects {
        Ok((None, true))
    } else {
        let project_root_path = discover_project_root(None)?;
        let current_project_hash = project_hash_from_root(&project_root_path.to_string_lossy());
        Ok((Some(current_project_hash), false))
    }
}

/// Generate unique project hash from log file path
/// Used only for sessions without discoverable project_root (orphaned sessions)
pub fn project_hash_from_log_path(log_path: &Path) -> ProjectHash {
    let mut hasher = Sha256::new();
    hasher.update(log_path.to_string_lossy().as_bytes());
    ProjectHash::new(format!("{:x}", hasher.finalize()))
}

/// Calculate repository hash from a directory path for git worktree support.
///
/// Returns Some(RepositoryHash) if the path is inside a git repository.
/// The hash is computed from the git common directory (shared .git),
/// so all worktrees of the same repository return the same hash.
/// Returns None for non-git directories.
pub fn repository_hash_from_path(path: &Path) -> Option<RepositoryHash> {
    use std::process::Command;

    let git_common_dir = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(path)
        .output()
        .ok()?;

    if !git_common_dir.status.success() {
        return None;
    }

    let common_dir_str = String::from_utf8_lossy(&git_common_dir.stdout);
    let common_dir_path = Path::new(common_dir_str.trim());

    // Normalize to absolute path
    let normalized = normalize_path(common_dir_path);
    let path_str = normalized.to_string_lossy();

    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    Some(RepositoryHash::new(format!("{:x}", hasher.finalize())))
}
