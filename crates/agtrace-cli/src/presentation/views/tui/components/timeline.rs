//! Timeline Component
//!
//! Encapsulates timeline list state and input handling.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, widgets::ListState, Frame};

use crate::presentation::view_models::TimelineViewModel;
use crate::presentation::views::tui::TimelineView;

/// Actions that Timeline can emit to parent
#[derive(Debug, Clone)]
pub enum TimelineAction {
    /// User selected an event for details
    ShowEventDetails(usize),
}

/// Timeline component with encapsulated state and logic
pub struct TimelineComponent {
    /// List state (scroll position, selection) - PRIVATE
    state: ListState,
}

impl TimelineComponent {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
        }
    }

    /// Handle keyboard input
    ///
    /// Returns an action if parent needs to respond (e.g., navigate to details page).
    pub fn handle_input(&mut self, key: KeyEvent, data_len: usize) -> Option<TimelineAction> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.next(data_len);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
                None
            }
            KeyCode::PageDown => {
                self.page_down(data_len);
                None
            }
            KeyCode::PageUp => {
                self.page_up();
                None
            }
            KeyCode::Home => {
                self.scroll_to_top();
                None
            }
            KeyCode::End => {
                self.scroll_to_bottom(data_len);
                None
            }
            KeyCode::Enter => self.state.selected().map(TimelineAction::ShowEventDetails),
            _ => None,
        }
    }

    /// Render timeline with data
    ///
    /// Performs index safety check before rendering.
    pub fn render(&mut self, f: &mut Frame, area: Rect, data: &TimelineViewModel) {
        // Index Safety: Clamp selection to data bounds
        if let Some(selected) = self.state.selected() {
            if selected >= data.events.len() && !data.events.is_empty() {
                self.state.select(Some(data.events.len() - 1));
            } else if data.events.is_empty() {
                self.state.select(None);
            }
        }

        // Build List widget and render with state
        let list = TimelineView::new(data).build_list();
        f.render_stateful_widget(list, area, &mut self.state);
    }

    // Private state manipulation methods - Renderer doesn't know these

    fn next(&mut self, data_len: usize) {
        if data_len == 0 {
            return;
        }

        let next = match self.state.selected() {
            Some(i) => {
                if i >= data_len - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(next));
    }

    fn previous(&mut self) {
        let prev = match self.state.selected() {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => 0,
        };
        self.state.select(Some(prev));
    }

    fn page_down(&mut self, data_len: usize) {
        if data_len == 0 {
            return;
        }

        let next = match self.state.selected() {
            Some(i) => (i + 10).min(data_len - 1),
            None => 0,
        };
        self.state.select(Some(next));
    }

    fn page_up(&mut self) {
        let prev = match self.state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.state.select(Some(prev));
    }

    fn scroll_to_top(&mut self) {
        self.state.select(Some(0));
    }

    fn scroll_to_bottom(&mut self, data_len: usize) {
        if data_len > 0 {
            self.state.select(Some(data_len - 1));
        }
    }
}

impl Default for TimelineComponent {
    fn default() -> Self {
        Self::new()
    }
}
