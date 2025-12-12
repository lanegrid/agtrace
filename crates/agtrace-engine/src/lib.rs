// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

mod activity;
pub mod analysis;
pub mod export;
pub mod summary;

pub use activity::{
    interpret_events, interpret_events_with_options, Activity, ActivityStats, ActivityStatus,
    InterpretOptions, ToolSummary,
};
