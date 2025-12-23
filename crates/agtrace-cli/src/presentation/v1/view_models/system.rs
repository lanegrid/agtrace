use std::path::PathBuf;

/// Legacy type, use WatchEventViewModel::Start in new code
#[derive(Debug, Clone)]
pub enum WatchStart {
    Provider { name: String, log_root: PathBuf },
    Session { id: String, log_root: PathBuf },
}

/// Legacy type, kept for backward compatibility
#[derive(Debug, Clone)]
pub struct WatchTokenUsage {
    pub total_tokens: u64,
    pub limit: Option<u64>,
    pub input_pct: Option<f64>,
    pub output_pct: Option<f64>,
    pub total_pct: Option<f64>,
}

/// Legacy type, kept for backward compatibility
#[derive(Debug, Clone)]
pub struct WatchSummary {
    pub recent_lines: Vec<String>,
    pub token_usage: Option<WatchTokenUsage>,
    pub turn_count: usize,
}
