pub mod event_preview;
mod full;
mod summary;
mod turn_steps;
mod turns;

pub use event_preview::{
    EventDetailsResponse, EventPreview, PreviewContent, SearchEventPreviewsData,
};
pub use full::SessionFullResponse;
pub use summary::SessionSummaryResponse;
pub use turn_steps::TurnStepsResponse;
pub use turns::SessionTurnsResponse;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummaryDto>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
}

/// Session summary DTO for list operations
/// Wraps agtrace_sdk::SessionSummary directly
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SessionSummaryDto(pub agtrace_sdk::SessionSummary);

impl SessionSummaryDto {
    pub fn from_sdk(summary: agtrace_sdk::SessionSummary) -> Self {
        Self(summary)
    }
}
