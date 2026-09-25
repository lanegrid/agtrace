//! Shared colors, glyphs and small formatting helpers.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders};

use crate::presentation::view_models::watch::StatusVm;

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

/// Accent of the focused pane (border, title marker, mode label).
pub const FOCUS_COLOR: Color = Color::Cyan;

/// Bordered block with `title`: the focused pane gets a thick bold cyan border
/// and a `▶` marker in the title; the others are dimmed.
pub fn pane_block<'a>(focused: bool, title: Vec<Span<'a>>) -> Block<'a> {
    let (border, border_type) = if focused {
        (
            Style::default()
                .fg(FOCUS_COLOR)
                .add_modifier(Modifier::BOLD),
            BorderType::Thick,
        )
    } else {
        (dim(), BorderType::Plain)
    };
    let mut spans = Vec::with_capacity(title.len() + 1);
    if focused {
        spans.push(Span::styled(
            " ▶",
            Style::default()
                .fg(FOCUS_COLOR)
                .add_modifier(Modifier::BOLD),
        ));
        spans.extend(title);
    } else {
        // Unfocused: every title span is dimmed (colours kept for warnings).
        spans.extend(title.into_iter().map(|s| {
            let style = s.style.add_modifier(Modifier::DIM);
            s.style(style)
        }));
    }
    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border)
        .title(Line::from(spans))
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
