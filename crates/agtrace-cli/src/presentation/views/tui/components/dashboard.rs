//! Dashboard Component (Page-level)
//!
//! Encapsulates the entire Dashboard page layout and coordination.
//! Follows v1's 3-pane structure: Dashboard + Turn History + Status Bar

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::presentation::view_models::TuiScreenViewModel;
use crate::presentation::views::tui::{DashboardView, StatusBarView};

use super::TurnHistoryComponent;

/// Actions that Dashboard can emit to parent
#[derive(Debug, Clone)]
pub enum DashboardAction {
    /// Navigate to event details (reserved for future use)
    ShowEventDetails(usize),
}

/// Dashboard component (page-level)
///
/// v1-style 3-pane layout:
/// 1. Dashboard (metadata + LIFE gauge)
/// 2. Turn History (SATURATION HISTORY with active turn details)
/// 3. Status Bar (status + keyboard shortcuts)
pub struct DashboardComponent {
    /// Turn history component (scrollable)
    turn_history: TurnHistoryComponent,
}

impl DashboardComponent {
    pub fn new() -> Self {
        Self {
            turn_history: TurnHistoryComponent::new(),
        }
    }

    /// Handle keyboard input for Dashboard page
    pub fn handle_input(
        &mut self,
        key: KeyEvent,
        _screen: &TuiScreenViewModel,
    ) -> Option<DashboardAction> {
        // Delegate to turn history component
        let item_count = self.turn_history.get_item_count();
        if self.turn_history.handle_input(key, item_count) {
            return None;
        }

        None
    }

    /// Render Dashboard page
    ///
    /// v1-style Layout: [Dashboard | Turn History (full width) | Status Bar]
    pub fn render(&mut self, f: &mut Frame, size: Rect, screen: &TuiScreenViewModel) {
        let main_chunks = Layout::vertical([
            Constraint::Length(7), // Dashboard (reduced from 9)
            Constraint::Min(10),   // Turn History (full width)
            Constraint::Length(3), // Status Bar
        ])
        .split(size);

        // 1. Dashboard (metadata + LIFE gauge)
        let dashboard_view = DashboardView::new(&screen.dashboard);
        f.render_widget(dashboard_view, main_chunks[0]);

        // 2. Turn History (SATURATION HISTORY + active turn details) - scrollable
        self.turn_history
            .render(f, main_chunks[1], &screen.turn_history);

        // 3. Status Bar (status + help)
        let status_bar_view = StatusBarView::new(&screen.status_bar);
        f.render_widget(status_bar_view, main_chunks[2]);
    }
}

impl Default for DashboardComponent {
    fn default() -> Self {
        Self::new()
    }
}
