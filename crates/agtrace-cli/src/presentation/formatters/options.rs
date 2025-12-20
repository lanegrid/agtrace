// DisplayOptions has been moved to view_models/options.rs
// Import directly from crate::presentation::view_models::DisplayOptions

/// Token usage summary for display
#[derive(Debug, Clone)]
pub struct TokenSummaryDisplay {
    pub input: i32,
    pub output: i32,
    pub cache_creation: i32,
    pub cache_read: i32,
    pub total: i32,
    pub limit: Option<u64>,
    pub model: Option<String>,
    /// Compaction buffer percentage (0-100)
    /// When input tokens exceed (100% - compaction_buffer_pct), compaction is triggered
    pub compaction_buffer_pct: Option<f64>,
}
