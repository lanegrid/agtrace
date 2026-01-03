// MCP Tools Implementation
//
// Design Rationale:
// - Progressive Disclosure: search_event_previews (breadth) → get_event_details (depth)
//   to avoid overwhelming LLM context with large payloads
// - Response Size Targets: Follow MCP best practices (< 30 KB per response)
// - Cursor-based Pagination: MCP 2024-11-05 spec compliance (stable, opaque tokens)
// - Structured Errors: McpError with codes/details instead of free-text strings
// - Type Safety: EventType/Provider enums prevent invalid filter values

use agtrace_sdk::{Client, Lens, SessionFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::models::{
    AnalysisViewModel, AnalyzeSessionArgs, EventDetailsViewModel, EventPreviewViewModel,
    GetEventDetailsArgs, GetSessionFullArgs, GetSessionSummaryArgs, GetSessionTurnsArgs,
    GetTurnStepsArgs, ListSessionsArgs, ListSessionsViewModel, McpError, ProjectInfoViewModel,
    SearchEventPreviewsArgs, SearchEventPreviewsViewModel, SessionFullViewModel,
    SessionSummaryViewModel, SessionTurnsViewModel, TurnStepsViewModel,
};

#[derive(Debug, Serialize, Deserialize)]
struct Cursor {
    offset: usize,
}

impl Cursor {
    fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json.as_bytes())
    }

    fn decode(cursor: &str) -> Option<Self> {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor).ok()?;
        let json = String::from_utf8(bytes).ok()?;
        serde_json::from_str(&json).ok()
    }
}

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
    let project_hash_filter = if let Some(root) = args.project_root {
        // Calculate hash from root path (server-side resolution)
        Some(agtrace_sdk::utils::project_hash_from_root(&root))
    } else {
        // Use explicit hash if provided
        args.project_hash.map(|h| h.into())
    };

    let mut filter = if let Some(hash) = project_hash_filter {
        SessionFilter::project(hash).limit(fetch_limit + offset)
    } else {
        SessionFilter::all().limit(fetch_limit + offset)
    };

    if let Some(provider) = args.provider {
        filter = filter.provider(provider.as_str().to_string());
    }

    if let Some(since) = args.since {
        filter = filter.since(since);
    }

    if let Some(until) = args.until {
        filter = filter.until(until);
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
        analyzer = analyzer.through(Lens::Failures);
    }

    if args.include_loops.unwrap_or(false) {
        analyzer = analyzer.through(Lens::Loops);
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

// search_event_previews: Lightweight event discovery (returns ~300 char previews)
// Rationale: Replaces search_events with better progressive disclosure.
// - Always returns previews (consistent sizing, ~10-15 KB per page)
// - Provides event_index for precise retrieval via get_event_details
// - Supports session_id filter for scoped searches (not available in old API)
pub async fn handle_search_event_previews(
    client: &Client,
    args: SearchEventPreviewsArgs,
) -> Result<Value, String> {
    let limit = args.limit.unwrap_or(10).min(50); // default 10, max 50
    let offset = args
        .cursor
        .as_ref()
        .and_then(|c| Cursor::decode(c))
        .map(|c| c.offset)
        .unwrap_or(0);

    // Resolve project filter: project_root takes priority over project_hash
    let project_hash_filter = if let Some(root) = args.project_root {
        // Calculate hash from root path (server-side resolution)
        Some(agtrace_sdk::utils::project_hash_from_root(&root))
    } else {
        // Use explicit hash if provided
        args.project_hash.map(|h| h.into())
    };

    // Build session filter
    let mut filter = if let Some(hash) = project_hash_filter {
        SessionFilter::project(hash).limit(1000)
    } else {
        SessionFilter::all().limit(1000)
    };

    if let Some(provider) = args.provider {
        filter = filter.provider(provider.as_str().to_string());
    }

    // If searching within specific session, use that
    let sessions = if let Some(ref session_id) = args.session_id {
        let _handle = client
            .sessions()
            .get(session_id)
            .map_err(|e| format!("Session not found: {}", e))?;

        // Create a minimal session summary
        vec![agtrace_sdk::SessionSummary {
            id: session_id.clone(),
            provider: String::new(), // Will be filled from actual session
            project_hash: agtrace_sdk::types::ProjectHash::from(String::new()),
            project_root: None,
            start_ts: None,
            snippet: None,
        }]
    } else {
        client
            .sessions()
            .list_without_refresh(filter)
            .map_err(|e| format!("Failed to list sessions: {}", e))?
    };

    let mut all_matches = Vec::new();

    for session_summary in sessions {
        let handle = match client.sessions().get(&session_summary.id) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let events = match handle.events() {
            Ok(e) => e,
            Err(_) => continue,
        };

        for (event_index, event) in events.iter().enumerate() {
            // Check event type filter
            if let Some(ref event_type_filter) = args.event_type
                && !event_type_filter.matches_payload(&event.payload)
            {
                continue;
            }

            // Check if query matches
            let event_json = match serde_json::to_string(&event.payload) {
                Ok(j) => j,
                Err(_) => continue,
            };

            if event_json.contains(&args.query) {
                let preview = EventPreviewViewModel::from_event(
                    session_summary.id.clone(),
                    event_index,
                    event,
                );
                all_matches.push(preview);
            }
        }
    }

    // Apply cursor-based pagination
    let fetch_limit = limit + 1;
    let mut matches: Vec<_> = all_matches
        .into_iter()
        .skip(offset)
        .take(fetch_limit)
        .collect();

    let has_more = matches.len() > limit;
    if has_more {
        matches.pop();
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

    let view_model = SearchEventPreviewsViewModel::new(matches, next_cursor);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

// get_event_details: Retrieve full event payload by index
// Rationale: Complements search_event_previews for drill-down workflow.
// - event_index enables precise, efficient retrieval (no re-search needed)
// - No truncation or size limits (single event ~1-5 KB is safe)
pub async fn handle_get_event_details(
    client: &Client,
    args: GetEventDetailsArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let events = handle
        .events()
        .map_err(|e| format!("Failed to load events: {}", e))?;

    if args.event_index >= events.len() {
        return Err(McpError::invalid_event_index(
            &args.session_id,
            args.event_index,
            events.len() - 1,
        )
        .to_string());
    }

    let event = &events[args.event_index];
    let view_model = EventDetailsViewModel::new(
        args.session_id,
        args.event_index,
        event.timestamp,
        event.payload.clone(),
    );

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

// ============================================================================
// New Specialized Session Tools (Approach B)
// ============================================================================

/// Get lightweight session overview (≤5 KB, guaranteed single-page)
pub async fn handle_get_session_summary(
    client: &Client,
    args: GetSessionSummaryArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let session = handle
        .assemble()
        .map_err(|e| format!("Failed to assemble session: {}", e))?;

    let metadata = handle
        .metadata()
        .map_err(|e| format!("Failed to get session metadata: {}", e))?;

    let view_model = SessionSummaryViewModel::new(session, metadata);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

/// Get turn-level summaries with pagination (10-30 KB per page)
pub async fn handle_get_session_turns(
    client: &Client,
    args: GetSessionTurnsArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let session = handle
        .assemble()
        .map_err(|e| format!("Failed to assemble session: {}", e))?;

    let limit = args.limit();
    let offset = args
        .cursor
        .as_ref()
        .and_then(|c| Cursor::decode(c))
        .map(|c| c.offset)
        .unwrap_or(0);

    let total_turns = session.turns.len();
    let remaining = total_turns.saturating_sub(offset);
    let has_more = remaining > limit;

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

    let view_model = SessionTurnsViewModel::new(session, offset, limit, next_cursor);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

/// Get detailed steps for a specific turn (20-50 KB)
pub async fn handle_get_turn_steps(
    client: &Client,
    args: GetTurnStepsArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let session = handle
        .assemble()
        .map_err(|e| format!("Failed to assemble session: {}", e))?;

    let total_turns = session.turns.len();
    let turn = session
        .turns
        .into_iter()
        .nth(args.turn_index)
        .ok_or_else(|| {
            format!(
                "Turn index {} out of range (session has {} turns)",
                args.turn_index, total_turns
            )
        })?;

    let view_model = TurnStepsViewModel::new(args.session_id.clone(), args.turn_index, turn);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}

/// Get complete session data with full payloads (50-100 KB per chunk, paginated)
pub async fn handle_get_session_full(
    client: &Client,
    args: GetSessionFullArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let session = handle
        .assemble()
        .map_err(|e| format!("Failed to assemble session: {}", e))?;

    let limit = args.limit();
    let offset = if args.is_initial() {
        0
    } else {
        args.cursor
            .as_ref()
            .and_then(|c| Cursor::decode(c))
            .map(|c| c.offset)
            .unwrap_or(0)
    };

    let total_turns = session.turns.len();
    let remaining = total_turns.saturating_sub(offset);
    let has_more = remaining > limit;

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

    let view_model = SessionFullViewModel::new(session, offset, limit, next_cursor);

    serde_json::to_value(&view_model).map_err(|e| format!("Serialization error: {}", e))
}
