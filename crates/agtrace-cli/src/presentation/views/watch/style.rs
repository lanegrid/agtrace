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

/// Compact duration: `12s`, `4m`, `2h14m`, `3d`.
pub fn short(secs: i64) -> String {
    let s = secs.max(0);
    match s {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => {
            let (h, m) = (s / 3600, (s % 3600) / 60);
            if m == 0 {
                format!("{h}h")
            } else {
                format!("{h}h{m:02}m")
            }
        }
        s => format!("{}d", s / 86_400),
    }
}

/// Character count capped at `max` (column sizing; wide glyphs are clipped by ratatui).
pub fn col_width<'a>(items: impl Iterator<Item = &'a str>, max: usize) -> u16 {
    items.map(|s| s.chars().count()).max().unwrap_or(0).min(max) as u16
}

/// Display width of `s` in terminal columns.
pub fn text_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// `s` clipped to `width` columns (with `…` when clipped).
pub fn clip(s: &str, width: usize) -> String {
    if text_width(s) <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if w + cw + 1 > width {
            break;
        }
        out.push(c);
        w += cw;
    }
    if width > 0 {
        out.push('…');
    }
    out
}

/// Word-wrap `text` to `width` columns: every source line becomes one or more
/// output lines (long words are split); empty source lines are kept.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for src in text.lines() {
        let src = src.trim_end();
        if src.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut lw = 0;
        for word in src.split_inclusive(' ') {
            let ww = text_width(word);
            // A trailing space may hang past the edge (it is trimmed).
            if lw + text_width(word.trim_end()) > width && lw > 0 {
                out.push(line.trim_end().to_string());
                line.clear();
                lw = 0;
            }
            if text_width(word.trim_end()) > width {
                // A word longer than the line: split it by columns.
                for c in word.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if lw + cw > width {
                        out.push(std::mem::take(&mut line));
                        lw = 0;
                    }
                    line.push(c);
                    lw += cw;
                }
            } else {
                line.push_str(word);
                lw += ww;
            }
        }
        out.push(line.trim_end().to_string());
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// `███░░░` occupancy bar of `width` cells for `pct` percent (clamped).
pub fn ctx_bar(pct: u16, width: usize) -> String {
    let filled = ((pct.min(100) as usize * width) + 50) / 100;
    let filled = if pct > 0 { filled.max(1) } else { 0 };
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width - filled.min(width))
    )
}

/// Colour of an activity-lane tone (`r` running, `i` idle, `d` ended, `c` compaction).
pub fn lane_style(tone: char) -> Style {
    match tone {
        'r' => Style::default().fg(Color::Green),
        'i' => Style::default().fg(Color::Blue),
        'c' => Style::default().fg(Color::Yellow),
        _ => dim(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_breaks_words_and_keeps_blank_lines() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap("a\n\nb", 5), vec!["a", "", "b"]);
        assert_eq!(wrap("abcdefgh", 3), vec!["abc", "def", "gh"]);
        assert_eq!(wrap("日本語テキスト", 6), vec!["日本語", "テキス", "ト"]);
        assert_eq!(wrap("", 4), vec![""]);
    }

    #[test]
    fn clip_and_bar() {
        assert_eq!(clip("abcdef", 4), "abc…");
        assert_eq!(clip("ab", 4), "ab");
        assert_eq!(ctx_bar(68, 7), "█████░░");
        assert_eq!(ctx_bar(1, 7), "█░░░░░░");
        assert_eq!(ctx_bar(0, 3), "░░░");
        assert_eq!(ctx_bar(150, 3), "███");
    }
}

/// Bottom-right block title telling how many rows are scrolled off above / below
/// (`↑3 ↓10`); None when everything is shown.
pub fn more_marks(start: usize, shown: usize, total: usize) -> Option<Line<'static>> {
    let below = total.saturating_sub(start + shown);
    if start == 0 && below == 0 {
        return None;
    }
    let mut parts = Vec::new();
    if start > 0 {
        parts.push(format!("↑{start}"));
    }
    if below > 0 {
        parts.push(format!("↓{below}"));
    }
    Some(Line::styled(format!(" {} ", parts.join(" ")), dim()).right_aligned())
}

/// Block title part naming the active `/` filter (`/pr17: 2 matches`).
pub fn filter_title(
    vm: &crate::presentation::view_models::watch::WatchScreenVm,
) -> Option<Span<'static>> {
    let s = &vm.status;
    if s.filter.is_empty() {
        return None;
    }
    let noun = if s.matches == 1 { "match" } else { "matches" };
    Some(Span::styled(
        format!("· /{}: {} {noun} ", s.filter, s.matches),
        Style::default().fg(Color::Yellow),
    ))
}
