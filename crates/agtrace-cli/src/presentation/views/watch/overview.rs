//! Overview content: for the top node, a summary of the sessions, then per
//! session a header line and its agents in place with status, context bar,
//! activity lane and what each is doing now (a session's finished agents fold
//! into one line); for a session node, that session alone; for a folded group,
//! its items.
//!
//! ```text
//! ┌ Overview · project demo · activity: last 60m ──────────────────────────────────┐
//! │ Sessions · 2 live                                                               │
//! │   claude  s-lead          ● busy      5 (3 run)  42%  now  ▸ Bash mise run …    │
//! │   codex   Review parser   ○ idle      4 (2 run)  12%   2m  idle 2m              │
//! │   agent          status  context     activity · 1 cell = 2m       now           │
//! │ s-lead · busy 5m · claude-opus-5-5[1m] · ctx 42% of 1.0M [1m] · ⟲1 · 5 agents   │
//! │   s-lead          ● busy  ███░░░ 42%  ▂▁  ▃▃·····▂   ▸ Bash mise run test (10s) │
//! │   ├ T audit-A     ○ idle  █░░░░░ 12%    ▂  ▁         idle 3m                   │
//! │     ✓ 3 finished · 1 earlier transcript  (d to show)                           │
//! ```
//!
//! The rows below the column header scroll (content focus: ↑/↓, PgUp/PgDn, g/G).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{
    clip, ctx_bar, ctx_style, dim, elapsed, filter_title, lane_style, more_marks, pane_block,
    short, status_glyph, status_style, status_word, text_width,
};
use crate::presentation::presenters::watch::tokens;
use crate::presentation::view_models::watch::{
    ContentVm, FoldedVm, NowVm, OverviewRowVm, Pane, RootHeaderVm, SectionMetrics, StatusVm,
    WatchScreenVm,
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

/// Block title of the content: what the selected node is.
fn title(vm: &WatchScreenVm) -> Vec<Span<'static>> {
    let window = vm
        .overview
        .as_ref()
        .map(|o| o.window.clone())
        .unwrap_or_default();
    let mut title = match &vm.content {
        ContentVm::Session { id, .. } => {
            let mut spans = Vec::new();
            match vm.sessions.rows.iter().find(|r| &r.id == id) {
                Some(s) => {
                    let (state, style) = super::sessions::state_cell(s);
                    let provider = if s.provider == "codex" {
                        "codex"
                    } else {
                        "claude"
                    };
                    spans.push(Span::styled(
                        format!(" {} ", s.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::raw(format!("· {provider} · ")));
                    spans.push(Span::styled(state, style));
                    if let Some(c) = &s.ctx {
                        spans.push(Span::raw(" · ctx "));
                        spans.push(Span::styled(format!("{}%", c.pct), ctx_style(c.pct)));
                        spans.push(Span::raw(format!(
                            " of {} [{}]",
                            tokens(c.window_tokens),
                            c.provenance
                        )));
                    }
                    spans.push(Span::raw(" "));
                }
                None => spans.push(Span::raw(" Session ")),
            }
            spans
        }
        ContentVm::Fold { parent, folded } => {
            let under = vm
                .nav
                .rows
                .iter()
                .find(|r| &r.key == parent)
                .map(|r| format!(" · under {}", r.label))
                .unwrap_or_default();
            vec![Span::raw(format!(" {}{under} ", folded_text(*folded)))]
        }
        _ => {
            let mut spans = vec![Span::raw(" Overview ")];
            if !vm.sessions.scope.is_empty() {
                spans.push(Span::raw(format!("· {} ", vm.sessions.scope)));
            }
            spans.push(Span::raw(format!("· activity: last {window} ")));
            spans
        }
    };
    title.extend(filter_title(vm));
    title
}

/// `⊘ 20 killed · ✓ 3 done · 1 earlier transcript`.
fn folded_text(f: FoldedVm) -> String {
    let mut parts = Vec::new();
    if f.killed > 0 {
        parts.push(format!("⊘ {} killed", f.killed));
    }
    if f.done > 0 {
        parts.push(format!("✓ {} done", f.done));
    }
    if f.transcripts > 0 {
        let noun = if f.transcripts == 1 {
            "transcript"
        } else {
            "transcripts"
        };
        parts.push(format!("{} earlier {noun}", f.transcripts));
    }
    parts.join(" · ")
}

/// Lines of the content inside the block: the fixed top (sessions summary,
/// column header) and the scrolling body; or a one-line message when empty.
fn content_lines(
    vm: &WatchScreenVm,
    width: usize,
    height: usize,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let Some(ov) = &vm.overview else {
        return (Vec::new(), Vec::new());
    };
    if ov.rows.is_empty() {
        let text = if !vm.status.filter.is_empty() {
            format!(
                " no agent matches \"{}\" — Esc clears the filter",
                vm.status.filter
            )
        } else if let ContentVm::Session {
            has_transcript: false,
            ..
        } = vm.content
        {
            " a live process without a transcript yet".to_string()
        } else {
            " no agents yet".to_string()
        };
        return (vec![Line::styled(text, dim())], Vec::new());
    }
    let cols = columns(width);
    // Sessions summary (top node): at most a third of the area (and 6 lines).
    // (The navigator lists the sessions too: skipped when the pane is narrow.)
    let mut top = if matches!(vm.content, ContentVm::Overview) && width >= 70 {
        super::sessions::summary_lines(&vm.sessions, width, (height.saturating_sub(1) / 3).min(6))
    } else {
        Vec::new()
    };
    // Old sessions have no activity in the window: say how to widen it.
    let quiet = ov.rows.iter().all(|r| r.lane.trim().is_empty());
    top.push(header_line(cols, &ov.cell, quiet));

    // Root header lines are interleaved with the rows, each session's folded
    // finished agents follow its rows.
    let mut lines: Vec<Line> = Vec::with_capacity(ov.rows.len() * 2);
    let mut folded = FoldedVm::default();
    for r in &ov.rows {
        if let Some(h) = &r.root {
            if folded.total() > 0 {
                lines.push(folded_line(folded));
            }
            folded = h.folded;
            lines.push(root_line(h, width));
        }
        lines.push(row_line(r, cols));
    }
    if folded.total() > 0 {
        lines.push(folded_line(folded));
    }
    if ov.older_hidden > 0 {
        let noun = if ov.older_hidden == 1 {
            "session"
        } else {
            "sessions"
        };
        lines.push(Line::styled(
            format!(" ▸ {} older {noun} — in the navigator", ov.older_hidden),
            dim(),
        ));
    }
    if matches!(vm.content, ContentVm::Fold { .. }) {
        lines.push(Line::styled(
            "   → lists them in the navigator · d shows them in place",
            dim(),
        ));
    }
    (top, lines)
}

/// Scrolling body lines and their visible height, for the content `area`.
pub fn metrics(area: Rect, vm: &WatchScreenVm) -> SectionMetrics {
    let block = pane_block(false, Vec::new());
    let inner = block.inner(area);
    let (top, body) = content_lines(vm, inner.width as usize, inner.height as usize);
    SectionMetrics {
        total: body.len(),
        height: (inner.height as usize).saturating_sub(top.len()),
    }
}

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let block = pane_block(vm.focus_pane == Pane::Content, title(vm));
    let inner = block.inner(area);
    if inner.height == 0 {
        f.render_widget(block, area);
        return;
    }
    let (top, lines) = content_lines(vm, inner.width as usize, inner.height as usize);
    let [head, body] =
        Layout::vertical([Constraint::Length(top.len() as u16), Constraint::Min(0)]).areas(inner);
    f.render_widget(Paragraph::new(top), head);
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

/// `✓ 18 finished · ⊘ 3 killed  (d to show)` under a session's rows.
fn folded_line(f: FoldedVm) -> Line<'static> {
    let mut spans = vec![Span::raw("    ")];
    if f.done > 0 {
        spans.push(Span::styled(
            format!("✓ {} finished", f.done),
            status_style(StatusVm::Done),
        ));
    }
    if f.killed > 0 {
        if f.done > 0 {
            spans.push(Span::styled(" · ", dim()));
        }
        spans.push(Span::styled(
            format!("⊘ {} killed", f.killed),
            status_style(StatusVm::Killed),
        ));
    }
    if f.transcripts > 0 {
        if f.done + f.killed > 0 {
            spans.push(Span::styled(" · ", dim()));
        }
        let noun = if f.transcripts == 1 {
            "transcript"
        } else {
            "transcripts"
        };
        spans.push(Span::styled(
            format!("{} earlier {noun}", f.transcripts),
            dim(),
        ));
    }
    spans.push(Span::styled("  (d to show)", dim()));
    Line::from(spans)
}

pub(super) fn pad(s: &str, width: usize) -> String {
    let s = clip(s, width);
    let w = text_width(&s);
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

fn header_line(cols: Columns, cell: &str, quiet: bool) -> Line<'static> {
    let mut text = format!(
        "{}{} {} {} ",
        " ".repeat(MARK_W),
        pad("agent", cols.name),
        pad("status", STATUS_W),
        pad("context", CTX_W)
    );
    if cols.lane > 0 {
        let label = if quiet {
            "no activity — + widens".to_string()
        } else {
            format!("activity {cell}/cell")
        };
        text.push_str(&pad(&label, cols.lane));
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
    // A prompt used as the name can be long: the facts after it matter too.
    spans.push(Span::styled(
        clip(&h.label, (width / 3).max(24)),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    spans.push(sep());
    spans.push(Span::styled(status_word(h.status), status_style(h.status)));
    if h.bg {
        spans.push(Span::styled(" (bg)", dim()));
    }
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
pub(super) fn clip_line(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
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
    if let Some(b) = r.badge {
        prefix.push_str(&format!("{b} "));
    }
    if r.root.is_some() && r.provider == "codex" {
        prefix.push_str("codex ");
    }
    let label_w = width.saturating_sub(text_width(&prefix));
    let label = pad(&r.label, label_w);
    vec![Span::styled(clip(&prefix, width), dim()), Span::raw(label)]
}

fn row_line(r: &OverviewRowVm, cols: Columns) -> Line<'static> {
    let mut spans = vec![Span::raw("  ")];
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
    Line::from(spans)
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
