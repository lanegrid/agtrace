//! One-line status bar.
//!
//! ```text
//!  TIMELINE · lead  9 agents (4 running, 3 idle) · hide done      j/k scroll · G follow · Esc back
//! ```
//!
//! Left: the mode label (screen, focused pane or detail section, and whose
//! timeline / detail), then the live toast, or else agent counts, diagnostics,
//! errors and active toggles. Right: key hints for the screen / focused pane.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{FOCUS_COLOR, dim};
use crate::presentation::view_models::watch::{
    FeedFilter, Pane, Screen, StatusBarVm, WatchScreenVm,
};

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
    if s.filter_editing {
        spans.push(Span::styled(
            format!("/{}▏", s.filter),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if !s.filter.is_empty() {
            let noun = if s.matches == 1 { "match" } else { "matches" };
            spans.push(Span::styled(format!("  {} {noun}", s.matches), dim()));
        }
    } else if let Some(t) = &vm.toast {
        spans.push(Span::styled(
            t.clone(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        info(s, &toggles, vm.screen != Screen::Detail, &mut spans);
    }
    let left = Line::from(spans);

    // Hints take what the left part leaves, dropping optional items first; the
    // essential ones (`↵ detail`, `? help`, `Esc back`, `Esc:reset`) always show.
    let items = if s.filter_editing {
        vec![
            hint("↑/↓ select", 1),
            hint("↵ open", 0),
            hint("Esc clear", 0),
        ]
    } else {
        hints(
            vm.screen,
            vm.focus_pane,
            !toggles.is_empty(),
            !s.filter.is_empty(),
        )
    };
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

/// Key hints of the screen / focused pane; `Esc:reset` only on the home views
/// (overview, agents tree) with toggles active (`Esc:clear filter` while a `/`
/// filter is set: Esc clears it first).
pub fn hints(screen: Screen, focus: Pane, toggles_active: bool, filtered: bool) -> Vec<Hint> {
    let (mut out, home) = match (screen, focus) {
        (Screen::Overview, _) => (
            vec![
                hint("↵ detail", 0),
                hint("/ find", 1),
                hint("+/- window", 3),
                hint("space fold", 4),
                hint("d done", 5),
                hint("2 agents", 2),
                hint("? help", 0),
            ],
            true,
        ),
        (Screen::Detail, _) => (
            vec![
                hint("i/n/r/t section", 1),
                hint("j/k scroll", 2),
                hint("J/K agent", 4),
                hint("G/g end/top", 3),
                hint("Esc back", 0),
            ],
            false,
        ),
        (Screen::Agents, Pane::Tree) => (
            vec![
                hint("↵ detail", 0),
                hint("/ find", 2),
                hint("space fold", 3),
                hint("f msgs", 4),
                hint("d done", 5),
                hint("1 overview", 1),
                hint("? help", 0),
            ],
            true,
        ),
        (Screen::Agents, Pane::Timeline) => (
            vec![
                hint("↵ detail", 1),
                hint("j/k scroll", 2),
                hint("G follow", 3),
                hint("Esc back", 0),
            ],
            false,
        ),
        (Screen::Agents, Pane::Feed) => (
            vec![
                hint("j/k scroll", 1),
                hint("f filter", 2),
                hint("Esc back", 0),
            ],
            false,
        ),
    };
    if home && filtered {
        out.push(hint("Esc:clear filter", 0));
    } else if home && toggles_active {
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

/// `OVERVIEW`, `AGENTS`, `TIMELINE · <agent>`, `MESSAGES`, `DETAIL · <SECTION>`
/// (the detail names its agent in its title), `FIND` while typing a filter.
fn mode_label(vm: &WatchScreenVm) -> String {
    if vm.status.filter_editing {
        return "FIND".to_string();
    }
    match (vm.screen, vm.focus_pane) {
        (Screen::Overview, _) => "OVERVIEW".to_string(),
        (Screen::Detail, _) => match &vm.detail {
            Some(d) => format!("DETAIL · {}", d.section.title().to_uppercase()),
            None => "DETAIL".to_string(),
        },
        (Screen::Agents, Pane::Tree) => "AGENTS".to_string(),
        (Screen::Agents, Pane::Timeline) => {
            format!("TIMELINE · {}", super::style::clip(&vm.focus.title, 24))
        }
        (Screen::Agents, Pane::Feed) => "MESSAGES".to_string(),
    }
}

/// Active view toggles (the ones Esc on the tree resets, plus auto-select); the
/// `/` filter is named in the pane title instead.
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

/// Agent counts (not on the detail screen, where the key hints need the room),
/// diagnostics, errors and active toggles.
fn info(s: &StatusBarVm, toggles: &[String], counts: bool, spans: &mut Vec<Span<'static>>) {
    let base = spans.len();
    // Separator before every item but the first.
    let sep = |spans: &Vec<Span<'static>>| (spans.len() > base).then(|| Span::styled(" · ", dim()));
    if counts {
        spans.push(Span::raw(format!(
            "{} agents ({} running, {} idle)",
            s.agents, s.running, s.idle
        )));
        if s.hidden > 0 {
            spans.push(Span::styled(format!(" {} hidden", s.hidden), dim()));
        }
    }
    if s.diagnostics > 0 {
        spans.extend(sep(spans));
        spans.push(Span::styled(
            format!("{} diag", s.diagnostics),
            Style::default().fg(Color::Yellow),
        ));
    }
    if s.errors > 0 {
        spans.extend(sep(spans));
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
        spans.extend(sep(spans));
        spans.push(Span::styled(
            shown.join(" "),
            Style::default().fg(FOCUS_COLOR),
        ));
    }
}
