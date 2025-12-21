use crate::presentation::view_models::EventViewModel;
use ratatui::widgets::{ListItem, ListState};
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
    pub fresh_input: i32,
    pub cache_creation: i32,
    pub cache_read: i32,
    pub output: i32,
}

pub(crate) struct AppState {
    pub mode: WatchMode,
    pub session_title: String,
    pub provider_name: Option<String>,
    pub attached_session_id: Option<String>,
    pub model: Option<String>,
    pub compaction_buffer_pct: Option<f64>,
    pub events_buffer: VecDeque<EventViewModel>,
    pub system_messages: VecDeque<String>,
    pub timeline_items: Vec<ListItem<'static>>,
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
            provider_name: None,
            attached_session_id: None,
            model: None,
            compaction_buffer_pct: None,
            events_buffer: VecDeque::new(),
            system_messages: VecDeque::new(),
            timeline_items: Vec::new(),
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
        self.timeline_items.len()
    }

    pub fn add_event(&mut self, event: &EventViewModel) {
        let item =
            super::mapper::event_to_list_item(event, self.turn_count, self.session_start_time);

        self.events_buffer.push_back(event.clone());
        self.timeline_items.push(item);

        if self.events_buffer.len() > 1000 {
            self.events_buffer.pop_front();
        }
        if self.timeline_items.len() > 1000 {
            self.timeline_items.remove(self.system_messages.len());
        }
    }

    pub fn add_system_message(&mut self, message: String) {
        let item = super::mapper::system_message_to_list_item(&message);

        self.system_messages.push_back(message);
        self.timeline_items
            .insert(self.system_messages.len().saturating_sub(1), item);
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

    pub fn reset_session_state(&mut self, session_id: String) {
        self.attached_session_id = Some(session_id);
        self.model = None;
        self.compaction_buffer_pct = None;
        self.context_usage = None;
        self.session_start_time = None;
        self.turn_count = 0;
        self.events_buffer.clear();

        let system_msg_count = self.system_messages.len();
        self.timeline_items = self.timeline_items.drain(..system_msg_count).collect();
        self.list_state = ListState::default();
        self.mode = WatchMode::AutoFollow;
    }
}
