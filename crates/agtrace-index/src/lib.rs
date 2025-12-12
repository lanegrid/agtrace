// SQLite pointer index
// Stores metadata only, no event normalization

mod db;
mod storage; // Legacy v1 file-based storage (deprecated)

// Public API
pub use db::{Database, LogFileRecord, ProjectRecord, SessionRecord, SessionSummary};

// Legacy storage (deprecated, kept for backward compatibility)
#[allow(deprecated)]
pub use storage::Storage;
