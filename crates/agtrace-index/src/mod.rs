// SQLite pointer index
// Stores metadata only, no event normalization

mod db;
mod storage; // Legacy v1 file-based storage (deprecated)

// Public API (minimal)
pub use db::{Database, SessionSummary};

// Hide internals
pub(crate) use db::{LogFileRecord, ProjectRecord, SessionRecord};

// Legacy storage (deprecated, kept for backward compatibility)
#[allow(deprecated)]
pub use storage::Storage;
