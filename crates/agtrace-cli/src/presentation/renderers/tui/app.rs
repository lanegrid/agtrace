use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchMode {
    AutoFollow,
    #[allow(dead_code)]
    Fixed,
}

pub(crate) struct ContextUsageState {
    pub used: u64,
    pub limit: u64,
    #[allow(dead_code)]
    pub input_pct: f64,
    #[allow(dead_code)]
    pub output_pct: f64,
}

pub(crate) struct AppState {
    pub mode: WatchMode,
    pub session_title: String,
    pub events_buffer: VecDeque<String>,
    pub footer_lines: Vec<String>,
    pub context_usage: Option<ContextUsageState>,
    pub session_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub turn_count: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: WatchMode::AutoFollow,
            session_title: String::new(),
            events_buffer: VecDeque::new(),
            footer_lines: Vec::new(),
            context_usage: None,
            session_start_time: None,
            turn_count: 0,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
