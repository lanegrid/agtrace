// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

mod activity;

pub use activity::{interpret_events, Activity, ActivityStats, ActivityStatus, ToolSummary};
