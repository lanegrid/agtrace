//! Session lines: the overview's summary block and the content of the
//! navigator's older-sessions group.
//!
//! ```text
//! ┌ Older sessions · 2 ───────────────────────────────────────────────────────┐
//! │   provider session                    state        agents     ctx  last  now │
//! │   codex    Review the parser change   ✓ ended      4          12%  2h    …   │
//! │   claude   e76c4850                   ✓ ended      1            —  3d    …   │
//! ```

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::overview::{clip_line, now_spans, pad};
use super::style::{ctx_style, dim, more_marks, pane_block, short, status_style};
use crate::presentation::view_models::watch::{
    Pane, SectionMetrics, SessionRowVm, SessionStateVm, SessionsVm, StatusVm, WatchScreenVm,
};

/// Left margin.
const MARK_W: usize = 3;
const PROVIDER_W: usize = 9;
/// `○ idle (bg)`.
const STATE_W: usize = 11;
/// `22 (10 run)`.
const AGENTS_W: usize = 11;
const CTX_W: usize = 4;
const LAST_W: usize = 4;

/// Column widths of a session line for an inner width (`now` = 0: dropped).
#[derive(Debug, Clone, Copy)]
struct Columns {
    name: usize,
    now: usize,
}

fn columns(width: usize) -> Columns {
    let fixed = MARK_W + PROVIDER_W + 1 + STATE_W + 1 + AGENTS_W + 1 + CTX_W + 1 + LAST_W + 1;
    let rest = width.saturating_sub(fixed);
    let name = (rest * 55 / 100).clamp(20, 44).min(rest);
    let now = rest.saturating_sub(name + 1);
    if now < 14 {
        // Too narrow for the "now" column: the name matters more.
        return Columns { name: rest, now: 0 };
    }
    Columns { name, now }
}

/// `● busy`, `○ idle (bg)`, `✓ ended`.
pub(super) fn state_cell(r: &SessionRowVm) -> (String, Style) {
    let (glyph, word, status) = match r.state {
        SessionStateVm::Busy => ("●", "busy", StatusVm::Running),
        SessionStateVm::Idle => ("○", "idle", StatusVm::Idle),
        SessionStateVm::Recent | SessionStateVm::Older => ("✓", "ended", StatusVm::Done),
    };
    let bg = if r.bg { " (bg)" } else { "" };
    (format!("{glyph} {word}{bg}"), status_style(status))
}

fn last_cell(secs: Option<i64>) -> String {
    match secs {
        Some(s) if s < 60 => "now".to_string(),
        Some(s) => short(s),
        None => "—".to_string(),
    }
}

/// One session line: provider, name, state, agents, context, last write and
/// (when there is room) what the root does now.
pub fn session_line(r: &SessionRowVm, width: usize) -> Line<'static> {
    let cols = columns(width);
    let mark = " ".repeat(MARK_W);
    let provider = if r.provider == "codex" {
        "codex"
    } else {
        "claude"
    };
    let name_style = if r.name_is_id {
        dim()
    } else {
        Style::default()
    };
    let (state, state_style) = state_cell(r);
    let agents = if !r.has_transcript {
        "—".to_string()
    } else if r.agents <= 1 {
        r.agents.to_string()
    } else {
        format!("{} ({} run)", r.agents, r.running)
    };
    let mut spans = vec![
        Span::raw(mark),
        Span::styled(pad(provider, PROVIDER_W), dim()),
        Span::styled(pad(&r.name, cols.name), name_style),
        Span::raw(" "),
        Span::styled(pad(&state, STATE_W), state_style),
        Span::raw(" "),
        Span::raw(pad(&agents, AGENTS_W)),
        Span::raw(" "),
    ];
    match &r.ctx {
        Some(c) => spans.push(Span::styled(
            format!("{:>w$}", format!("{}%", c.pct), w = CTX_W),
            ctx_style(c.pct),
        )),
        None => spans.push(Span::styled(format!("{:>w$}", "—", w = CTX_W), dim())),
    }
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        format!("{:>w$}", last_cell(r.last_secs), w = LAST_W),
        dim(),
    ));
    spans.push(Span::raw(" "));
    if cols.now > 0 {
        if r.has_transcript {
            spans.extend(now_spans(&r.now, cols.now));
        } else {
            spans.push(Span::styled(
                super::style::clip("no transcript yet", cols.now),
                dim(),
            ));
        }
    }
    clip_line(spans, width)
}

fn header_line(width: usize) -> Line<'static> {
    let cols = columns(width);
    let mut text = format!(
        "{}{}{} {} {} {:>cw$} {:>lw$} ",
        " ".repeat(MARK_W),
        pad("provider", PROVIDER_W),
        pad("session", cols.name),
        pad("state", STATE_W),
        pad("agents", AGENTS_W),
        "ctx",
        "last",
        cw = CTX_W,
        lw = LAST_W,
    );
    if cols.now > 0 {
        text.push_str("now");
    }
    Line::styled(text, dim())
}

/// `3 live, 1 recent, 4 older`.
pub fn counts(s: &SessionsVm) -> String {
    let mut parts = vec![format!("{} live", s.live)];
    if s.recent > 0 {
        parts.push(format!("{} recent", s.recent));
    }
    if s.older > 0 {
        parts.push(format!("{} older", s.older));
    }
    parts.join(", ")
}

/// Lines of the older sessions (column header first).
fn older_lines(vm: &WatchScreenVm, width: usize) -> Vec<Line<'static>> {
    vm.sessions
        .rows
        .iter()
        .filter(|r| r.state == SessionStateVm::Older)
        .map(|r| session_line(r, width))
        .collect()
}

/// Scrolling lines of the older sessions and their visible height.
pub fn metrics(area: Rect, vm: &WatchScreenVm) -> SectionMetrics {
    let inner = pane_block(false, Vec::new()).inner(area);
    SectionMetrics {
        total: older_lines(vm, inner.width as usize).len(),
        height: (inner.height as usize).saturating_sub(1),
    }
}

/// Content of the navigator's older-sessions group.
pub fn render_older(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let s = &vm.sessions;
    let title = vec![Span::raw(format!(" Older sessions · {} ", s.older))];
    let block = pane_block(vm.focus_pane == Pane::Content, title);
    let inner = block.inner(area);
    if inner.height == 0 {
        f.render_widget(block, area);
        return;
    }
    let width = inner.width as usize;
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    f.render_widget(Paragraph::new(header_line(width)), head);
    let lines = older_lines(vm, width);
    let height = body.height as usize;
    let total = lines.len();
    let start = vm.content_scroll.min(total.saturating_sub(height));
    let visible: Vec<Line> = lines.into_iter().skip(start).take(height).collect();
    let block = match more_marks(start, visible.len(), total) {
        Some(marks) => block.title_bottom(marks),
        None => block,
    };
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(visible), body);
}

/// Summary block at the top of the overview: a title line and up to `max` lines of
/// live / recent sessions (`+N more` when they do not fit). Empty when there is
/// only one session.
pub fn summary_lines(s: &SessionsVm, width: usize, max: usize) -> Vec<Line<'static>> {
    if s.total() <= 1 || max < 2 {
        return Vec::new();
    }
    let rows: Vec<&SessionRowVm> = s
        .rows
        .iter()
        .filter(|r| r.state != SessionStateVm::Older)
        .collect();
    let mut out = vec![Line::from(vec![
        Span::styled(
            format!(" Sessions · {}", counts(s)),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  · pick one in the navigator", dim()),
    ])];
    let room = max - 1;
    let shown = if rows.len() > room {
        room.saturating_sub(1)
    } else {
        rows.len()
    };
    for r in rows.iter().take(shown) {
        out.push(session_line(r, width));
    }
    let more = rows.len() - shown;
    if more > 0 {
        out.push(Line::styled(
            format!("{}+{more} more in the navigator", " ".repeat(MARK_W)),
            dim(),
        ));
    }
    out
}
