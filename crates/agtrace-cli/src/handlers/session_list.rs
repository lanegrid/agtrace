use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::types::OutputFormat;
use agtrace_index::Database;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use chrono::DateTime;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    db: &Database,
    project_hash: Option<String>,
    limit: usize,
    all_projects: bool,
    format: OutputFormat,
    source: Option<String>,
    since: Option<String>,
    until: Option<String>,
    no_auto_refresh: bool,
    data_dir: &Path,
    project_root: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    // Auto-refresh index before listing (unless disabled)
    if !no_auto_refresh {
        let ctx =
            ExecutionContext::new(data_dir.to_path_buf(), project_root.clone(), all_projects)?;

        // Run incremental scan quietly (verbose=false)
        if let Err(e) = crate::handlers::index::handle(&ctx, "all".to_string(), false, false, view)
        {
            // Don't fail the list command if refresh fails - just warn
            view.render_warning(&format!("Warning: auto-refresh failed: {}", e))?;
        }
    }

    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Fetch more sessions to allow filtering
    let fetch_limit = limit * 3;
    let mut sessions = db.list_sessions(effective_project_hash, fetch_limit)?;

    // Filter by source (provider)
    if let Some(src) = source {
        sessions.retain(|s| s.provider == src);
    }

    // Filter by since (start_ts >= since)
    if let Some(since_str) = since {
        if let Ok(since_dt) = DateTime::parse_from_rfc3339(&since_str) {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                        return dt >= since_dt;
                    }
                }
                false
            });
        }
    }

    // Filter by until (start_ts <= until)
    if let Some(until_str) = until {
        if let Ok(until_dt) = DateTime::parse_from_rfc3339(&until_str) {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                        return dt <= until_dt;
                    }
                }
                false
            });
        }
    }

    // Apply limit after filtering
    sessions.truncate(limit);

    view.render_session_list(&sessions, format)?;

    Ok(())
}
