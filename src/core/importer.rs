use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider};
use crate::utils::discover_project_root;
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

    pub fn import_path(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>> {
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

            let search_root = if should_filter_by_project {
                if let Some(ref target_root) = target_project_root {
                    for provider in &self.providers {
                        if let Some(optimized_root) = provider.get_search_root(path, target_root) {
                            return self.import_from_directory(
                                &optimized_root,
                                context,
                                Some(target_root),
                                provider.as_ref(),
                            );
                        }
                    }
                }
                path
            } else {
                path
            };

            for entry in WalkDir::new(search_root).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }

                let entry_path = entry.path();

                for provider in &self.providers {
                    if provider.can_handle(entry_path) {
                        if should_filter_by_project {
                            if let Some(ref target_root) = target_project_root {
                                if !provider.belongs_to_project(entry_path, target_root) {
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

    fn import_from_directory(
        &self,
        search_root: &Path,
        context: &ImportContext,
        target_root: Option<&PathBuf>,
        provider: &dyn LogProvider,
    ) -> Result<Vec<AgentEventV1>> {
        let mut all_events = Vec::new();

        for entry in WalkDir::new(search_root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let entry_path = entry.path();

            if !provider.can_handle(entry_path) {
                continue;
            }

            if let Some(target_root) = target_root {
                if !provider.belongs_to_project(entry_path, target_root) {
                    continue;
                }
            }

            match provider.normalize_file(entry_path, context) {
                Ok(events) => {
                    all_events.extend(events);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", entry_path.display(), e);
                }
            }
        }

        Ok(all_events)
    }
}
