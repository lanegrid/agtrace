use crate::context::ExecutionContext;
use agtrace_index::{LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::ScanContext;
use agtrace_types::project_hash_from_root;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

pub fn handle(ctx: &ExecutionContext, provider: String, force: bool, verbose: bool) -> Result<()> {
    let db = ctx.db()?;
    let providers_with_roots = ctx.resolve_providers(&provider)?;

    let current_project_root = ctx.project_root.as_ref().map(|p| p.display().to_string());
    let all_projects = ctx.all_projects;

    let mut total_sessions = 0;
    let mut scanned_files = 0;
    let mut skipped_files = 0;

    // Build index of existing files for incremental scan (if not force mode)
    let indexed_files = if force {
        HashSet::new()
    } else {
        db.get_all_log_files()?
            .into_iter()
            .filter_map(|f| {
                if should_skip_indexed_file(&f) {
                    Some(f.path)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>()
    };

    if verbose && !force {
        println!(
            "Incremental scan mode: {} files already indexed",
            indexed_files.len()
        );
    }

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
            // Check if all files in this session are already indexed and unchanged
            let all_files_unchanged = !force
                && session
                    .log_files
                    .iter()
                    .all(|f| indexed_files.contains(&f.path));

            if all_files_unchanged {
                skipped_files += session.log_files.len();
                continue;
            }

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
                scanned_files += 1;
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

    if verbose {
        println!(
            "Scan complete: {} sessions, {} files scanned, {} files skipped",
            total_sessions, scanned_files, skipped_files
        );
    } else {
        println!("Scan complete: {} sessions registered", total_sessions);
    }

    Ok(())
}

/// Check if an indexed file should be skipped (unchanged)
fn should_skip_indexed_file(indexed: &LogFileRecord) -> bool {
    let path = Path::new(&indexed.path);

    // File doesn't exist anymore - don't skip (will be removed from index)
    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false, // Error reading metadata - rescan
    };

    // Compare file size
    if let Some(db_size) = indexed.file_size {
        if db_size != metadata.len() as i64 {
            return false; // Size changed - rescan
        }
    } else {
        return false; // No size in DB - rescan
    }

    // Compare mod time
    if let Some(db_mod_time) = &indexed.mod_time {
        if let Ok(fs_mod_time) = metadata.modified() {
            let fs_mod_time_str = format!("{:?}", fs_mod_time);
            if db_mod_time != &fs_mod_time_str {
                return false; // Mod time changed - rescan
            }
        } else {
            return false; // Can't read mod time - rescan
        }
    } else {
        return false; // No mod time in DB - rescan
    }

    // File unchanged - skip
    true
}
