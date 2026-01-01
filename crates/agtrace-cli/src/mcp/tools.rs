use agtrace_sdk::{Client, Lens, SessionFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsArgs {
    #[serde(default)]
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub project_hash: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionDetailsArgs {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeSessionArgs {
    pub session_id: String,
    #[serde(default)]
    pub include_failures: Option<bool>,
    #[serde(default)]
    pub include_loops: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEventsArgs {
    pub pattern: String,
    #[serde(default)]
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub event_type: Option<String>,
}

pub async fn handle_list_sessions(
    client: &Client,
    args: ListSessionsArgs,
) -> Result<Value, String> {
    let limit = args.limit.unwrap_or(10);

    let mut filter = if let Some(project_hash) = args.project_hash {
        SessionFilter::project(project_hash.into()).limit(limit)
    } else {
        SessionFilter::all().limit(limit)
    };

    if let Some(provider) = args.provider {
        filter = filter.provider(provider);
    }

    if let Some(since) = args.since {
        filter = filter.since(since);
    }

    if let Some(until) = args.until {
        filter = filter.until(until);
    }

    let mut sessions = client
        .sessions()
        .list(filter)
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    // Truncate snippets to prevent large responses
    for session in &mut sessions {
        if let Some(ref snippet) = session.snippet
            && snippet.len() > 200 {
                session.snippet = Some(format!(
                    "{}...",
                    &snippet.chars().take(197).collect::<String>()
                ));
            }
    }

    serde_json::to_value(&sessions).map_err(|e| format!("Serialization error: {}", e))
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

    serde_json::to_value(&session).map_err(|e| format!("Serialization error: {}", e))
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
    let limit = args.limit.unwrap_or(50);

    let mut filter = SessionFilter::all().limit(1000);

    if let Some(provider) = args.provider {
        filter = filter.provider(provider);
    }

    let sessions = client
        .sessions()
        .list_without_refresh(filter)
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    let mut matches = Vec::new();
    let mut total_matches = 0;

    for session_summary in sessions {
        if total_matches >= limit {
            break;
        }

        let handle = match client.sessions().get(&session_summary.id) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let events = match handle.events() {
            Ok(e) => e,
            Err(_) => continue,
        };

        for event in events {
            if total_matches >= limit {
                break;
            }

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

                matches.push(serde_json::json!({
                    "session_id": session_summary.id,
                    "timestamp": event.timestamp,
                    "type": format!("{:?}", event.payload),
                    "payload": event.payload,
                }));
                total_matches += 1;
            }
        }
    }

    Ok(serde_json::json!({
        "matches": matches,
        "total": total_matches,
    }))
}

pub async fn handle_get_project_info(client: &Client) -> Result<Value, String> {
    let projects = client
        .projects()
        .list()
        .map_err(|e| format!("Failed to list projects: {}", e))?;

    serde_json::to_value(&projects).map_err(|e| format!("Serialization error: {}", e))
}
