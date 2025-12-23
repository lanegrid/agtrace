//! TUI View Components
//!
//! This module contains Ratatui Widget implementations for the Watch TUI.
//! Each component is a thin wrapper around a ViewModel that implements
//! the Widget trait for rendering.
//!
//! ## Design Principles:
//! - Views take a reference to ViewModel (no ownership)
//! - NO logic, calculations, or formatting (except UI-specific layout)
//! - Only map ViewModel data to Ratatui widgets
//! - Color mapping from StatusLevel to Ratatui colors happens here

pub mod dashboard;
pub mod status_bar;
pub mod timeline;
pub mod turn_history;

pub use dashboard::DashboardView;
pub use status_bar::StatusBarView;
pub use timeline::TimelineView;
pub use turn_history::TurnHistoryView;

use crate::presentation::v2::view_models::common::StatusLevel;
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
