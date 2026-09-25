//! One-line status bar.
//!
//! ```text
//!  yohaku-studio › Projects制作… › v8fix1 · NOW  4 sessions (3 live) · 26 agents (2 run)   j/k scroll · ← nav · ? help
//! ```
//!
//! Left: the breadcrumb of the selection (scope › session › agent or group, plus
//! the detail section), then the live toast, or else the session and agent
//! counts, folded agents, diagnostics, errors and active toggles. Right: key
//! hints for the focused pane.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{FOCUS_COLOR, clip, dim, text_width};
use crate::presentation::view_models::watch::{
    ContentVm, NARROW_WIDTH, Pane, StatusBarVm, WatchScreenVm,
};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let s = &vm.status;
    let mut spans = vec![Span::styled(
        format!(" {} ", mode_label(vm, area.width as usize)),
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
        info(s, &mut spans);
    }
    let left = Line::from(spans);

    // Hints take what the left part leaves, dropping optional items first; the
    // left part is clipped to the rest.
    let items = if s.filter_editing {
        vec![
            hint("↑/↓ select", 1),
            hint("↵ keep", 0),
            hint("Esc clear", 0),
        ]
    } else {
        hints(
            vm.focus_pane,
            matches!(vm.content, ContentVm::Agent { .. }),
            !s.filter.is_empty(),
            area.width < NARROW_WIDTH,
            vm.nav_hidden,
        )
    };
    let essential = fit(&items, 0);
    let avail = (area.width as usize)
        .saturating_sub(left.width() + 1)
        .max(essential.chars().count() + 1);
    let text = fit(&items, avail.saturating_sub(1));
    let hints_w = (text.chars().count() as u16 + 1).min(area.width);
    let [l, r] = Layout::horizontal([Constraint::Min(0), Constraint::Length(hints_w)]).areas(area);
    let left = super::overview::clip_line(left.spans, l.width.saturating_sub(1) as usize);
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

/// Key hints of the focused pane: `detail` when the content is an agent's detail
/// (section keys), `narrow` below [`NARROW_WIDTH`] (`s` hides / shows the
/// navigator), `Esc:clear filter` while a `/` filter is set.
pub fn hints(
    focus: Pane,
    detail: bool,
    filtered: bool,
    narrow: bool,
    nav_hidden: bool,
) -> Vec<Hint> {
    let mut out = match focus {
        Pane::Navigator => {
            let mut out = vec![
                hint("↑↓ move", 3),
                hint("→ open", 0),
                hint("← back", 2),
                hint("↵ read", 1),
            ];
            if detail {
                out.push(hint("i/n/r/t section", 5));
            }
            out.extend([hint("/ find", 4), hint("d done", 6)]);
            out
        }
        Pane::Content => {
            let mut out = vec![hint("j/k scroll", 1)];
            if detail {
                out.push(hint("i/n/r/t section", 2));
            }
            out.extend([
                hint("G/g end/top", 4),
                hint("Tab msgs", 3),
                hint("← nav", 0),
            ]);
            out
        }
        Pane::Feed => vec![hint("j/k scroll", 1), hint("G follow", 2), hint("← nav", 0)],
    };
    if narrow {
        // Hidden: how to get it back is essential.
        out.push(if nav_hidden {
            hint("s nav", 0)
        } else {
            hint("s hide nav", 5)
        });
    }
    if filtered {
        out.push(hint("Esc:clear filter", 0));
    }
    out.push(hint("? help", 0));
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

/// The breadcrumb (`scope › session › agent · SECTION`), `FIND` while typing a
/// filter; crumbs are clipped to fit a third of the terminal.
fn mode_label(vm: &WatchScreenVm, width: usize) -> String {
    if vm.status.filter_editing {
        return "FIND".to_string();
    }
    let section = match (&vm.content, &vm.detail) {
        (ContentVm::Agent { .. }, Some(d)) => format!(" · {}", d.section.title().to_uppercase()),
        _ => String::new(),
    };
    let crumbs = &vm.status.crumbs;
    let seps = 3 * crumbs.len().saturating_sub(1);
    let budget = (width * 2 / 5)
        .max(24)
        .saturating_sub(section.chars().count() + seps);
    // Shorten the longest crumb first (down to 6 columns each).
    let mut widths: Vec<usize> = crumbs.iter().map(|c| text_width(c)).collect();
    while widths.iter().sum::<usize>() > budget {
        let Some((i, w)) = widths
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(i, w)| (*w, usize::MAX - i))
        else {
            break;
        };
        if w <= 6 {
            break;
        }
        widths[i] = w - 1;
    }
    let text = crumbs
        .iter()
        .zip(widths)
        .map(|(c, w)| clip(c, w))
        .collect::<Vec<_>>()
        .join(" › ");
    format!("{text}{section}")
}

/// Session and agent counts, folded agents, diagnostics, errors and active
/// toggles.
fn info(s: &StatusBarVm, spans: &mut Vec<Span<'static>>) {
    let noun = if s.sessions == 1 {
        "session"
    } else {
        "sessions"
    };
    spans.push(Span::raw(format!(
        "{} {noun} ({} live)",
        s.sessions, s.live
    )));
    spans.push(Span::styled(" · ", dim()));
    spans.push(Span::raw(format!(
        "{} agents ({} run)",
        s.agents, s.running
    )));
    if s.folded > 0 {
        spans.push(Span::styled(format!(" · {} folded", s.folded), dim()));
    }
    let sep = || Span::styled(" · ", dim());
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
    let mut shown = Vec::new();
    if s.show_done {
        shown.push("done shown");
    }
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
