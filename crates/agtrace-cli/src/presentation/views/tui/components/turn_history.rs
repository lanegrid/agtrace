//! Turn History Component
//!
//! Encapsulates turn history list state and input handling.
//! Supports scrolling (j/k, arrows, PageUp/Down, Home/End) and auto-follow.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect, widgets::ListState};

use crate::presentation::view_models::TurnHistoryViewModel;
use crate::presentation::views::tui::TurnHistoryView;

/// Turn history component with encapsulated state and logic
pub struct TurnHistoryComponent {
    /// List state (scroll position, selection) - PRIVATE
    state: ListState,
    /// Previous list item count for auto-follow detection
    prev_item_count: usize,
}

impl TurnHistoryComponent {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            prev_item_count: 0,
        }
    }

    /// Handle keyboard input
    ///
    /// Returns true if the input was handled.
    pub fn handle_input(&mut self, key: KeyEvent, data_len: usize) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.next(data_len);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
                true
            }
            KeyCode::PageDown => {
                self.page_down(data_len);
                true
            }
            KeyCode::PageUp => {
                self.page_up();
                true
            }
            KeyCode::Home => {
                self.scroll_to_top();
                true
            }
            KeyCode::End => {
                self.scroll_to_bottom(data_len);
                true
            }
            _ => false,
        }
    }

    /// Get the current item count (for input handling)
    pub fn get_item_count(&self) -> usize {
        self.prev_item_count
    }

    /// Render turn history with data
    ///
    /// Performs index safety check and auto-follow before rendering.
    pub fn render(&mut self, f: &mut Frame, area: Rect, data: &TurnHistoryViewModel) {
        let view = TurnHistoryView::new(data);

        // Handle waiting state - delegate to view's Widget implementation
        if view.has_waiting_state() || view.is_empty() {
            f.render_widget(view, area);
            self.prev_item_count = 0;
            return;
        }

        let (list, block, list_area, item_count) = view.build_list_with_layout(area);

        // Auto-follow: scroll to bottom on initial render or when new items are added
        if item_count > 0 && (self.prev_item_count == 0 || item_count > self.prev_item_count) {
            self.scroll_to_bottom(item_count);
        }
        self.prev_item_count = item_count;

        // Index Safety: Clamp selection to data bounds
        if let Some(selected) = self.state.selected() {
            if selected >= item_count && item_count > 0 {
                self.state.select(Some(item_count - 1));
            } else if item_count == 0 {
                self.state.select(None);
            }
        }

        // Calculate inner area before consuming the block
        let inner = block.inner(area);

        // Render block border first
        f.render_widget(block, area);

        // Render list with state
        f.render_stateful_widget(list, list_area, &mut self.state);

        // Render active turn detail section if applicable
        if let Some(detail_area) = TurnHistoryView::new(data).get_active_turn_detail_area(inner) {
            TurnHistoryView::new(data).render_active_turn_detail_to(f, detail_area);
        }
    }

    // Private state manipulation methods

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

impl Default for TurnHistoryComponent {
    fn default() -> Self {
        Self::new()
    }
}
