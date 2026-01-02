mod builder;
mod common;
mod error;
mod request;
mod response;
mod tool_summary;

// Public API
pub use builder::SessionResponseBuilder;
pub use common::{DetailLevel, EventType, McpResponse, PaginationMeta, Provider};
pub use error::{ErrorCode, McpError};
pub use request::{
    AnalyzeSessionArgs, GetEventDetailsArgs, GetSessionDetailsArgs, ListSessionsArgs,
    SearchEventPreviewsArgs, SearchEventsArgs,
};
pub use response::{
    EventMatchDto, EventPreviewDto, ListSessionsResponse, SearchEventsResponse, SessionSummaryDto,
};
