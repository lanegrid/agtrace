// SQLite pointer index
// Stores metadata only, no event normalization

mod db;
mod error;
mod queries;
mod records;
mod schema;

// Public API
pub use db::Database;
pub use error::{Error, Result};
pub use records::{LogFileRecord, ProjectRecord, SessionRecord, SessionSummary};
