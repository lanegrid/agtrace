use crate::context::ExecutionContext;
use crate::views::doctor::print_results;
use agtrace_engine::{categorize_parse_error, DiagnoseResult, FailureExample, FailureType};
use agtrace_providers::LogProvider;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

pub fn handle(ctx: &ExecutionContext, provider_filter: String, verbose: bool) -> Result<()> {
    let providers_with_roots = ctx.resolve_providers(&provider_filter)?;

    let mut results = Vec::new();

    for (provider, log_root) in providers_with_roots {
        if !log_root.exists() {
            eprintln!(
                "Warning: log_root does not exist for {}: {}",
                provider.name(),
                log_root.display()
            );
            continue;
        }

        let result = diagnose_provider(provider.as_ref(), &log_root)?;
        results.push(result);
    }

    print_results(&results, verbose);

    Ok(())
}

fn diagnose_provider(provider: &dyn LogProvider, log_root: &Path) -> Result<DiagnoseResult> {
    let mut all_files = Vec::new();

    // Collect all files
    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if provider.can_handle(path) {
            all_files.push(path.to_path_buf());
        }
    }

    // Process all files (no sampling)
    let files_to_check = all_files;

    let mut result = DiagnoseResult {
        provider_name: provider.name().to_string(),
        total_files: files_to_check.len(),
        successful: 0,
        failures: HashMap::new(),
    };

    // Test each file
    for file_path in files_to_check {
        match test_parse_file(provider, &file_path) {
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

fn test_parse_file(provider: &dyn LogProvider, path: &Path) -> Result<(), (FailureType, String)> {
    use agtrace_providers::ImportContext;

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
