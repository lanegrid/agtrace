use crate::config::Config;
use agtrace_index::{Database, LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::{ClaudeProvider, CodexProvider, GeminiProvider, LogProvider, ScanContext};
use agtrace_types::project_hash_from_root;
use anyhow::{Context, Result};

pub fn handle(
    db: &Database,
    config: &Config,
    provider: String,
    project_root: Option<String>,
    all_projects: bool,
    _force: bool,
    verbose: bool,
) -> Result<()> {
    let current_project_root = if let Some(root) = project_root {
        Some(root)
    } else if let Ok(cwd) = std::env::current_dir() {
        Some(cwd.display().to_string())
    } else {
        None
    };

    let providers: Vec<Box<dyn LogProvider>> = match provider.as_str() {
        "claude" => vec![Box::new(ClaudeProvider::new())],
        "codex" => vec![Box::new(CodexProvider::new())],
        "gemini" => vec![Box::new(GeminiProvider::new())],
        "all" => vec![
            Box::new(ClaudeProvider::new()),
            Box::new(CodexProvider::new()),
            Box::new(GeminiProvider::new()),
        ],
        _ => anyhow::bail!("Unknown provider: {}", provider),
    };

    let mut total_sessions = 0;

    for provider in providers {
        let provider_name = provider.name();

        let provider_config = match config.providers.get(provider_name) {
            Some(cfg) => cfg,
            None => {
                if verbose {
                    println!("No configuration found for provider: {}", provider_name);
                }
                continue;
            }
        };

        if !provider_config.enabled {
            if verbose {
                println!("Skipping disabled provider: {}", provider_name);
            }
            continue;
        }

        let log_root = &provider_config.log_root;
        if !log_root.exists() {
            if verbose {
                println!(
                    "Warning: log_root does not exist for {}: {}",
                    provider_name,
                    log_root.display()
                );
            }
            continue;
        }

        if verbose {
            println!("Scanning provider: {}", provider_name);
        }

        let project_hash = if let Some(root) = &current_project_root {
            project_hash_from_root(root)
        } else {
            "unknown".to_string()
        };

        let scan_context = ScanContext {
            project_hash: project_hash.clone(),
            project_root: if all_projects {
                None
            } else {
                current_project_root.clone()
            },
        };

        let sessions = provider
            .scan(log_root, &scan_context)
            .with_context(|| format!("Failed to scan {}", provider_name))?;

        if verbose {
            println!(
                "  Found {} sessions for project {}",
                sessions.len(),
                if all_projects { "(all)" } else { &project_hash }
            );
        }

        for session in sessions {
            if verbose {
                println!("  Registered: {}", session.session_id);
            }

            let project_record = ProjectRecord {
                hash: session.project_hash.clone(),
                root_path: session.project_root.clone(),
                last_scanned_at: Some(chrono::Utc::now().to_rfc3339()),
            };
            db.insert_or_update_project(&project_record)?;

            let session_record = SessionRecord {
                id: session.session_id.clone(),
                project_hash: session.project_hash.clone(),
                provider: session.provider.clone(),
                start_ts: session.start_ts.clone(),
                end_ts: session.end_ts.clone(),
                snippet: session.snippet.clone(),
                is_valid: true,
            };
            db.insert_or_update_session(&session_record)?;

            for log_file in session.log_files {
                let log_file_record = LogFileRecord {
                    path: log_file.path,
                    session_id: session.session_id.clone(),
                    role: log_file.role,
                    file_size: log_file.file_size,
                    mod_time: log_file.mod_time,
                };
                db.insert_or_update_log_file(&log_file_record)?;
            }

            total_sessions += 1;
        }
    }

    println!("Scan complete: {} sessions registered", total_sessions);

    Ok(())
}
