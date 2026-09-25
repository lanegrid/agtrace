//! Agent detail screen: header, context history and totals, then four focusable
//! sections (Instructions, Now, Result, Timeline) with wrapped, scrollable text.
//!
//! ```text
//! ┏ ▶ Detail · audit-A ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃ audit-A · teammate of s-lead (team audit) · general-purpose · ○ idle 3m     ┃
//! ┃ ctx ██░░░░░░░░ 12% of 200k [table]  ▁▁▂   in 24k / out 10 · 1 turns · 1 tools┃
//! ┃── ▶ Instructions (1) ─────────────────────────────────────────────────────── ┃
//! ┃[12:00 spawned by s-lead · NEW_TASK]                                          ┃
//! ┃  review parser                                                               ┃
//! ```
//!
//! [`plan`] (wrapping + height allocation) is shared by [`render`] and [`metrics`],
//! which the handler stores so scroll keys can clamp against the last frame.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::style::{
    FOCUS_COLOR, clip, ctx_bar, ctx_style, dim, elapsed, pane_block, short, status_glyph,
    status_style, status_word, text_width, wrap,
};
use crate::presentation::presenters::watch::tokens;
use crate::presentation::view_models::watch::{
    ActivityVm, DetailSection, DetailVm, PlanVm, ResultVm, RowKind, SectionMetrics, StatusVm,
    TaskStatusVm, TimelineRowVm, WatchScreenVm,
};

/// Header rows above the sections (identity line, context / totals line).
const HEADER_ROWS: u16 = 2;
/// Indent of wrapped body text under an item header.
const INDENT: &str = "  ";

/// Wrapped section contents and the rows allocated to each (heading included).
struct Plan {
    contents: [Vec<Line<'static>>; 4],
    heights: [usize; 4],
}

fn block(vm: &WatchScreenVm) -> Block<'static> {
    let title = match &vm.detail {
        Some(d) => format!(" Detail · {} ", d.title),
        None => " Detail ".to_string(),
    };
    pane_block(true, vec![Span::raw(title)])
}

/// Section area inside the block (below the header rows).
fn sections_area(area: Rect, vm: &WatchScreenVm) -> Rect {
    let inner = block(vm).inner(area);
    let [_, body] =
        Layout::vertical([Constraint::Length(HEADER_ROWS), Constraint::Min(0)]).areas(inner);
    body
}

/// Wrapped totals and visible body heights of the sections for `area` (the screen
/// area of the detail block, i.e. without the status bar).
pub fn metrics(area: Rect, vm: &WatchScreenVm) -> [SectionMetrics; 4] {
    let Some(d) = &vm.detail else {
        return Default::default();
    };
    let body = sections_area(area, vm);
    let p = plan(body, d);
    std::array::from_fn(|i| SectionMetrics {
        total: p.contents[i].len(),
        height: p.heights[i].saturating_sub(1),
    })
}

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let block = block(vm);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let Some(d) = &vm.detail else {
        f.render_widget(
            Paragraph::new(Line::styled(" agent not found — Esc to go back", dim())),
            inner,
        );
        return;
    };
    let [head, body] =
        Layout::vertical([Constraint::Length(HEADER_ROWS), Constraint::Min(0)]).areas(inner);
    f.render_widget(
        Paragraph::new(vec![
            identity_line(d, head.width as usize),
            context_line(d, head.width as usize),
        ]),
        head,
    );
    let p = plan(body, d);
    let mut y = body.y;
    for (i, section) in DetailSection::ALL.iter().enumerate() {
        let h = p.heights[i];
        if h == 0 {
            continue;
        }
        let rows = h - 1;
        let total = p.contents[i].len();
        let start = d.scroll[i].start(total, rows);
        let focused = d.section == *section;
        let heading = heading_line(
            *section,
            d,
            focused,
            start,
            rows,
            total,
            body.width as usize,
        );
        let mut lines = vec![heading];
        lines.extend(p.contents[i].iter().skip(start).take(rows).cloned());
        let r = Rect::new(body.x, y, body.width, h as u16);
        f.render_widget(Paragraph::new(lines), r);
        y += h as u16;
    }
}

fn plan(body: Rect, d: &DetailVm) -> Plan {
    let w = body.width as usize;
    let contents = [
        instructions(d, w),
        now(d, w),
        result(d, w),
        timeline(&d.timeline, w),
    ];
    Plan {
        heights: allocate(body.height as usize, &contents, d.section),
        contents,
    }
}

/// Rows per section (heading included): every section first gets its heading and
/// a few lines, then the focused section grows, then the timeline, the
/// instructions, the result and the now section, each up to its content.
fn allocate(total: usize, contents: &[Vec<Line<'static>>; 4], focus: DetailSection) -> [usize; 4] {
    let desired: [usize; 4] = std::array::from_fn(|i| 1 + contents[i].len());
    let mut h = [0usize; 4];
    let mut left = total;
    for (i, d) in desired.iter().enumerate() {
        let min = (1 + (d - 1).min(3)).min(*d);
        let take = min.min(left);
        h[i] = take;
        left -= take;
    }
    let order = [
        focus.index(),
        DetailSection::Timeline.index(),
        DetailSection::Instructions.index(),
        DetailSection::Result.index(),
        DetailSection::Now.index(),
    ];
    for i in order {
        let grow = desired[i].saturating_sub(h[i]).min(left);
        h[i] += grow;
        left -= grow;
    }
    // Spare rows stay blank below the last section.
    h
}

fn heading_line(
    s: DetailSection,
    d: &DetailVm,
    focused: bool,
    start: usize,
    rows: usize,
    total: usize,
    width: usize,
) -> Line<'static> {
    let count = match s {
        DetailSection::Instructions => format!(" ({})", d.instructions.len()),
        DetailSection::Timeline => format!(" ({})", d.timeline.len()),
        _ => String::new(),
    };
    let marker = if focused { "▶ " } else { "" };
    let left = format!("── {marker}{}{count} ", s.title());
    let pos = if total > rows && rows > 0 {
        format!(
            " {}-{}/{total}{} ",
            start + 1,
            (start + rows).min(total),
            if start + rows < total { " ↓" } else { "" }
        )
    } else {
        String::new()
    };
    let fill = width.saturating_sub(text_width(&left) + text_width(&pos));
    let style = if focused {
        Style::default()
            .fg(FOCUS_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        dim()
    };
    Line::from(vec![
        Span::styled(left, style),
        Span::styled("─".repeat(fill), style),
        Span::styled(pos, style),
    ])
}

fn identity_line(d: &DetailVm, width: usize) -> Line<'static> {
    let sep = || Span::styled(" · ", dim());
    let mut spans = vec![Span::styled(
        d.title.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    spans.push(sep());
    spans.push(Span::raw(d.relation.clone()));
    if let Some(t) = &d.agent_type {
        spans.push(sep());
        spans.push(Span::styled(t.clone(), dim()));
    }
    if let Some(m) = &d.model {
        spans.push(sep());
        spans.push(Span::styled(m.clone(), dim()));
    }
    if let Some(e) = &d.effort {
        spans.push(sep());
        spans.push(Span::styled(format!("effort {e}"), dim()));
    }
    spans.push(sep());
    let mut st = format!("{} {}", status_glyph(d.status), status_word(d.status));
    if let Some(s) = d.status_secs {
        st.push_str(&format!(" {}", short(s)));
    }
    spans.push(Span::styled(st, status_style(d.status)));
    fit(spans, width)
}

fn context_line(d: &DetailVm, width: usize) -> Line<'static> {
    let mut spans = vec![Span::styled("ctx ", dim())];
    match &d.ctx {
        Some(c) => {
            spans.push(Span::styled(ctx_bar(c.pct, 10), ctx_style(c.pct)));
            spans.push(Span::styled(format!(" {}%", c.pct), ctx_style(c.pct)));
            spans.push(Span::raw(format!(
                " of {} [{}]",
                tokens(c.window_tokens),
                c.provenance
            )));
        }
        None => spans.push(Span::styled("unknown", dim())),
    }
    if !d.spark.is_empty() {
        spans.push(Span::raw("  "));
        for c in d.spark.chars() {
            let style = if c == '⟲' {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            };
            spans.push(Span::styled(c.to_string(), style));
        }
    }
    let t = &d.totals;
    let mut totals = format!(
        "   in {} / out {}{}",
        tokens(t.input_tokens),
        if t.output_partial { "≥" } else { "" },
        tokens(t.output_tokens),
    );
    // Claude subagents log no turn ends: omit a meaningless "0 turns".
    if t.turns > 0 {
        totals.push_str(&format!(" · {} turns", t.turns));
    }
    totals.push_str(&format!(" · {} tools", t.tool_calls));
    spans.push(Span::styled(totals, dim()));
    fit(spans, width)
}

/// Spans cut to `width` columns.
fn fit(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
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

fn item_header(text: String) -> Line<'static> {
    Line::styled(
        text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

/// `text` wrapped under an item header, indented.
fn body(out: &mut Vec<Line<'static>>, text: &str, width: usize, style: Style) {
    for l in wrap(text, width.saturating_sub(INDENT.len())) {
        out.push(Line::styled(format!("{INDENT}{l}"), style));
    }
}

fn instructions(d: &DetailVm, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    if d.instructions.is_empty() {
        out.push(Line::styled(" (no instructions seen in the log)", dim()));
        return out;
    }
    for (n, i) in d.instructions.iter().enumerate() {
        if n > 0 {
            out.push(Line::raw(""));
        }
        out.push(item_header(format!("[{} {}]", i.time, i.header)));
        match (&i.text, &i.note) {
            (Some(t), _) => body(&mut out, t, width, Style::default()),
            // The note says what is known besides the (encrypted) body.
            (None, Some(note)) => body(&mut out, note, width, dim()),
            (None, None) if i.encrypted => body(&mut out, "[encrypted]", width, dim()),
            (None, None) => body(&mut out, "(no text)", width, dim()),
        }
    }
    out
}

fn now(d: &DetailVm, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    match &d.now.tool {
        Some(ActivityVm::Tool {
            name,
            summary,
            elapsed_secs,
            more,
        }) => {
            let mut spans = vec![
                Span::styled(format!("▸ {name} "), Style::default().fg(Color::Green)),
                Span::raw(summary.clone()),
                Span::styled(format!("  ({})", elapsed(*elapsed_secs)), dim()),
            ];
            if *more > 0 {
                spans.push(Span::styled(format!(" +{more} open"), dim()));
            }
            out.push(fit(spans, width));
        }
        _ => {
            let text = match d.now.status {
                StatusVm::Running => "working (no tool running)",
                StatusVm::Idle => "idle — waiting for input",
                StatusVm::Done => "done",
                StatusVm::Killed => "killed",
                StatusVm::Failed => "failed",
                StatusVm::Unknown => "no activity yet",
            };
            out.push(Line::styled(text.to_string(), status_style(d.now.status)));
        }
    }
    plan_block(&mut out, &d.now.plan, width);
    if let Some(said) = &d.now.said {
        let what = if d.now.said_is_reasoning {
            "thinking"
        } else {
            "last said"
        };
        let time = d.now.said_time.as_deref().unwrap_or("");
        out.push(Line::styled(format!("{what} ({time}):"), dim()));
        body(&mut out, said, width, Style::default());
    }
    out
}

/// The "Plan" sub-block of the Now section: goal, task list and plan text.
fn plan_block(out: &mut Vec<Line<'static>>, p: &PlanVm, width: usize) {
    if p.is_empty() {
        return;
    }
    if let Some((objective, status)) = &p.goal {
        let mut text = format!("Goal: {objective}");
        if let Some(s) = status {
            text.push_str(&format!(" ({s})"));
        }
        for l in wrap(&text, width) {
            out.push(Line::styled(l, Style::default().fg(Color::Cyan)));
        }
    }
    if !p.tasks.is_empty() {
        let done = p
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatusVm::Completed)
            .count();
        out.push(Line::styled(
            format!("tasks ({done}/{} done):", p.tasks.len()),
            dim(),
        ));
        for t in &p.tasks {
            let (glyph, style) = match t.status {
                TaskStatusVm::Pending => ("☐", Style::default()),
                TaskStatusVm::InProgress => ("▸", Style::default().fg(Color::Cyan)),
                TaskStatusVm::Completed => ("✓", dim()),
                TaskStatusVm::Other => ("?", dim()),
            };
            let mut spans = vec![
                Span::styled(format!("{INDENT}{glyph} "), style),
                Span::styled(t.text.clone(), style),
            ];
            if let Some(by) = &t.by {
                spans.push(Span::styled(format!("  [{by}]"), dim()));
            }
            out.push(fit(spans, width));
        }
    }
    if let Some(text) = &p.text {
        let time = p.text_time.as_deref().unwrap_or("");
        out.push(Line::styled(format!("plan ({time}):"), dim()));
        body(out, text, width, Style::default());
    }
}

fn result(d: &DetailVm, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    match &d.result {
        ResultVm::Pending { status } => {
            let text = match status {
                StatusVm::Idle => "(idle — no result reported yet)",
                _ => "(running — no result yet)",
            };
            out.push(Line::styled(text, dim()));
        }
        ResultVm::Reported {
            time,
            tag,
            text,
            encrypted,
        } => {
            out.push(item_header(format!("[{time} {tag}]")));
            match text {
                Some(t) => body(&mut out, t, width, Style::default()),
                None if *encrypted => body(&mut out, "[encrypted]", width, dim()),
                None => body(&mut out, "(no text)", width, dim()),
            }
        }
        ResultVm::LastMessage {
            time,
            text,
            status,
            reason,
        } => {
            let mut head = format!("[{time} last message · {}", ended_word(*status));
            if let Some(r) = reason {
                head.push_str(&format!(": {r}"));
            }
            head.push_str(", no result reported]");
            out.push(item_header(head));
            body(&mut out, text, width, Style::default());
        }
        ResultVm::Ended { status, reason } => {
            let mut text = format!("({} without a reported result", ended_word(*status));
            if let Some(r) = reason {
                text.push_str(&format!(": {r}"));
            }
            text.push(')');
            out.push(Line::styled(text, status_style(*status)));
        }
    }
    out
}

fn ended_word(s: StatusVm) -> &'static str {
    match s {
        StatusVm::Killed => "killed",
        StatusVm::Failed => "failed",
        _ => "done",
    }
}

fn kind_style(k: RowKind) -> Style {
    match k {
        RowKind::User | RowKind::Command => Style::default().fg(Color::Cyan),
        RowKind::Assistant => Style::default(),
        RowKind::Tool | RowKind::SubAction => Style::default().fg(Color::Green),
        RowKind::ToolError | RowKind::Interrupt => Style::default().fg(Color::Red),
        RowKind::MessageIn | RowKind::MessageOut => Style::default().fg(Color::Magenta),
        RowKind::Spawn | RowKind::Lifecycle => Style::default().fg(Color::Blue),
        RowKind::Compaction | RowKind::ModelChange => Style::default().fg(Color::Yellow),
        RowKind::TurnEnd | RowKind::Queued => dim(),
    }
}

fn timeline(rows: &[TimelineRowVm], width: usize) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return vec![Line::styled(" (no events yet)", dim())];
    }
    let label_w = rows
        .iter()
        .map(|r| text_width(&r.label))
        .max()
        .unwrap_or(0)
        .min(14);
    rows.iter()
        .map(|r| {
            let style = kind_style(r.kind);
            let label = clip(&r.label, label_w);
            let pad = " ".repeat(label_w.saturating_sub(text_width(&label)));
            fit(
                vec![
                    Span::styled(format!("{} ", r.time), dim()),
                    Span::styled(format!("{} ", r.icon), style),
                    Span::styled(format!("{label}{pad} "), style),
                    Span::raw(r.text.clone()),
                ],
                width,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(n: usize) -> Vec<Line<'static>> {
        (0..n).map(|_| Line::raw("x")).collect()
    }

    #[test]
    fn allocation_gives_the_focused_section_the_room() {
        let c = [lines(40), lines(2), lines(1), lines(30)];
        let h = allocate(30, &c, DetailSection::Instructions);
        assert_eq!(h.iter().sum::<usize>(), 30);
        // Now and Result are shown in full; the rest goes to the focus first.
        assert_eq!(h[1], 3);
        assert_eq!(h[2], 2);
        assert_eq!(h[3], 4, "timeline keeps its minimum");
        assert_eq!(h[0], 21);
        let h = allocate(30, &c, DetailSection::Timeline);
        assert_eq!(h[0], 4);
        assert_eq!(h[3], 21);
        // Tiny terminal: headings first, in order.
        let h = allocate(3, &c, DetailSection::Timeline);
        assert_eq!(h, [3, 0, 0, 0]);
    }
}
