use serde::Serialize;

use crate::presentation::v2::renderers::ConsolePresentable;

#[derive(Debug, Serialize)]
pub struct SessionListViewModel {
    pub sessions: Vec<SessionListEntry>,
    pub total_count: usize,
    pub applied_filters: FilterSummary,
}

#[derive(Debug, Serialize)]
pub struct SessionListEntry {
    pub id: String,
    pub provider: String,
    pub project_hash: String,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FilterSummary {
    pub project_filter: Option<String>,
    pub source_filter: Option<String>,
    pub time_range: Option<String>,
    pub limit: usize,
}

impl ConsolePresentable for SessionListViewModel {
    fn render_console(&self) {
        use crate::presentation::formatters::session_list::SessionEntry;
        use crate::presentation::formatters::SessionListView;

        if self.sessions.is_empty() {
            println!("No sessions found.");
            if let Some(ref project) = self.applied_filters.project_filter {
                println!("Project filter: {}", project);
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                println!("Source filter: {}", source);
            }
            return;
        }

        let entries: Vec<SessionEntry> = self
            .sessions
            .iter()
            .map(|s| SessionEntry {
                id: s.id.clone(),
                provider: s.provider.clone(),
                start_ts: s.start_ts.clone(),
                snippet: s.snippet.clone(),
            })
            .collect();

        print!("{}", SessionListView::from_entries(entries));

        if self.applied_filters.project_filter.is_some()
            || self.applied_filters.source_filter.is_some()
            || self.applied_filters.time_range.is_some()
        {
            println!();
            println!("Filters applied:");
            if let Some(ref project) = self.applied_filters.project_filter {
                println!("  Project: {}", project);
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                println!("  Source: {}", source);
            }
            if let Some(ref range) = self.applied_filters.time_range {
                println!("  Time range: {}", range);
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SessionDetailViewModel {
    pub session_id: String,
    pub provider: String,
    pub view_mode: ViewMode,
    pub content: DetailContent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Raw,
    Json,
    Compact,
    Timeline,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DetailContent {
    Raw {
        files: Vec<RawFile>,
    },
    Events {
        events: serde_json::Value,
    },
    Session {
        session: serde_json::Value,
        options: DisplayOptions,
    },
}

#[derive(Debug, Serialize)]
pub struct RawFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct DisplayOptions {
    pub enable_color: bool,
    pub relative_time: bool,
    pub truncate_text: Option<usize>,
}

impl ConsolePresentable for SessionDetailViewModel {
    fn render_console(&self) {
        use crate::presentation::view_models::RawFileContent;

        match &self.content {
            DetailContent::Raw { files } => {
                let raw_files: Vec<RawFileContent> = files
                    .iter()
                    .map(|f| RawFileContent {
                        path: f.path.clone(),
                        content: f.content.clone(),
                    })
                    .collect();
                for file in raw_files {
                    println!("{}", file.content);
                }
            }
            DetailContent::Events { events } => {
                println!("{}", serde_json::to_string_pretty(events).unwrap());
            }
            DetailContent::Session {
                session: _,
                options: _,
            } => {
                // For now, delegate to the v1 rendering
                // In a full v2 migration, we'd rewrite CompactSessionView to use v2 types
                println!(
                    "Session detail rendering (compact mode) - session_id: {}",
                    self.session_id
                );
                println!("View v1 implementation for full rendering");
            }
        }
    }
}
