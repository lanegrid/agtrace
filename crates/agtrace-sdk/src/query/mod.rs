//! Query types for session filtering and data retrieval.
//!
//! This module provides type-safe filtering and query parameters
//! for MCP and programmatic SDK usage.

pub mod analysis;
pub mod filters;
pub mod list;
pub mod project;
pub mod session;

pub use analysis::{AnalysisViewModel, AnalyzeSessionArgs};
pub use filters::{EventType, Provider, truncate_json_value, truncate_string};
pub use list::{ListSessionsArgs, ListSessionsViewModel};
pub use project::ProjectInfoViewModel;
pub use session::{
    Cursor, EventMatch, GetTurnsArgs, GetTurnsResponse, ListTurnsArgs, ListTurnsResponse,
    SearchEventsArgs, SearchEventsResponse, StepDetail, ToolDetail, TurnDetail, TurnMetadata,
    TurnStatus,
};
