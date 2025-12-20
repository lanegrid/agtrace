use crate::presentation::view_models::EventViewModel;
use ratatui::widgets::ListState;
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
    pub events_buffer: VecDeque<EventViewModel>,
    pub system_messages: VecDeque<String>,
    pub footer_lines: Vec<String>,
    pub context_usage: Option<ContextUsageState>,
    pub session_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub turn_count: usize,
    pub list_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: WatchMode::AutoFollow,
            session_title: String::new(),
            events_buffer: VecDeque::new(),
            system_messages: VecDeque::new(),
            footer_lines: Vec::new(),
            context_usage: None,
            session_start_time: None,
            turn_count: 0,
            list_state: ListState::default(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_items(&self) -> usize {
        self.system_messages.len() + self.events_buffer.len()
    }

    pub fn select_next(&mut self) {
        let total = self.total_items();
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= total.saturating_sub(1) {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));

        if i >= total.saturating_sub(1) {
            self.mode = WatchMode::AutoFollow;
        } else {
            self.mode = WatchMode::Fixed;
        }
    }

    pub fn select_previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.mode = WatchMode::Fixed;
    }

    pub fn on_tick(&mut self) {
        let total = self.total_items();
        if self.mode == WatchMode::AutoFollow && total > 0 {
            self.list_state.select(Some(total.saturating_sub(1)));
        }
    }
}
