//! Shared colors, glyphs and small formatting helpers.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

use crate::presentation::view_models::agent_watch::StatusVm;

pub fn status_glyph(s: StatusVm) -> &'static str {
    match s {
        StatusVm::Running => "●",
        StatusVm::Idle => "○",
        StatusVm::Done => "✓",
        StatusVm::Failed => "✗",
        StatusVm::Killed => "⊘",
        StatusVm::Unknown => "·",
    }
}

pub fn status_word(s: StatusVm) -> &'static str {
    match s {
        StatusVm::Running => "busy",
        StatusVm::Idle => "idle",
        StatusVm::Done => "done",
        StatusVm::Failed => "fail",
        StatusVm::Killed => "kill",
        StatusVm::Unknown => "",
    }
}

pub fn status_style(s: StatusVm) -> Style {
    match s {
        StatusVm::Running => Style::default().fg(Color::Green),
        StatusVm::Idle => Style::default().fg(Color::Blue),
        StatusVm::Done => Style::default().fg(Color::DarkGray),
        StatusVm::Failed => Style::default().fg(Color::Red),
        StatusVm::Killed => Style::default().fg(Color::Magenta),
        StatusVm::Unknown => Style::default().fg(Color::DarkGray),
    }
}

/// ≥95% red, ≥80% yellow.
pub fn ctx_style(pct: u16) -> Style {
    if pct >= 95 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if pct >= 80 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Bordered block; the focused pane gets a highlighted border.
pub fn pane_block<'a>(focused: bool) -> Block<'a> {
    let border = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(border)
}

/// `12s`, `3m04s`, `1h02m`.
pub fn elapsed(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// Character count capped at `max` (column sizing; wide glyphs are clipped by ratatui).
pub fn col_width<'a>(items: impl Iterator<Item = &'a str>, max: usize) -> u16 {
    items.map(|s| s.chars().count()).max().unwrap_or(0).min(max) as u16
}
