pub mod io;
pub mod mapper;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider};
use crate::utils::paths_equal;
use anyhow::Result;
use std::path::Path;

pub use self::io::{extract_cwd_from_codex_file, is_empty_codex_session, normalize_codex_file};

pub struct CodexProvider;

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn can_handle(&self, path: &Path) -> bool {
        let is_jsonl = path.extension().map_or(false, |e| e == "jsonl");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        is_jsonl && filename.starts_with("rollout-") && !is_empty_codex_session(path)
    }

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        let filename = path.file_name().unwrap().to_string_lossy();
        let session_id_base = if filename.ends_with(".jsonl") { &filename[..filename.len() - 6] } else { filename.as_ref() };
        let session_id = context.session_id_prefix.as_ref().map(|p| format!("{}{}", p, session_id_base)).unwrap_or_else(|| session_id_base.to_string());
        normalize_codex_file(path, &session_id, context.project_root_override.as_deref())
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        extract_cwd_from_codex_file(path).map(|cwd| paths_equal(target_project_root, Path::new(&cwd))).unwrap_or(false)
    }
}
