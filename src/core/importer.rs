use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider};
use crate::utils::{discover_project_root, paths_equal};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ImportService {
    providers: Vec<Box<dyn LogProvider>>,
}

impl ImportService {
    pub fn new(providers: Vec<Box<dyn LogProvider>>) -> Self {
        Self { providers }
    }

    pub fn import_path(
        &self,
        path: &Path,
        context: &ImportContext,
    ) -> Result<Vec<AgentEventV1>> {
        let mut all_events = Vec::new();

        if path.is_file() {
            for provider in &self.providers {
                if provider.can_handle(path) {
                    match provider.normalize_file(path, context) {
                        Ok(events) => {
                            all_events.extend(events);
                            break;
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            let (target_project_root, should_filter_by_project) = if context.all_projects {
                (None, false)
            } else if let Some(ref pr) = context.project_root_override {
                (Some(PathBuf::from(pr)), true)
            } else {
                (Some(discover_project_root(None)?), true)
            };

            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }

                let entry_path = entry.path();

                for provider in &self.providers {
                    if provider.can_handle(entry_path) {
                        // For project filtering, we need provider-specific CWD extraction
                        if should_filter_by_project {
                            if let Some(ref target_root) = target_project_root {
                                if !self.should_process_file(entry_path, target_root)? {
                                    continue;
                                }
                            }
                        }

                        match provider.normalize_file(entry_path, context) {
                            Ok(events) => {
                                all_events.extend(events);
                                break;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to parse {}: {}",
                                    entry_path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        } else {
            anyhow::bail!(
                "Root path does not exist or is not accessible: {}",
                path.display()
            );
        }

        Ok(all_events)
    }

    fn should_process_file(&self, path: &Path, target_root: &Path) -> Result<bool> {
        if let Some(session_cwd) = self.extract_cwd(path) {
            let session_cwd_path = Path::new(&session_cwd);
            Ok(paths_equal(target_root, session_cwd_path))
        } else {
            eprintln!(
                "Warning: Could not extract cwd from {}, skipping",
                path.display()
            );
            Ok(false)
        }
    }

    fn extract_cwd(&self, path: &Path) -> Option<String> {
        use crate::providers::{claude, codex};

        if path.extension().map_or(false, |e| e == "jsonl") {
            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

            if filename.starts_with("rollout-") {
                return codex::extract_cwd_from_codex_file(path);
            } else {
                return claude::extract_cwd_from_claude_file(path);
            }
        }

        None
    }
}
