pub mod event_preview;
mod full;
mod steps;
mod summary;
mod turns;

pub use event_preview::{
    EventDetailsResponse, EventPreview, PreviewContent, SearchEventPreviewsData,
};
pub use full::SessionFullResponse;
pub use steps::SessionStepsResponse;
pub use summary::SessionSummaryResponse;
pub use turns::SessionTurnsResponse;

use serde::Serialize;

use super::common::truncate_string;

const MAX_SNIPPET_LEN: usize = 200;

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummaryDto>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionSummaryDto {
    pub id: String,
    pub project_hash: Option<String>,
    pub provider: String,
    pub start_time: Option<String>,
    pub snippet: Option<String>,
    pub turn_count: Option<usize>,
    pub duration_seconds: Option<u64>,
    pub total_tokens: Option<u64>,
}

impl SessionSummaryDto {
    pub fn from_sdk(summary: agtrace_sdk::SessionSummary) -> Self {
        Self {
            id: summary.id,
            project_hash: Some(summary.project_hash.to_string()),
            provider: summary.provider,
            start_time: summary.start_ts,
            snippet: summary
                .snippet
                .map(|s| truncate_string(&s, MAX_SNIPPET_LEN)),
            turn_count: None,       // TODO: Add to SessionSummary in index layer
            duration_seconds: None, // TODO: Add to SessionSummary in index layer
            total_tokens: None,     // TODO: Add to SessionSummary in index layer
        }
    }
}
