use crate::presentation::v2::view_models::{
    CommandResultViewModel, DetailContent, DisplayOptions, FilterSummary, Guidance, RawFile,
    SessionDetailViewModel, SessionListEntry, SessionListViewModel, StatusBadge, ViewMode,
};
use agtrace_index::SessionSummary;

pub fn present_session_list(
    sessions: Vec<SessionSummary>,
    project_filter: Option<String>,
    source_filter: Option<String>,
    time_range: Option<String>,
    limit: usize,
) -> CommandResultViewModel<SessionListViewModel> {
    let total_count = sessions.len();

    let entries: Vec<SessionListEntry> = sessions
        .into_iter()
        .map(|s| SessionListEntry {
            id: s.id,
            provider: s.provider,
            project_hash: s.project_hash,
            start_ts: s.start_ts,
            snippet: s.snippet,
        })
        .collect();

    let content = SessionListViewModel {
        sessions: entries,
        total_count,
        applied_filters: FilterSummary {
            project_filter,
            source_filter,
            time_range,
            limit,
        },
    };

    let mut result = CommandResultViewModel::new(content);

    if total_count == 0 {
        result = result
            .with_badge(StatusBadge::info("No sessions found"))
            .with_suggestion(
                Guidance::new("Index sessions to populate the database")
                    .with_command("agtrace index update"),
            )
            .with_suggestion(
                Guidance::new("Or scan all projects")
                    .with_command("agtrace index update --all-projects"),
            );
    } else {
        let label = if total_count == 1 {
            "1 session found".to_string()
        } else {
            format!("{} sessions found", total_count)
        };
        result = result.with_badge(StatusBadge::success(label));

        if total_count >= limit {
            result = result.with_suggestion(
                Guidance::new(format!(
                    "Showing first {} sessions, use --limit to see more",
                    limit
                ))
                .with_command(format!("agtrace session list --limit {}", limit * 2)),
            );
        }
    }

    result
}

pub fn present_session_raw(
    session_id: String,
    provider: String,
    files: Vec<(String, String)>,
) -> CommandResultViewModel<SessionDetailViewModel> {
    let raw_files: Vec<RawFile> = files
        .into_iter()
        .map(|(path, content)| RawFile { path, content })
        .collect();

    let content = SessionDetailViewModel {
        session_id: session_id.clone(),
        provider,
        view_mode: ViewMode::Raw,
        content: DetailContent::Raw { files: raw_files },
    };

    CommandResultViewModel::new(content)
        .with_badge(StatusBadge::info(format!("Raw files for {}", session_id)))
}

pub fn present_session_events_json(
    session_id: String,
    provider: String,
    events: serde_json::Value,
) -> CommandResultViewModel<SessionDetailViewModel> {
    let content = SessionDetailViewModel {
        session_id: session_id.clone(),
        provider,
        view_mode: ViewMode::Json,
        content: DetailContent::Events { events },
    };

    CommandResultViewModel::new(content)
        .with_badge(StatusBadge::success(format!("Events for {}", session_id)))
}

pub fn present_session_compact(
    session_id: String,
    provider: String,
    session_data: serde_json::Value,
    enable_color: bool,
    relative_time: bool,
    truncate_text: Option<usize>,
) -> CommandResultViewModel<SessionDetailViewModel> {
    let content = SessionDetailViewModel {
        session_id: session_id.clone(),
        provider,
        view_mode: ViewMode::Compact,
        content: DetailContent::Session {
            session: session_data,
            options: DisplayOptions {
                enable_color,
                relative_time,
                truncate_text,
            },
        },
    };

    CommandResultViewModel::new(content)
        .with_badge(StatusBadge::success(format!("Session {}", session_id)))
        .with_suggestion(
            Guidance::new("View detailed timeline")
                .with_command(format!("agtrace session show {} --verbose", session_id)),
        )
}
