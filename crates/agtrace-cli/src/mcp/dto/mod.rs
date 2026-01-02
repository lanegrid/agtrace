mod builder;
mod common;
mod error;
mod request;
mod response;
mod tool_summary;

// Public API
pub use builder::SessionResponseBuilder;
pub use common::{EventType, McpResponse, PaginationMeta};
pub use error::McpError;
pub use request::{
    AnalyzeSessionArgs, GetEventDetailsArgs, GetSessionDetailsArgs, ListSessionsArgs,
    SearchEventPreviewsArgs, SearchEventsArgs,
};
pub use response::{
    EventDetailsResponse, EventMatchDto, EventPreview, EventPreviewDto, ListSessionsResponse,
    PreviewContent, SearchEventPreviewsData, SearchEventsResponse, SessionSummaryDto,
};
