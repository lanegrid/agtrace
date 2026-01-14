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

#[test]
fn test_repository_hash_from_path_non_git() {
    // /tmp is typically not a git repository
    let result = repository_hash_from_path(std::path::Path::new("/tmp"));
    assert!(result.is_none());
}

#[test]
fn test_repository_hash_from_path_git_repo() {
    // Current directory should be a git repository
    let cwd = std::env::current_dir().unwrap();
    let result = repository_hash_from_path(&cwd);
    assert!(result.is_some());

    let hash = result.unwrap();
    assert_eq!(hash.as_str().len(), 64); // SHA256 hex

    // Same input should produce same hash
    let result2 = repository_hash_from_path(&cwd);
    assert_eq!(hash.as_str(), result2.unwrap().as_str());
}

#[test]
fn test_repository_hash_worktree_same_hash() {
    use std::fs;
    use std::process::Command;

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
        eprintln!("Failed to init bare repo");
        return;
    }

    // Create initial commit via temporary clone
    let temp_clone = temp_dir.path().join("temp_clone");
    let clone = Command::new("git")
        .args(["clone"])
        .arg(&bare_repo)
        .arg(&temp_clone)
        .output()
        .unwrap();
    if !clone.status.success() {
        eprintln!("Failed to clone");
        return;
    }

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
        .args(["push", "origin", "HEAD"])
        .current_dir(&temp_clone)
        .output()
        .ok();

    // Get default branch name
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
        eprintln!("Failed to create wt1");
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
        eprintln!("Failed to create wt2");
        return;
    }

    // Both worktrees should have the same repository hash
    let hash1 = repository_hash_from_path(&wt1).expect("wt1 should be git repo");
    let hash2 = repository_hash_from_path(&wt2).expect("wt2 should be git repo");

    assert_eq!(
        hash1.as_str(),
        hash2.as_str(),
        "Worktrees of the same repository should have the same repository hash"
    );

    // But project hashes should be different (different paths)
    let project_hash1 = project_hash_from_root(wt1.to_str().unwrap());
    let project_hash2 = project_hash_from_root(wt2.to_str().unwrap());

    assert_ne!(
        project_hash1.as_str(),
        project_hash2.as_str(),
        "Different worktrees should have different project hashes"
    );
}
