use agtrace_engine::{categorize_parse_error, DiagnoseResult, FailureExample, FailureType};
use agtrace_providers::{ImportContext, LogProvider};
use agtrace_types::AgentEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub enum CheckStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub file_path: String,
    pub provider_name: String,
    pub status: CheckStatus,
    pub events: Vec<AgentEvent>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InspectContentType {
    Raw(String),
    Json(serde_json::Value),
}

#[derive(Debug, Clone)]
pub struct InspectLine {
    pub number: usize,
    pub content: InspectContentType,
}

#[derive(Debug, Clone)]
pub struct InspectResult {
    pub file_path: String,
    pub total_lines: usize,
    pub shown_lines: usize,
    pub lines: Vec<InspectLine>,
}

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

    pub fn check_file(
        file_path: &str,
        provider: &dyn LogProvider,
        provider_name: &str,
    ) -> Result<CheckResult> {
        let path = Path::new(file_path);

        if !path.exists() {
            anyhow::bail!("File not found: {}", file_path);
        }

        let context = ImportContext {
            project_root_override: None,
            session_id_prefix: None,
            all_projects: true,
        };

        match provider.normalize_file(path, &context) {
            Ok(events) => Ok(CheckResult {
                file_path: file_path.to_string(),
                provider_name: provider_name.to_string(),
                status: CheckStatus::Success,
                events,
                error_message: None,
            }),
            Err(e) => Ok(CheckResult {
                file_path: file_path.to_string(),
                provider_name: provider_name.to_string(),
                status: CheckStatus::Failure,
                events: vec![],
                error_message: Some(format!("{:#}", e)),
            }),
        }
    }

    pub fn inspect_file(file_path: &str, lines: usize, json_format: bool) -> Result<InspectResult> {
        let path = Path::new(file_path);

        if !path.exists() {
            anyhow::bail!("File not found: {}", file_path);
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let total_lines = std::fs::read_to_string(path)?.lines().count();

        let mut rendered_lines = Vec::new();
        for (idx, line) in reader.lines().take(lines).enumerate() {
            let line = line?;
            let content = if json_format {
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(json) => InspectContentType::Json(json),
                    Err(_) => InspectContentType::Raw(line.clone()),
                }
            } else {
                InspectContentType::Raw(line.clone())
            };
            rendered_lines.push(InspectLine {
                number: idx + 1,
                content,
            });
        }

        Ok(InspectResult {
            file_path: file_path.to_string(),
            total_lines,
            shown_lines: rendered_lines.len(),
            lines: rendered_lines,
        })
    }
}
