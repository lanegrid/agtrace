use agtrace_engine::{categorize_parse_error, DiagnoseResult, FailureExample, FailureType};
use agtrace_providers::{ImportContext, LogProvider};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct DoctorService;

impl DoctorService {
    pub fn diagnose_all(
        providers: &[(Box<dyn LogProvider>, PathBuf)],
    ) -> Result<Vec<DiagnoseResult>> {
        let mut results = Vec::new();
        for (provider, root) in providers {
            if root.exists() {
                let res = Self::diagnose_provider(provider.as_ref(), root)?;
                results.push(res);
            }
        }
        Ok(results)
    }

    fn diagnose_provider(provider: &dyn LogProvider, log_root: &Path) -> Result<DiagnoseResult> {
        let mut all_files = Vec::new();

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if provider.can_handle(path) {
                all_files.push(path.to_path_buf());
            }
        }

        let files_to_check = all_files;

        let mut result = DiagnoseResult {
            provider_name: provider.name().to_string(),
            total_files: files_to_check.len(),
            successful: 0,
            failures: HashMap::new(),
        };

        for file_path in files_to_check {
            match Self::test_parse_file(provider, &file_path) {
                Ok(_) => {
                    result.successful += 1;
                }
                Err((failure_type, reason)) => {
                    result
                        .failures
                        .entry(failure_type)
                        .or_default()
                        .push(FailureExample {
                            path: file_path.display().to_string(),
                            reason,
                        });
                }
            }
        }

        Ok(result)
    }

    fn test_parse_file(
        provider: &dyn LogProvider,
        path: &Path,
    ) -> Result<(), (FailureType, String)> {
        let context = ImportContext {
            project_root_override: None,
            session_id_prefix: None,
            all_projects: true,
        };

        match provider.normalize_file(path, &context) {
            Ok(_events) => Ok(()),
            Err(e) => {
                let error_msg = format!("{:?}", e);
                Err(categorize_parse_error(&error_msg))
            }
        }
    }
}
