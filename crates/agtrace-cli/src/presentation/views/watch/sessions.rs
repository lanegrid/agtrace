//! Sessions screen (`0`) and the session lines shared with the overview's summary
//! block.
//!
//! ```text
//! ┏ ▶ Sessions · project demo · since 2h · 2 live, 1 recent ━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃   provider session                    state        agents     ctx  last  now    ┃
//! ┃▶  claude   s-lead                     ● busy       5 (3 run)  42%  now   ▸ Bash ┃
//! ┃ ◆ codex    Review the parser change   ○ idle       4 (2 run)  12%  2m    idle 2m┃
//! ┃   claude   e76c4850                   ○ idle (bg)  —            —  28m   no tra…┃
//! ┃   ▸ 3 older sessions — space to list                                            ┃
//! ```
//!
//! `▶` marks the cursor, `◆` the focused session.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::overview::{clip_line, now_spans, pad};
use super::style::{FOCUS_COLOR, ctx_style, dim, more_marks, pane_block, short, status_style};
use crate::presentation::view_models::watch::{
    SessionRowVm, SessionStateVm, SessionsVm, StatusVm, WatchScreenVm,
};

/// Cursor / focus marks.
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
fn state_cell(r: &SessionRowVm) -> (String, Style) {
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

/// One session line: marks, provider, name, state, agents, context, last write
/// and (when there is room) what the root does now.
pub fn session_line(r: &SessionRowVm, width: usize, cursor: bool) -> Line<'static> {
    let cols = columns(width);
    let mark = format!(
        "{}{} ",
        if cursor && r.selected { "▶" } else { " " },
        if r.focused { "◆" } else { " " }
    );
    let provider = if r.provider == "codex" {
        "codex"
    } else {
        "claude"
    };
    let name_style = if r.name_is_id {
        dim()
    } else if r.selected && cursor {
        Style::default().add_modifier(Modifier::BOLD)
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
        Span::styled(mark, Style::default().fg(FOCUS_COLOR)),
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
    let line = clip_line(spans, width);
    if cursor && r.selected {
        line.style(Style::default().add_modifier(Modifier::REVERSED))
    } else {
        line
    }
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

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let s = &vm.sessions;
    let mut title = vec![Span::raw(" Sessions ")];
    if !s.scope.is_empty() {
        title.push(Span::raw(format!("· {} ", s.scope)));
    }
    title.push(Span::raw(format!("· {} ", counts(s))));
    let block = pane_block(true, title);
    let inner = block.inner(area);
    if inner.height == 0 {
        f.render_widget(block, area);
        return;
    }
    if s.total() == 0 {
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(Line::styled(" no sessions in scope yet", dim())),
            inner,
        );
        return;
    }
    let width = inner.width as usize;
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    f.render_widget(Paragraph::new(header_line(width)), head);
    let mut lines: Vec<Line> = s
        .rows
        .iter()
        .map(|r| session_line(r, width, true))
        .collect();
    if s.older_folded > 0 {
        let noun = if s.older_folded == 1 {
            "session"
        } else {
            "sessions"
        };
        lines.push(Line::styled(
            format!(
                "{}▸ {} older {noun} — space to list",
                " ".repeat(MARK_W),
                s.older_folded
            ),
            dim(),
        ));
    }
    let height = body.height as usize;
    let sel = s.selected_index().unwrap_or(0);
    let start = if sel < height { 0 } else { sel + 1 - height };
    let total = lines.len();
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
/// only one session or a session is focused.
pub fn summary_lines(s: &SessionsVm, width: usize, max: usize) -> Vec<Line<'static>> {
    if s.focus.is_some() || s.total() <= 1 || max < 2 {
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
        Span::styled("  · 0 to list / focus one", dim()),
    ])];
    let room = max - 1;
    let shown = if rows.len() > room {
        room.saturating_sub(1)
    } else {
        rows.len()
    };
    for r in rows.iter().take(shown) {
        out.push(session_line(r, width, false));
    }
    let more = rows.len() - shown;
    if more > 0 {
        out.push(Line::styled(
            format!("{}+{more} more — 0 to list", " ".repeat(MARK_W)),
            dim(),
        ));
    }
    out
}
