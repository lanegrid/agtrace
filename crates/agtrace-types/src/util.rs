use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Calculate project_hash from project_root using SHA256
///
/// This function canonicalizes the path before hashing to ensure consistency
/// across symlinks and different path representations.
/// For example, `/var/folders/...` and `/private/var/folders/...` will produce
/// the same hash on macOS where `/var` is a symlink to `/private/var`.
pub fn project_hash_from_root(project_root: &str) -> crate::ProjectHash {
    // Normalize path to resolve symlinks and relative paths
    let normalized = normalize_path(Path::new(project_root));
    let path_str = normalized.to_string_lossy();

    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    crate::ProjectHash::new(format!("{:x}", hasher.finalize()))
}

/// Check if string is 64-character hexadecimal
pub fn is_64_char_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
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
    explicit_hash: Option<&crate::ProjectHash>,
    all_projects: bool,
) -> Result<(Option<crate::ProjectHash>, bool)> {
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

/// Truncate a string to a maximum length
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "...(truncated)"
    }
}

/// Generate unique project hash from log file path
/// Used only for sessions without discoverable project_root (orphaned sessions)
pub fn project_hash_from_log_path(log_path: &Path) -> crate::ProjectHash {
    let mut hasher = Sha256::new();
    hasher.update(log_path.to_string_lossy().as_bytes());
    crate::ProjectHash::new(format!("{:x}", hasher.finalize()))
}
