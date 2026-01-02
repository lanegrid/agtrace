use agtrace_sdk::{Client, Lens, SessionFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::dto::{
    AnalyzeSessionArgs, EventDetailsResponse, EventMatchDto, EventPreview, EventPreviewDto,
    GetEventDetailsArgs, GetSessionDetailsArgs, ListSessionsArgs, ListSessionsResponse, McpError,
    McpResponse, PaginationMeta, PreviewContent, SearchEventPreviewsArgs, SearchEventPreviewsData,
    SearchEventsArgs, SearchEventsResponse, SessionResponseBuilder, SessionSummaryDto,
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

    let mut filter = if let Some(project_hash) = args.project_hash {
        SessionFilter::project(project_hash.into()).limit(fetch_limit + offset)
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

    let total_in_page = sessions.len();
    let response = ListSessionsResponse {
        sessions: sessions
            .into_iter()
            .map(SessionSummaryDto::from_sdk)
            .collect(),
        total_in_page,
        next_cursor,
        hint: "Use get_session_details(id, detail_level='summary') for turn breakdown".to_string(),
    };

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

pub async fn handle_get_session_details(
    client: &Client,
    args: GetSessionDetailsArgs,
) -> Result<Value, String> {
    let handle = client
        .sessions()
        .get(&args.session_id)
        .map_err(|e| format!("Session not found: {}", e))?;

    let session = handle
        .assemble()
        .map_err(|e| format!("Failed to assemble session: {}", e))?;

    SessionResponseBuilder::new(session)
        .detail_level(args.detail_level())
        .include_reasoning(args.include_reasoning())
        .build()
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

    serde_json::to_value(&report).map_err(|e| format!("Serialization error: {}", e))
}

pub async fn handle_search_events(
    client: &Client,
    args: SearchEventsArgs,
) -> Result<Value, String> {
    let limit = args.limit.unwrap_or(5).min(20); // default 5, max 20 per spec
    let offset = args
        .cursor
        .as_ref()
        .and_then(|c| Cursor::decode(c))
        .map(|c| c.offset)
        .unwrap_or(0);
    let include_full_payload = args.include_full_payload.unwrap_or(false);

    let mut filter = SessionFilter::all().limit(1000);

    if let Some(provider) = args.provider {
        filter = filter.provider(provider);
    }

    let sessions = client
        .sessions()
        .list_without_refresh(filter)
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

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

        for event in events {
            let event_json = match serde_json::to_string(&event.payload) {
                Ok(j) => j,
                Err(_) => continue,
            };

            if event_json.contains(&args.pattern) {
                if let Some(ref event_type_filter) = args.event_type {
                    let event_type = format!("{:?}", event.payload);
                    if !event_type.starts_with(event_type_filter) {
                        continue;
                    }
                }

                let match_dto = if include_full_payload {
                    EventMatchDto::Full {
                        session_id: session_summary.id.clone(),
                        timestamp: event.timestamp,
                        event_type: format!("{:?}", event.payload),
                        payload: event.payload,
                    }
                } else {
                    EventMatchDto::Snippet {
                        session_id: session_summary.id.clone(),
                        timestamp: event.timestamp,
                        event_type: format!("{:?}", event.payload),
                        preview: EventPreviewDto::from_payload(&event.payload),
                    }
                };

                all_matches.push(match_dto);
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

    let total = matches.len();
    let response = SearchEventsResponse {
        matches,
        total,
        next_cursor,
        hint: "Use get_session_details(session_id, detail_level='steps') to see all events in a session"
            .to_string(),
    };

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

pub async fn handle_get_project_info(client: &Client) -> Result<Value, String> {
    let projects = client
        .projects()
        .list()
        .map_err(|e| format!("Failed to list projects: {}", e))?;

    serde_json::to_value(&projects).map_err(|e| format!("Serialization error: {}", e))
}

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

    // Build session filter
    let mut filter = SessionFilter::all().limit(1000);

    if let Some(provider) = args.provider {
        filter = filter.provider(provider.as_str().to_string());
    }

    // If searching within specific session, use that
    let sessions = if let Some(ref session_id) = args.session_id {
        let handle = client
            .sessions()
            .get(session_id)
            .map_err(|e| format!("Session not found: {}", e))?;

        // Create a minimal session summary
        vec![agtrace_sdk::SessionSummary {
            id: session_id.clone(),
            provider: String::new(), // Will be filled from actual session
            project_hash: agtrace_sdk::types::ProjectHash::from(String::new()),
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
                && !event_type_filter.matches_payload(&event.payload) {
                    continue;
                }

            // Check if query matches
            let event_json = match serde_json::to_string(&event.payload) {
                Ok(j) => j,
                Err(_) => continue,
            };

            if event_json.contains(&args.query) {
                let event_type = match &event.payload {
                    agtrace_sdk::types::EventPayload::ToolCall(_) => {
                        super::dto::EventType::ToolCall
                    }
                    agtrace_sdk::types::EventPayload::ToolResult(_) => {
                        super::dto::EventType::ToolResult
                    }
                    agtrace_sdk::types::EventPayload::Message(_) => super::dto::EventType::Message,
                    agtrace_sdk::types::EventPayload::User(_) => super::dto::EventType::User,
                    agtrace_sdk::types::EventPayload::Reasoning(_) => {
                        super::dto::EventType::Reasoning
                    }
                    agtrace_sdk::types::EventPayload::TokenUsage(_) => {
                        super::dto::EventType::TokenUsage
                    }
                    agtrace_sdk::types::EventPayload::Notification(_) => {
                        super::dto::EventType::Notification
                    }
                };

                all_matches.push(EventPreview {
                    session_id: session_summary.id.clone(),
                    event_index,
                    timestamp: event.timestamp,
                    event_type,
                    preview: PreviewContent::from_payload(&event.payload),
                });
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

    let total_in_page = matches.len();
    let response = McpResponse {
        data: SearchEventPreviewsData { matches },
        pagination: Some(PaginationMeta {
            total_in_page,
            next_cursor,
            has_more,
        }),
        hint: Some(
            "Use get_event_details(session_id, event_index) to retrieve full event payload"
                .to_string(),
        ),
    };

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

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
    let response = EventDetailsResponse::from_event(args.session_id, args.event_index, event);

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}
