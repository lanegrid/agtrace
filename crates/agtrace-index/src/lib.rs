// SQLite pointer index
// Stores metadata only, no event normalization

mod db;
mod queries;
mod records;
mod schema;

// Public API
pub use db::Database;
pub use records::{LogFileRecord, ProjectRecord, SessionRecord, SessionSummary};
