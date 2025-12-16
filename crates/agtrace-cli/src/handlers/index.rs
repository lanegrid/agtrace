use crate::context::ExecutionContext;
use agtrace_index::{LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::ScanContext;
use agtrace_types::project_hash_from_root;
use anyhow::{Context, Result};

pub fn handle(ctx: &ExecutionContext, provider: String, _force: bool, verbose: bool) -> Result<()> {
    let db = ctx.db()?;
    let providers_with_roots = ctx.resolve_providers(&provider)?;

    let current_project_root = ctx.project_root.as_ref().map(|p| p.display().to_string());
    let all_projects = ctx.all_projects;

    let mut total_sessions = 0;

    for (provider, log_root) in providers_with_roots {
        let provider_name = provider.name();
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
            .scan(&log_root, &scan_context)
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
