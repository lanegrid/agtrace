use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Calculate project_hash from project_root using SHA256
pub fn project_hash_from_root(project_root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Truncate a string to a maximum length
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "...(truncated)"
    }
}

/// Discover project root based on priority:
/// 1. explicit_project_root (--project-root flag)
/// 2. AGTRACE_PROJECT_ROOT environment variable
/// 3. Current working directory
pub fn discover_project_root(explicit_project_root: Option<&str>) -> Result<PathBuf> {
    // Priority 1: Explicit --project-root flag
    if let Some(root) = explicit_project_root {
        return Ok(PathBuf::from(root));
    }

    // Priority 2: AGTRACE_PROJECT_ROOT environment variable
    if let Ok(env_root) = std::env::var("AGTRACE_PROJECT_ROOT") {
        return Ok(PathBuf::from(env_root));
    }

    // Priority 3: Current working directory
    let cwd = std::env::current_dir()?;
    Ok(cwd)
}

/// Normalize a path for comparison (resolve to absolute, canonicalize if possible)
pub fn normalize_path(path: &Path) -> PathBuf {
    // Try to canonicalize, but if that fails (e.g., path doesn't exist),
    // just return the absolute path
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

/// Encode project_root path to Claude Code directory name format
/// Claude Code replaces both '/' and '.' with '-'
/// Example: /Users/username/projects/example-project
///          -> -Users-username-projects-example-project
pub fn encode_claude_project_dir(project_root: &Path) -> String {
    let path_str = project_root.to_string_lossy();
    let encoded = path_str
        .replace('/', "-")
        .replace('.', "-")
        .trim_start_matches('-')
        .to_string();
    format!("-{}", encoded)
}

/// Resolve effective project hash based on explicit hash or all_projects flag
pub fn resolve_effective_project_hash(
    explicit_hash: Option<&str>,
    all_projects: bool,
) -> Result<(Option<String>, bool)> {
    if let Some(hash) = explicit_hash {
        Ok((Some(hash.to_string()), false))
    } else if all_projects {
        Ok((None, true))
    } else {
        let project_root_path = discover_project_root(None)?;
        let current_project_hash = project_hash_from_root(&project_root_path.to_string_lossy());
        Ok((Some(current_project_hash), false))
    }
}

/// Check if string is 64-character hexadecimal
pub fn is_64_char_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}
