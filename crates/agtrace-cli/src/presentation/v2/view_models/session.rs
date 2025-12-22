use serde::Serialize;
use std::fmt;

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

impl fmt::Display for SessionListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::formatters::session_list::SessionEntry;
        use crate::presentation::formatters::SessionListView;

        if self.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            if let Some(ref project) = self.applied_filters.project_filter {
                writeln!(f, "Project filter: {}", project)?;
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                writeln!(f, "Source filter: {}", source)?;
            }
            return Ok(());
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

        write!(f, "{}", SessionListView::from_entries(entries))?;

        if self.applied_filters.project_filter.is_some()
            || self.applied_filters.source_filter.is_some()
            || self.applied_filters.time_range.is_some()
        {
            writeln!(f)?;
            writeln!(f, "Filters applied:")?;
            if let Some(ref project) = self.applied_filters.project_filter {
                writeln!(f, "  Project: {}", project)?;
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                writeln!(f, "  Source: {}", source)?;
            }
            if let Some(ref range) = self.applied_filters.time_range {
                writeln!(f, "  Time range: {}", range)?;
            }
        }

        Ok(())
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

impl fmt::Display for SessionDetailViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::view_models::RawFileContent;

        match &self.content {
            DetailContent::Raw { files } => {
                let raw_files: Vec<RawFileContent> = files
                    .iter()
                    .map(|file| RawFileContent {
                        path: file.path.clone(),
                        content: file.content.clone(),
                    })
                    .collect();
                for file in raw_files {
                    writeln!(f, "{}", file.content)?;
                }
            }
            DetailContent::Events { events } => {
                writeln!(f, "{}", serde_json::to_string_pretty(events).unwrap())?;
            }
            DetailContent::Session {
                session: _,
                options: _,
            } => {
                // For now, delegate to the v1 rendering
                // In a full v2 migration, we'd rewrite CompactSessionView to use v2 types
                writeln!(
                    f,
                    "Session detail rendering (compact mode) - session_id: {}",
                    self.session_id
                )?;
                writeln!(f, "View v1 implementation for full rendering")?;
            }
        }

        Ok(())
    }
}
