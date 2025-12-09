pub mod io;
pub mod mapper;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider};
use crate::utils::{encode_claude_project_dir, paths_equal};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use self::io::{extract_cwd_from_claude_file, normalize_claude_file};

pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn can_handle(&self, path: &Path) -> bool {
        path.is_file() && path.extension().map_or(false, |e| e == "jsonl")
    }

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_claude_file(path, context.project_root_override.as_deref())
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        extract_cwd_from_claude_file(path)
            .map(|cwd| paths_equal(target_project_root, Path::new(&cwd)))
            .unwrap_or(false)
    }

    fn get_search_root(&self, log_root: &Path, target_project_root: &Path) -> Option<PathBuf> {
        let dir_name = encode_claude_project_dir(target_project_root);
        let project_specific_root = log_root.join(dir_name);
        (project_specific_root.exists() && project_specific_root.is_dir())
            .then_some(project_specific_root)
    }
}
