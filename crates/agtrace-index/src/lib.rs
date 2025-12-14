// SQLite pointer index
// Stores metadata only, no event normalization

mod db;

// Public API
pub use db::{Database, LogFileRecord, ProjectRecord, SessionRecord, SessionSummary};
