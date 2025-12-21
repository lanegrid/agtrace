use ratatui::{layout::Rect, Frame};

use super::app::AppState;

pub(crate) trait Component {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState);
}

pub(crate) mod dashboard;
pub(crate) mod timeline;

pub(crate) use dashboard::DashboardComponent;
pub(crate) use timeline::TimelineComponent;
