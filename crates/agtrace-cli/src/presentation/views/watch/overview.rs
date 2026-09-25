//! Overview screen: every agent of every root with status, context bar, activity
//! lane and what it is doing now, plus a compact message feed.
//!
//! ```text
//! ┏ ▶ Overview · project demo · last 60m ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃   agent          status  context     activity · 1 cell = 2m       now          ┃
//! ┃ s-lead · busy 5m · claude-opus-5-5[1m] · ctx 42% of 1.0M [1m] · ⟲1 · 5 agents  ┃
//! ┃▶ s-lead          ● busy  ███░░░ 42%  ▂▁  ▃▃·····▂   ▸ Bash mise run test (10s)┃
//! ┃  ├ T audit-A     ○ idle  █░░░░░ 12%    ▂  ▁         idle 3m                  ┃
//! ```

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{
    clip, ctx_bar, ctx_style, dim, elapsed, lane_style, pane_block, short, status_glyph,
    status_style, status_word, text_width,
};
use crate::presentation::presenters::watch::tokens;
use crate::presentation::view_models::watch::{
    NowVm, OverviewRowVm, RootHeaderVm, StatusVm, WatchScreenVm,
};

/// Columns of a status cell (`⊘ kill`).
const STATUS_W: usize = 6;
/// Cells of the context bar; the column adds ` 100%`.
const BAR_W: usize = 6;
const CTX_W: usize = BAR_W + 5;
/// Selection marker (`▶ `).
const MARK_W: usize = 2;

/// Column widths of the overview table for an inner width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Columns {
    pub name: usize,
    pub lane: usize,
    pub now: usize,
}

/// Name `clamp(22%, 14, 28)`, now `clamp(24%, 14, 40)`, the lane takes the rest
/// (dropped below 8 cells).
pub fn columns(inner_width: usize) -> Columns {
    let name = (inner_width * 22 / 100).clamp(14, 28);
    let now_min = (inner_width * 24 / 100).clamp(14, 40);
    let fixed = MARK_W + name + 1 + STATUS_W + 1 + CTX_W + 1;
    let lane = inner_width.saturating_sub(fixed + now_min + 1);
    let lane = if lane < 8 { 0 } else { lane };
    let now = inner_width.saturating_sub(fixed + if lane > 0 { lane + 1 } else { 0 });
    Columns { name, lane, now }
}

/// Main area, feed and status line of the overview screen.
pub fn areas(area: Rect) -> (Rect, Rect, Rect) {
    let feed_h = (area.height / 5).clamp(4, 8);
    let [main, feed, status] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(feed_h),
        Constraint::Length(1),
    ])
    .areas(area);
    (main, feed, status)
}

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let Some(ov) = &vm.overview else { return };
    let mut title = vec![Span::raw(" Overview ")];
    if !vm.status.scope.is_empty() {
        title.push(Span::raw(format!("· {} ", vm.status.scope)));
    }
    title.push(Span::raw(format!("· activity: last {} ", ov.window)));
    let block = pane_block(true, title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    if ov.rows.is_empty() {
        f.render_widget(Paragraph::new(Line::styled(" no agents yet", dim())), inner);
        return;
    }
    let cols = columns(inner.width as usize);
    let [head, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    f.render_widget(Paragraph::new(header_line(cols, &ov.cell)), head);

    // Root header lines are interleaved with the rows; keep the selected row visible.
    let mut lines: Vec<Line> = Vec::with_capacity(ov.rows.len() * 2);
    let mut selected_line = 0;
    for r in &ov.rows {
        if let Some(h) = &r.root {
            lines.push(root_line(h, inner.width as usize));
        }
        if r.selected {
            selected_line = lines.len();
        }
        lines.push(row_line(r, cols));
    }
    let height = body.height as usize;
    let start = if selected_line < height {
        0
    } else {
        selected_line + 1 - height
    };
    let visible: Vec<Line> = lines.into_iter().skip(start).take(height).collect();
    f.render_widget(Paragraph::new(visible), body);
}

fn pad(s: &str, width: usize) -> String {
    let s = clip(s, width);
    let w = text_width(&s);
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

fn header_line(cols: Columns, cell: &str) -> Line<'static> {
    let mut text = format!(
        "{}{} {} {} ",
        " ".repeat(MARK_W),
        pad("agent", cols.name),
        pad("status", STATUS_W),
        pad("context", CTX_W)
    );
    if cols.lane > 0 {
        text.push_str(&pad(&format!("activity {cell}/cell"), cols.lane));
        text.push(' ');
    }
    text.push_str("now");
    Line::styled(text, dim())
}

fn root_line(h: &RootHeaderVm, width: usize) -> Line<'static> {
    let sep = || Span::styled(" · ", dim());
    let mut spans = vec![Span::raw(" ")];
    if h.provider == "codex" {
        spans.push(Span::styled("codex ", dim()));
    }
    spans.push(Span::styled(
        h.label.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    spans.push(sep());
    spans.push(Span::styled(status_word(h.status), status_style(h.status)));
    if let Some(age) = h.age_secs {
        spans.push(Span::styled(format!(" · up {}", short(age)), dim()));
    }
    if let Some(m) = &h.model {
        spans.push(sep());
        spans.push(Span::styled(m.clone(), dim()));
    }
    if let Some(c) = &h.ctx {
        spans.push(sep());
        spans.push(Span::raw("ctx "));
        spans.push(Span::styled(ctx_bar(c.pct, 10), ctx_style(c.pct)));
        spans.push(Span::styled(format!(" {}%", c.pct), ctx_style(c.pct)));
        spans.push(Span::raw(format!(
            " of {} [{}]",
            tokens(c.window_tokens),
            c.provenance
        )));
    }
    if h.compactions > 0 {
        spans.push(sep());
        spans.push(Span::styled(
            format!("⟲{}", h.compactions),
            Style::default().fg(Color::Yellow),
        ));
    }
    if h.agents > 1 {
        spans.push(sep());
        spans.push(Span::styled(
            format!("{} agents ({} running)", h.agents, h.running),
            dim(),
        ));
    }
    // Least important: clipped first on narrow terminals.
    if let Some(e) = &h.effort {
        spans.push(sep());
        spans.push(Span::styled(format!("effort {e}"), dim()));
    }
    clip_line(spans, width)
}

/// Drop trailing spans that do not fit (and clip the last one).
fn clip_line(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let mut out = Vec::new();
    let mut w = 0;
    for s in spans {
        let sw = text_width(&s.content);
        if w + sw > width {
            let rest = width.saturating_sub(w);
            if rest > 1 {
                out.push(Span::styled(clip(&s.content, rest), s.style));
            }
            break;
        }
        w += sw;
        out.push(s);
    }
    Line::from(out)
}

fn name_cell(r: &OverviewRowVm, width: usize) -> Vec<Span<'static>> {
    let mut prefix = String::new();
    for g in &r.guides {
        prefix.push_str(if *g { "│ " } else { "  " });
    }
    if r.depth > 0 {
        prefix.push_str(if r.is_last_sibling { "└ " } else { "├ " });
    }
    if r.collapsed {
        prefix.push_str(&format!("▸+{} ", r.hidden_descendants));
    }
    if let Some(b) = r.badge {
        prefix.push_str(&format!("{b} "));
    }
    if r.depth == 0 && r.provider == "codex" {
        prefix.push_str("codex ");
    }
    let label_w = width.saturating_sub(text_width(&prefix));
    let label = pad(&r.label, label_w);
    let label_style = if r.selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    vec![
        Span::styled(clip(&prefix, width), dim()),
        Span::styled(label, label_style),
    ]
}

fn row_line(r: &OverviewRowVm, cols: Columns) -> Line<'static> {
    let mut spans = vec![Span::raw(if r.selected { "▶ " } else { "  " })];
    spans.extend(name_cell(r, cols.name));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        pad(
            &format!("{} {}", status_glyph(r.status), status_word(r.status)),
            STATUS_W,
        ),
        status_style(r.status),
    ));
    spans.push(Span::raw(" "));
    match &r.ctx {
        Some(c) => {
            spans.push(Span::styled(ctx_bar(c.pct, BAR_W), ctx_style(c.pct)));
            spans.push(Span::styled(
                format!("{:>5}", format!("{}%", c.pct)),
                ctx_style(c.pct),
            ));
        }
        None => spans.push(Span::raw(" ".repeat(CTX_W))),
    }
    spans.push(Span::raw(" "));
    if cols.lane > 0 {
        let cells = r.lane.chars().count();
        spans.push(Span::raw(" ".repeat(cols.lane.saturating_sub(cells))));
        for (g, t) in r.lane.chars().zip(r.lane_tones.chars()) {
            spans.push(Span::styled(g.to_string(), lane_style(t)));
        }
        spans.push(Span::raw(" "));
    }
    spans.extend(now_spans(&r.now, cols.now));
    let line = Line::from(spans);
    if r.selected {
        line.style(Style::default().add_modifier(Modifier::REVERSED))
    } else {
        line
    }
}

pub fn now_spans(now: &NowVm, width: usize) -> Vec<Span<'static>> {
    let spans = match now {
        NowVm::Tool {
            name,
            summary,
            elapsed_secs,
        } => vec![
            Span::styled(format!("▸ {name} "), Style::default().fg(Color::Green)),
            Span::raw(summary.clone()),
            Span::styled(format!(" ({})", elapsed(*elapsed_secs)), dim()),
        ],
        NowVm::Task { text } => vec![
            Span::styled("▸ ", Style::default().fg(Color::Cyan)),
            Span::styled(text.clone(), Style::default().fg(Color::Cyan)),
        ],
        NowVm::Said { text } => vec![Span::styled(format!("\"{text}\""), dim())],
        NowVm::Idle { secs } => vec![Span::styled(
            format!("idle {}", short(*secs)),
            Style::default().fg(Color::Blue),
        )],
        NowVm::Result { text } => vec![
            Span::styled("result: ", dim()),
            Span::raw(format!("\"{text}\"")),
        ],
        NowVm::Ended {
            status,
            secs,
            reason,
        } => {
            let word = match status {
                StatusVm::Done => "done",
                StatusVm::Killed => "killed",
                StatusVm::Failed => "failed",
                _ => "ended",
            };
            let mut text = word.to_string();
            if let Some(s) = secs {
                text.push_str(&format!(" {} ago", short(*s)));
            }
            let mut out = vec![Span::styled(text, status_style(*status))];
            if let Some(r) = reason {
                out.push(Span::styled(format!(" · {r}"), dim()));
            }
            out
        }
        NowVm::None => Vec::new(),
    };
    clip_line(spans, width).spans
}
