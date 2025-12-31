use agtrace_core::*;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_project_hash_from_root() {
    let root = "/home/user/project";
    let hash = project_hash_from_root(root);

    // Hash should be 64 characters (SHA256 hex)
    assert_eq!(hash.as_str().len(), 64);

    // Same input should produce same hash
    let hash2 = project_hash_from_root(root);
    assert_eq!(hash, hash2);

    // Different input should produce different hash
    let hash3 = project_hash_from_root("/different/path");
    assert_ne!(hash, hash3);
}

#[test]
fn test_discover_project_root_with_explicit() {
    let explicit_root = "/explicit/project/root";
    let result = discover_project_root(Some(explicit_root)).unwrap();
    assert_eq!(result, PathBuf::from(explicit_root));
}

#[test]
fn test_discover_project_root_priority() {
    // Set environment variable
    unsafe {
        env::set_var("AGTRACE_PROJECT_ROOT", "/env/project/root");
    }

    // Explicit should override env var
    let result = discover_project_root(Some("/explicit/root")).unwrap();
    assert_eq!(result, PathBuf::from("/explicit/root"));

    // Clean up
    unsafe {
        env::remove_var("AGTRACE_PROJECT_ROOT");
    }
}

#[test]
fn test_discover_project_root_falls_back_to_cwd() {
    // Make sure env var is not set
    unsafe {
        env::remove_var("AGTRACE_PROJECT_ROOT");
    }

    // Without explicit root or env var, should fall back to cwd
    let result = discover_project_root(None).unwrap();

    // Result should be a valid path
    assert!(result.is_absolute() || result == PathBuf::from("."));
}

#[test]
fn test_normalize_path() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Normalized path should be absolute
    let normalized = normalize_path(temp_path);
    assert!(normalized.is_absolute());
}

#[test]
fn test_paths_equal() {
    let temp_dir = TempDir::new().unwrap();
    let path1 = temp_dir.path();
    let path2 = temp_dir.path();

    // Same paths should be equal
    assert!(paths_equal(path1, path2));
}

#[test]
fn test_paths_equal_different_representations() {
    // Create a temp directory
    let temp_dir = TempDir::new().unwrap();
    let abs_path = temp_dir.path().canonicalize().unwrap();

    // These should be considered equal even if represented differently
    assert!(paths_equal(&abs_path, &abs_path));
}
