//! TUI View Components and Stateful Components
//!
//! This module contains:
//! 1. **Views**: Stateless Ratatui Widget implementations (dashboard, timeline, etc.)
//! 2. **Components**: Stateful components that encapsulate UI state + input + render logic
//!
//! ## Design Principles:
//!
//! ### Views (Stateless):
//! - Take a reference to ViewModel (no ownership)
//! - NO logic, calculations, or formatting (except UI-specific layout)
//! - Only map ViewModel data to Ratatui widgets
//! - Color mapping from StatusLevel to Ratatui colors happens here
//!
//! ### Components (Stateful):
//! - Encapsulate UI state (ListState, scroll position, etc.)
//! - Handle keyboard input
//! - Perform index safety checks
//! - Delegate rendering to Views
//! - Emit actions to parent for navigation/side effects

pub mod components;
pub mod dashboard;
pub mod status_bar;
pub mod timeline;
pub mod turn_history;

pub use components::{DashboardComponent, TimelineComponent};
pub use dashboard::DashboardView;
pub use status_bar::StatusBarView;
pub use timeline::TimelineView;
pub use turn_history::TurnHistoryView;

use crate::presentation::view_models::common::StatusLevel;
use ratatui::style::Color;

/// Convert StatusLevel to Ratatui Color
pub(crate) fn status_level_to_color(level: StatusLevel) -> Color {
    match level {
        StatusLevel::Success => Color::Green,
        StatusLevel::Info => Color::Cyan,
        StatusLevel::Warning => Color::Yellow,
        StatusLevel::Error => Color::Red,
    }
}
