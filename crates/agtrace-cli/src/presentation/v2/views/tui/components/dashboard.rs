//! Dashboard Component (Page-level)
//!
//! Encapsulates the entire Dashboard page layout and coordination.

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::presentation::v2::view_models::TuiScreenViewModel;
use crate::presentation::v2::views::tui::{DashboardView, StatusBarView, TurnHistoryView};

use super::timeline::{TimelineAction, TimelineComponent};

/// Actions that Dashboard can emit to parent
#[derive(Debug, Clone)]
pub enum DashboardAction {
    /// Navigate to event details
    ShowEventDetails(usize),
}

/// Dashboard component (page-level)
///
/// Coordinates multiple sub-components (timeline, turn history, etc.).
pub struct DashboardComponent {
    /// Timeline component
    timeline: TimelineComponent,
}

impl DashboardComponent {
    pub fn new() -> Self {
        Self {
            timeline: TimelineComponent::new(),
        }
    }

    /// Handle keyboard input for Dashboard page
    pub fn handle_input(
        &mut self,
        key: KeyEvent,
        screen: &TuiScreenViewModel,
    ) -> Option<DashboardAction> {
        // Delegate to timeline component
        let timeline_len = screen.timeline.events.len();
        if let Some(action) = self.timeline.handle_input(key, timeline_len) {
            match action {
                TimelineAction::ShowEventDetails(idx) => {
                    return Some(DashboardAction::ShowEventDetails(idx));
                }
            }
        }

        None
    }

    /// Render Dashboard page
    ///
    /// Layout: [Dashboard info | Timeline + Turn History | Status Bar]
    pub fn render(&mut self, f: &mut Frame, size: Rect, screen: &TuiScreenViewModel) {
        let main_chunks = Layout::vertical([
            Constraint::Length(9),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

        let dashboard_view = DashboardView::new(&screen.dashboard);
        f.render_widget(dashboard_view, main_chunks[0]);

        let content_chunks =
            Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(main_chunks[1]);

        self.timeline.render(f, content_chunks[0], &screen.timeline);

        let turn_history_view = TurnHistoryView::new(&screen.turn_history);
        f.render_widget(turn_history_view, content_chunks[1]);

        let status_bar_view = StatusBarView::new(&screen.status_bar);
        f.render_widget(status_bar_view, main_chunks[2]);
    }
}

impl Default for DashboardComponent {
    fn default() -> Self {
        Self::new()
    }
}
