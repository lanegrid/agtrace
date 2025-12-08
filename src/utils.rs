use sha2::{Digest, Sha256};

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
