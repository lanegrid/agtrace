/// Display formatting options
#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub enable_color: bool,
    pub relative_time: bool,
    pub truncate_text: Option<usize>,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        }
    }
}

// Backwards compatibility alias
pub type DisplayOptions = FormatOptions;

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
