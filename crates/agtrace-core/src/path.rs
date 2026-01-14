use agtrace_types::ProjectHash;
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
///
/// Git worktree support: If the path is inside a git worktree, uses the
/// git common directory (shared .git) instead of the working directory path.
/// This ensures all worktrees of the same repository share the same project hash.
pub fn project_hash_from_root(project_root: &str) -> ProjectHash {
    let path = Path::new(project_root);

    // Check for git worktree - use common git dir as the canonical project identifier
    let hash_target = if let Some(git_common_dir) = detect_git_common_dir(path) {
        git_common_dir
    } else {
        // Normalize path to resolve symlinks and relative paths
        normalize_path(path)
    };

    let path_str = hash_target.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    ProjectHash::new(format!("{:x}", hasher.finalize()))
}

/// Detect the git common directory for worktree support.
///
/// Returns Some(path) only if the directory is part of a git worktree
/// (i.e., git-dir and git-common-dir are different).
/// Returns None for non-git directories or regular git repositories.
fn detect_git_common_dir(path: &Path) -> Option<PathBuf> {
    use std::process::Command;

    // Get git-dir and git-common-dir
    let git_dir = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .ok()?;

    let git_common_dir = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(path)
        .output()
        .ok()?;

    if !git_dir.status.success() || !git_common_dir.status.success() {
        return None;
    }

    let git_dir_str = String::from_utf8_lossy(&git_dir.stdout);
    let git_common_dir_str = String::from_utf8_lossy(&git_common_dir.stdout);

    let git_dir_path = Path::new(git_dir_str.trim());
    let git_common_dir_path = Path::new(git_common_dir_str.trim());

    // Normalize both paths before comparison (handles relative vs absolute paths)
    let git_dir_normalized = normalize_path(git_dir_path);
    let git_common_dir_normalized = normalize_path(git_common_dir_path);

    // Only return common dir if this is actually a worktree (dirs are different)
    if git_dir_normalized != git_common_dir_normalized {
        Some(git_common_dir_normalized)
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_project_hash_from_root_non_git_directory() {
        // /tmp is typically not a git repository
        let hash = project_hash_from_root("/tmp");
        assert!(!hash.as_str().is_empty());
        assert_eq!(hash.as_str().len(), 64); // SHA256 produces 64 hex chars
    }

    #[test]
    fn test_project_hash_from_root_normal_git_repo() {
        // Current directory should be a normal git repo (not a worktree)
        let cwd = std::env::current_dir().unwrap();
        let hash = project_hash_from_root(cwd.to_str().unwrap());

        // Hash should be based on the cwd, not git-common-dir
        // (since this is not a worktree)
        let expected = {
            let normalized = normalize_path(&cwd);
            let path_str = normalized.to_string_lossy();
            let mut hasher = sha2::Sha256::new();
            hasher.update(path_str.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        assert_eq!(hash.as_str(), expected);
    }

    #[test]
    fn test_detect_git_common_dir_non_git() {
        // /tmp should return None (not a git repo)
        let result = detect_git_common_dir(Path::new("/tmp"));
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_git_common_dir_normal_repo() {
        // Current directory is a normal git repo, not a worktree
        let cwd = std::env::current_dir().unwrap();
        let result = detect_git_common_dir(&cwd);
        // Normal git repo should return None (git-dir == git-common-dir after normalization)
        assert!(result.is_none());
    }

    #[test]
    fn test_git_worktree_same_hash() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary bare repository and two worktrees
        let temp_dir = TempDir::new().unwrap();
        let bare_repo = temp_dir.path().join("repo.git");
        let wt1 = temp_dir.path().join("wt1");
        let wt2 = temp_dir.path().join("wt2");

        // Initialize bare repo
        let init = Command::new("git")
            .args(["init", "--bare"])
            .arg(&bare_repo)
            .output()
            .unwrap();
        if !init.status.success() {
            eprintln!("Failed to init bare repo: {:?}", init);
            return;
        }

        // Create initial commit in a temporary clone
        let temp_clone = temp_dir.path().join("temp_clone");
        let clone = Command::new("git")
            .args(["clone"])
            .arg(&bare_repo)
            .arg(&temp_clone)
            .output()
            .unwrap();
        if !clone.status.success() {
            eprintln!("Failed to clone: {:?}", clone);
            return;
        }

        // Create initial commit
        fs::write(temp_clone.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&temp_clone)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&temp_clone)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&temp_clone)
            .output()
            .ok();
        Command::new("git")
            .args(["push", "origin", "master"])
            .current_dir(&temp_clone)
            .output()
            .ok();

        // Get the default branch name
        let branch_output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&temp_clone)
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();
        if branch.is_empty() {
            eprintln!("Could not determine branch name");
            return;
        }

        // Create worktrees
        let wt1_result = Command::new("git")
            .args(["worktree", "add"])
            .arg(&wt1)
            .arg(&branch)
            .current_dir(&bare_repo)
            .output()
            .unwrap();
        if !wt1_result.status.success() {
            eprintln!(
                "Failed to create wt1: {}",
                String::from_utf8_lossy(&wt1_result.stderr)
            );
            return;
        }

        let wt2_result = Command::new("git")
            .args(["worktree", "add", "-b", "feature"])
            .arg(&wt2)
            .arg(&branch)
            .current_dir(&bare_repo)
            .output()
            .unwrap();
        if !wt2_result.status.success() {
            eprintln!(
                "Failed to create wt2: {}",
                String::from_utf8_lossy(&wt2_result.stderr)
            );
            return;
        }

        // Both worktrees should produce the same hash
        let hash1 = project_hash_from_root(wt1.to_str().unwrap());
        let hash2 = project_hash_from_root(wt2.to_str().unwrap());

        assert_eq!(
            hash1.as_str(),
            hash2.as_str(),
            "Worktrees of the same repository should have the same project hash"
        );

        // Verify that detect_git_common_dir returns the same path for both
        let common1 = detect_git_common_dir(&wt1).expect("wt1 should be detected as worktree");
        let common2 = detect_git_common_dir(&wt2).expect("wt2 should be detected as worktree");
        assert_eq!(
            common1, common2,
            "Both worktrees should share the same git common dir"
        );
    }
}
