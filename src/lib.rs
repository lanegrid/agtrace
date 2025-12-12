// Core modules (stable public API)
pub mod types;     // Base types (AgentEventV1, enums)
pub mod providers; // Provider normalization (Schema-on-Read)
pub mod index;     // SQLite pointer index
pub mod engine;    // Core processing logic
pub mod config;    // Configuration

// CLI module (binary-only, not part of library API)
pub mod cli;

// Re-exports for backward compatibility (deprecated, will be removed in Phase 4)
pub mod activity {
    //! DEPRECATED: Use `crate::engine` instead
    pub use crate::engine::*;
}

pub mod db {
    //! DEPRECATED: Use `crate::index` instead
    pub use crate::index::*;
}

pub mod storage {
    //! DEPRECATED: Use `crate::index::Database` instead
    #[allow(deprecated)]
    pub use crate::index::Storage;
}

pub mod model {
    //! DEPRECATED: Use `crate::types` instead
    pub use crate::types::*;
}

pub mod utils {
    //! DEPRECATED: Functions moved to `crate::types::util`
    pub use crate::types::{
        discover_project_root, encode_claude_project_dir, is_64_char_hex, normalize_path,
        paths_equal, project_hash_from_root, resolve_effective_project_hash, truncate,
    };
}
