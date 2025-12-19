use std::path::Path;

/// Shorten a path relative to project root if possible
pub fn shorten_path(path: &str, project_root: Option<&Path>) -> String {
    if let Some(root) = project_root {
        if let Ok(relative) = Path::new(path).strip_prefix(root) {
            let relative_str = relative.to_string_lossy();
            if relative_str.len() < path.len() {
                return relative_str.to_string();
            }
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_shorten_path_with_project_root() {
        let root = PathBuf::from("/Users/test/project");
        let path = "/Users/test/project/src/main.rs";
        assert_eq!(shorten_path(path, Some(&root)), "src/main.rs");
    }

    #[test]
    fn test_shorten_path_without_project_root() {
        let path = "/Users/test/project/src/main.rs";
        assert_eq!(shorten_path(path, None), path);
    }

    #[test]
    fn test_shorten_path_outside_project() {
        let root = PathBuf::from("/Users/test/project");
        let path = "/Users/test/other/file.rs";
        assert_eq!(shorten_path(path, Some(&root)), path);
    }
}
