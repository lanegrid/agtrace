use crate::{Error, Result};
use agtrace_engine::{DiagnoseResult, FailureExample, FailureType, categorize_parse_error};
use agtrace_providers::{ParseDiagnostics, ProviderAdapter};
use agtrace_types::AgentEvent;
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

/// Summarize line-level decode problems as a doctor failure (None = healthy).
fn diagnostics_failure(diagnostics: &ParseDiagnostics) -> Option<(FailureType, String)> {
    if !diagnostics.has_errors() {
        return None;
    }
    let mut parts = Vec::new();
    if diagnostics.invalid_json > 0 {
        parts.push(format!("{} invalid JSON line(s)", diagnostics.invalid_json));
    }
    for (kind, count) in &diagnostics.schema_mismatch {
        parts.push(format!("{count} '{kind}' line(s) with unexpected schema"));
    }
    let summary = parts.join(", ");
    let Some(first) = diagnostics.samples.first() else {
        return Some((FailureType::ParseError, summary));
    };
    let (failure_type, _) = categorize_parse_error(&first.message);
    let reason = format!(
        "{summary}; first at line {}{}: {}",
        first.line + 1,
        first
            .kind
            .as_deref()
            .map(|k| format!(" ({k})"))
            .unwrap_or_default(),
        first.message
    );
    Some((failure_type, reason))
}

impl DoctorService {
    pub fn diagnose_all(providers: &[(ProviderAdapter, PathBuf)]) -> Result<Vec<DiagnoseResult>> {
        let mut results = Vec::new();
        for (provider, root) in providers {
            if root.exists() {
                let res = Self::diagnose_provider(provider, root)?;
                results.push(res);
            }
        }
        Ok(results)
    }

    fn diagnose_provider(provider: &ProviderAdapter, log_root: &Path) -> Result<DiagnoseResult> {
        let mut all_files = Vec::new();

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if provider.discovery.probe(path).is_match() {
                all_files.push(path.to_path_buf());
            }
        }

        let files_to_check = all_files;

        let mut result = DiagnoseResult {
            provider_name: provider.id().to_string(),
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

    /// A file is healthy when it decodes (no I/O error) and no line was invalid JSON
    /// or failed its typed deserialize. Unknown / ignored record kinds are fine.
    fn test_parse_file(
        provider: &ProviderAdapter,
        path: &Path,
    ) -> std::result::Result<(), (FailureType, String)> {
        match provider.decode_file(path) {
            Ok((_, _, diagnostics)) => match diagnostics_failure(&diagnostics) {
                None => Ok(()),
                Some(failure) => Err(failure),
            },
            Err(e) => {
                let error_msg = format!("{:?}", e);
                Err(categorize_parse_error(&error_msg))
            }
        }
    }

    pub fn check_file(
        file_path: &str,
        provider: &ProviderAdapter,
        provider_name: &str,
    ) -> Result<CheckResult> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(Error::InvalidOperation(format!(
                "File not found: {}",
                file_path
            )));
        }

        match provider.decode_file(path) {
            Ok((_, events, diagnostics)) => {
                let failure = diagnostics_failure(&diagnostics);
                Ok(CheckResult {
                    file_path: file_path.to_string(),
                    provider_name: provider_name.to_string(),
                    status: if failure.is_some() {
                        CheckStatus::Failure
                    } else {
                        CheckStatus::Success
                    },
                    events,
                    error_message: failure.map(|(_, reason)| reason),
                })
            }
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
            return Err(Error::InvalidOperation(format!(
                "File not found: {}",
                file_path
            )));
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
