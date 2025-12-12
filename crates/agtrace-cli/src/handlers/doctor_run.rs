use crate::config::Config;
use agtrace_providers::{ClaudeProvider, CodexProvider, GeminiProvider, LogProvider};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct DiagnoseResult {
    pub provider_name: String,
    pub total_files: usize,
    pub successful: usize,
    pub failures: HashMap<FailureType, Vec<FailureExample>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureType {
    MissingField(String),
    TypeMismatch(String),
    ParseError,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::MissingField(field) => write!(f, "missing_field ({})", field),
            FailureType::TypeMismatch(field) => write!(f, "type_mismatch ({})", field),
            FailureType::ParseError => write!(f, "parse_error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FailureExample {
    pub path: String,
    pub reason: String,
}

pub fn handle(config: &Config, provider_filter: String, verbose: bool) -> Result<()> {
    let providers: Vec<Box<dyn LogProvider>> = match provider_filter.as_str() {
        "claude" => vec![Box::new(ClaudeProvider::new())],
        "codex" => vec![Box::new(CodexProvider::new())],
        "gemini" => vec![Box::new(GeminiProvider::new())],
        "all" => vec![
            Box::new(ClaudeProvider::new()),
            Box::new(CodexProvider::new()),
            Box::new(GeminiProvider::new()),
        ],
        _ => anyhow::bail!("Unknown provider: {}", provider_filter),
    };

    let mut results = Vec::new();

    for provider in providers {
        let provider_name = provider.name();

        let provider_config = match config.providers.get(provider_name) {
            Some(cfg) => cfg,
            None => {
                eprintln!("No configuration found for provider: {}", provider_name);
                continue;
            }
        };

        if !provider_config.enabled {
            continue;
        }

        let log_root = &provider_config.log_root;
        if !log_root.exists() {
            eprintln!(
                "Warning: log_root does not exist for {}: {}",
                provider_name,
                log_root.display()
            );
            continue;
        }

        let result = diagnose_provider(provider.as_ref(), log_root)?;
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

            // Categorize error
            if error_msg.contains("missing field") {
                if let Some(field) = extract_field_name(&error_msg) {
                    Err((
                        FailureType::MissingField(field.clone()),
                        format!("Missing required field: {}", field),
                    ))
                } else {
                    Err((FailureType::ParseError, error_msg))
                }
            } else if error_msg.contains("expected") || error_msg.contains("invalid type") {
                if let Some(field) = extract_field_name(&error_msg) {
                    Err((
                        FailureType::TypeMismatch(field.clone()),
                        format!("Type mismatch for field: {}", field),
                    ))
                } else {
                    Err((FailureType::ParseError, error_msg))
                }
            } else {
                Err((FailureType::ParseError, error_msg))
            }
        }
    }
}

fn extract_field_name(error_msg: &str) -> Option<String> {
    // Try to extract field name from error message
    // Example: "missing field `source`" or "field `source`"
    if let Some(start) = error_msg.find("field `") {
        let rest = &error_msg[start + 7..];
        if let Some(end) = rest.find('`') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn print_results(results: &[DiagnoseResult], verbose: bool) {
    println!("{}", "=== Diagnose Results ===".bold());
    println!();

    let mut total_failures = 0;

    for result in results {
        println!("Provider: {}", result.provider_name.bright_blue().bold());
        println!("  Total files scanned: {}", result.total_files);

        let success_rate = if result.total_files > 0 {
            (result.successful as f64 / result.total_files as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "  Successfully parsed: {} ({:.1}%)",
            result.successful.to_string().green(),
            success_rate
        );

        let failure_count = result.total_files - result.successful;
        if failure_count > 0 {
            println!(
                "  Parse failures: {} ({:.1}%)",
                failure_count.to_string().red(),
                100.0 - success_rate
            );
            println!();
            println!("  Failure breakdown:");

            for (failure_type, examples) in &result.failures {
                println!("  {} {}: {} files", "✗".red(), failure_type, examples.len());

                let display_count = if verbose {
                    examples.len()
                } else {
                    1.min(examples.len())
                };

                for example in examples.iter().take(display_count) {
                    println!("    Example: {}", example.path.bright_black());
                    println!("    Reason: {}", example.reason);
                    println!();
                }

                if !verbose && examples.len() > 1 {
                    println!("    ... and {} more files", examples.len() - 1);
                    println!();
                }
            }

            total_failures += failure_count;
        }

        println!();
    }

    println!("{}", "---".bright_black());
    if total_failures > 0 {
        println!(
            "Summary: {} files need schema updates to parse correctly",
            total_failures.to_string().yellow()
        );
        if !verbose {
            println!(
                "Run with {} to see all problematic files",
                "--verbose".cyan()
            );
        }
    } else {
        println!("{}", "All files parsed successfully!".green().bold());
    }
}
