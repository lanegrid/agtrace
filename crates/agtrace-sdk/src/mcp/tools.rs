//! MCP tool handlers.

use serde_json::Value;

use crate::query::{
    AnalysisViewModel, AnalyzeSessionArgs, Cursor, GetTurnsArgs, ListSessionsArgs,
    ListSessionsViewModel, ListTurnsArgs, ProjectInfoViewModel, SearchEventsArgs,
};
use crate::{Client, Diagnostic, SessionFilter};

pub async fn handle_list_sessions(
    client: &Client,
    args: ListSessionsArgs,
) -> Result<Value, String> {
    let limit = args.limit.unwrap_or(10).min(50); // max 50 per spec
    let offset = args
        .cursor
        .as_ref()
        .and_then(|c| Cursor::decode(c))
        .map(|c| c.offset)
        .unwrap_or(0);

    // Fetch limit + 1 to check if there are more results
    let fetch_limit = limit + 1;

    // Resolve project filter: project_root takes priority over project_hash
    let project_hash_filter = if let Some(ref root) = args.project_root {
        // Calculate hash from root path (server-side resolution)
        Some(crate::utils::project_hash_from_root(root))
    } else {
        // Use explicit hash if provided
        args.project_hash.clone().map(|h| h.into())
    };

    let mut filter = if let Some(hash) = project_hash_filter {
        SessionFilter::project(hash).limit(fetch_limit + offset)
    } else {
        SessionFilter::all().limit(fetch_limit + offset)
    };

    if let Some(ref provider) = args.provider {
        filter = filter.provider(provider.as_str().to_string());
    }

    if let Some(ref since) = args.since {
        filter = filter.since(since.clone());
    }

    if let Some(ref until) = args.until {
        filter = filter.until(until.clone());
    }

    if args.include_children.unwrap_or(false) {
        filter = filter.include_children();
    }

    let all_sessions = client
        .sessions()
        .list(filter)
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    // Skip offset and take limit + 1
    let mut sessions: Vec<_> = all_sessions
        .into_iter()
        .skip(offset)
        .take(fetch_limit)
        .collect();

    // Determine if there are more results
    let has_more = sessions.len() > limit;
    if has_more {
        sessions.pop(); // Remove the extra item
    }

    let next_cursor = if has_more {
        Some(
            Cursor {
                offset: offset + limit,
            }
            .encode(),
        )
    } else {
        None
    };

    let view_model = ListSessionsViewModel::new(sessions, next_cursor);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

pub async fn handle_analyze_session(
    client: &Client,
    args: AnalyzeSessionArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let mut analyzer = handle
        .analyze()
        .map_err(|e| format!("Failed to create analyzer: {}", e))?;

    if args.include_failures.unwrap_or(true) {
        analyzer = analyzer.check(Diagnostic::Failures);
    }

    if args.include_loops.unwrap_or(false) {
        analyzer = analyzer.check(Diagnostic::Loops);
    }

    let report = analyzer
        .report()
        .map_err(|e| format!("Failed to generate report: {}", e))?;

    let view_model = AnalysisViewModel::new(report);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

pub async fn handle_get_project_info(client: &Client) -> Result<Value, String> {
    let projects = client
        .projects()
        .list()
        .map_err(|e| format!("Failed to list projects: {}", e))?;

    let view_model = ProjectInfoViewModel::new(projects);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

/// Search events and return navigation coordinates
pub async fn handle_search_events(
    client: &Client,
    args: SearchEventsArgs,
) -> Result<Value, String> {
    let response = client
        .sessions()
        .search_events(args)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

/// List turns with metadata only (no payload)
pub async fn handle_list_turns(client: &Client, args: ListTurnsArgs) -> Result<Value, String> {
    let response = client
        .sessions()
        .list_turns(args)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

/// Get specific turns with safety valves
pub async fn handle_get_turns(client: &Client, args: GetTurnsArgs) -> Result<Value, String> {
    let response = client
        .sessions()
        .get_turns(args)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}
