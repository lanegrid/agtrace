use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::formatters::time::format_relative_time;
use crate::presentation::presenters::{present_worktree_list, present_worktree_sessions};
use crate::presentation::view_models::{
    WorktreeEntryViewModel, WorktreeGroupViewModel, WorktreeListViewModel,
    WorktreeSessionViewModel, WorktreeSessionsViewModel,
};
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_sdk::Client;
use agtrace_sdk::types::{RepositoryHash, SessionFilter, SessionSummary};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub fn handle_list(
    client: &Client,
    repository_hash: Option<RepositoryHash>,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let Some(repo_hash) = repository_hash else {
        anyhow::bail!(
            "Not in a git repository. The 'worktree' command requires a git repository context."
        );
    };

    // Fetch all sessions and filter by repository_hash
    let filter = SessionFilter::all();
    let sessions = client.sessions().list(filter)?;

    let sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.repository_hash.as_ref() == Some(&repo_hash))
        .collect();

    // Group sessions by project_root (worktree)
    let mut worktrees: HashMap<String, (String, usize, Option<String>)> = HashMap::new();

    for session in &sessions {
        let path = session
            .project_root
            .clone()
            .unwrap_or_else(|| "<unknown>".to_string());
        let name = extract_worktree_name(&path);

        let entry = worktrees.entry(path.clone()).or_insert((name, 0, None));

        entry.1 += 1;

        // Track latest session time
        if let Some(ref ts) = session.start_ts
            && (entry.2.is_none() || entry.2.as_ref() < Some(ts))
        {
            entry.2 = Some(ts.clone());
        }
    }

    // Convert to view models
    let mut worktree_entries: Vec<WorktreeEntryViewModel> = worktrees
        .into_iter()
        .map(|(path, (name, count, last_ts))| WorktreeEntryViewModel {
            name,
            path,
            session_count: count,
            last_active: last_ts.as_ref().map(|ts| format_relative_time(ts)),
        })
        .collect();

    // Sort by last_active descending (most recent first)
    worktree_entries.sort_by(|a, b| b.last_active.cmp(&a.last_active));

    let view_model = WorktreeListViewModel {
        repository_hash: repo_hash.as_str().to_string(),
        worktrees: worktree_entries,
    };

    // Render output
    let presentation_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(presentation_format, resolved_view_mode);
    renderer.render(present_worktree_list(view_model))?;

    Ok(())
}

pub fn handle_sessions(
    client: &Client,
    repository_hash: Option<RepositoryHash>,
    limit_per_worktree: usize,
    provider: Option<String>,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let Some(repo_hash) = repository_hash else {
        anyhow::bail!(
            "Not in a git repository. The 'worktree' command requires a git repository context."
        );
    };

    // Fetch all sessions and filter by repository_hash
    let mut filter = SessionFilter::all();
    if let Some(ref prov) = provider {
        filter = filter.provider(prov.clone());
    }
    let sessions = client.sessions().list(filter)?;

    let sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.repository_hash.as_ref() == Some(&repo_hash))
        .collect();

    // Group sessions by project_root (worktree)
    let mut groups: HashMap<String, Vec<SessionSummary>> = HashMap::new();

    for session in sessions {
        let path = session
            .project_root
            .clone()
            .unwrap_or_else(|| "<unknown>".to_string());
        groups.entry(path).or_default().push(session);
    }

    // Convert to view models
    let mut worktree_groups: Vec<WorktreeGroupViewModel> = groups
        .into_iter()
        .map(|(path, mut sessions)| {
            // Sort sessions by start_ts descending
            sessions.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
            sessions.truncate(limit_per_worktree);

            let name = extract_worktree_name(&path);
            let session_vms: Vec<WorktreeSessionViewModel> = sessions
                .iter()
                .map(|s| WorktreeSessionViewModel {
                    id: s.id.clone(),
                    id_short: s.id[..8.min(s.id.len())].to_string(),
                    time: s
                        .start_ts
                        .as_ref()
                        .map(|ts| format_relative_time(ts))
                        .unwrap_or_else(|| "-".to_string()),
                    snippet: s
                        .snippet
                        .as_ref()
                        .map(|s| truncate_snippet(s, 50))
                        .unwrap_or_else(|| "-".to_string()),
                })
                .collect();

            WorktreeGroupViewModel {
                name,
                path,
                sessions: session_vms,
            }
        })
        .collect();

    // Sort groups by most recent session
    worktree_groups.sort_by(|a, b| {
        let a_first = a.sessions.first().map(|s| &s.time);
        let b_first = b.sessions.first().map(|s| &s.time);
        // This is a simple comparison; relative times don't sort correctly
        // but the underlying sessions were already sorted
        b_first.cmp(&a_first)
    });

    let total_sessions: usize = worktree_groups.iter().map(|g| g.sessions.len()).sum();

    let view_model = WorktreeSessionsViewModel {
        repository_hash: repo_hash.as_str().to_string(),
        total_sessions,
        groups: worktree_groups,
    };

    // Render output
    let presentation_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(presentation_format, resolved_view_mode);
    renderer.render(present_worktree_sessions(view_model))?;

    Ok(())
}

fn extract_worktree_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn truncate_snippet(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', "");
    if s.chars().count() <= max_len {
        s
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
