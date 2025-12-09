pub mod io;
pub mod mapper;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider};
use crate::utils::project_hash_from_root;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use self::io::{extract_project_hash_from_gemini_file, normalize_gemini_file};

pub struct GeminiProvider;

impl GeminiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn can_handle(&self, path: &Path) -> bool {
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        filename == "logs.json" || (filename.starts_with("session-") && filename.ends_with(".json"))
    }

    fn normalize_file(&self, path: &Path, _context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_gemini_file(path)
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        let target_hash = project_hash_from_root(&target_project_root.to_string_lossy());
        if let Some(file_hash) = extract_project_hash_from_gemini_file(path) {
            file_hash == target_hash
        } else {
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if crate::utils::is_64_char_hex(dir_name) {
                        return dir_name == target_hash;
                    }
                }
            }
            false
        }
    }

    fn get_search_root(&self, log_root: &Path, target_project_root: &Path) -> Option<PathBuf> {
        let hash = project_hash_from_root(&target_project_root.to_string_lossy());
        let dir = log_root.join(&hash);
        (dir.exists() && dir.is_dir()).then_some(dir)
    }
}
