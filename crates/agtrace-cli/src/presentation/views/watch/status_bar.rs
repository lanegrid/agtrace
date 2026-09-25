//! One-line status bar.
//!
//! ```text
//!  TIMELINE · lead  9 agents (4 running, 3 idle) · hide done      j/k scroll · G follow · Esc back
//! ```
//!
//! Left: the mode label (which pane has focus, and whose timeline), then the live
//! toast, or else agent counts, diagnostics, errors and active toggles. Right: key
//! hints for the focused pane.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{FOCUS_COLOR, dim};
use crate::presentation::view_models::watch::{FeedFilter, Pane, StatusBarVm, WatchScreenVm};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let s = &vm.status;
    let toggles = toggles(s);

    let mut spans = vec![Span::styled(
        format!(" {} ", mode_label(vm)),
        Style::default()
            .fg(Color::Black)
            .bg(FOCUS_COLOR)
            .add_modifier(Modifier::BOLD),
    )];
    spans.push(Span::raw(" "));
    if let Some(t) = &vm.toast {
        spans.push(Span::styled(
            t.clone(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        info(s, &toggles, &mut spans);
    }
    let left = Line::from(spans);

    // Hints take what the left part leaves, dropping optional items first; the
    // essential ones (`↵ open`, `? help`, `Esc back`, `Esc:reset`) always show.
    let items = hints(vm.focus_pane, !toggles.is_empty());
    let essential = fit(&items, 0);
    let avail = (area.width as usize)
        .saturating_sub(left.width() + 1)
        .max(essential.chars().count() + 1);
    let text = fit(&items, avail.saturating_sub(1));
    let hints_w = (text.chars().count() as u16 + 1).min(area.width);
    let [l, r] = Layout::horizontal([Constraint::Min(0), Constraint::Length(hints_w)]).areas(area);
    f.render_widget(Paragraph::new(left), l);
    f.render_widget(Paragraph::new(Line::styled(text, dim()).right_aligned()), r);
}

/// One key hint; optional ones are dropped (lowest priority first) when the
/// status bar is too narrow.
#[derive(Debug, Clone, Copy)]
pub struct Hint {
    pub text: &'static str,
    /// 0 = essential; higher is dropped earlier.
    pub drop_rank: u8,
}

const fn hint(text: &'static str, drop_rank: u8) -> Hint {
    Hint { text, drop_rank }
}

/// Key hints of the focused pane; `Esc:reset` only on the tree with toggles active.
pub fn hints(focus: Pane, toggles_active: bool) -> Vec<Hint> {
    let mut out = match focus {
        Pane::Tree => vec![
            hint("↵ open", 0),
            hint("space fold", 1),
            hint("f msgs", 2),
            hint("d done", 3),
            hint("? help", 0),
        ],
        Pane::Timeline => vec![
            hint("j/k scroll", 1),
            hint("G follow", 2),
            hint("Esc back", 0),
        ],
        Pane::Feed => vec![
            hint("j/k scroll", 1),
            hint("f filter", 2),
            hint("Esc back", 0),
        ],
    };
    if focus == Pane::Tree && toggles_active {
        out.push(hint("Esc:reset", 0));
    }
    out
}

/// Join `items` with ` · `, dropping optional items (highest rank first) until
/// the text fits in `width` columns; essential items are always kept.
pub fn fit(items: &[Hint], width: usize) -> String {
    let join = |max_rank: u8| {
        items
            .iter()
            .filter(|h| h.drop_rank <= max_rank)
            .map(|h| h.text)
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let top = items.iter().map(|h| h.drop_rank).max().unwrap_or(0);
    (0..=top)
        .rev()
        .map(join)
        .find(|t| t.chars().count() <= width)
        .unwrap_or_else(|| join(0))
}

/// `AGENTS`, `TIMELINE · <agent>`, `MESSAGES`.
fn mode_label(vm: &WatchScreenVm) -> String {
    match vm.focus_pane {
        Pane::Tree => "AGENTS".to_string(),
        Pane::Timeline => format!("TIMELINE · {}", vm.focus.title),
        Pane::Feed => "MESSAGES".to_string(),
    }
}

/// Active view toggles (the ones Esc on the tree resets, plus auto-select).
fn toggles(s: &StatusBarVm) -> Vec<String> {
    let mut out = Vec::new();
    if s.feed_filter == FeedFilter::Selected {
        out.push("feed:selected".to_string());
    }
    if s.hide_done {
        out.push("hide done".to_string());
    }
    if s.collapsed > 0 {
        out.push(format!("{} collapsed", s.collapsed));
    }
    out
}

fn info(s: &StatusBarVm, toggles: &[String], spans: &mut Vec<Span<'static>>) {
    let sep = || Span::styled(" · ", dim());
    spans.push(Span::raw(format!(
        "{} agents ({} running, {} idle)",
        s.agents, s.running, s.idle
    )));
    if s.hidden > 0 {
        spans.push(Span::styled(format!(" {} hidden", s.hidden), dim()));
    }
    if s.diagnostics > 0 {
        spans.push(sep());
        spans.push(Span::styled(
            format!("{} diag", s.diagnostics),
            Style::default().fg(Color::Yellow),
        ));
    }
    if s.errors > 0 {
        spans.push(sep());
        let msg = s.last_error.as_deref().unwrap_or_default();
        spans.push(Span::styled(
            format!("{} err: {msg}", s.errors),
            Style::default().fg(Color::Red),
        ));
    }
    let mut shown: Vec<&str> = toggles.iter().map(String::as_str).collect();
    if s.auto_select {
        shown.push("auto");
    }
    if !shown.is_empty() {
        spans.push(sep());
        spans.push(Span::styled(
            shown.join(" "),
            Style::default().fg(FOCUS_COLOR),
        ));
    }
}
