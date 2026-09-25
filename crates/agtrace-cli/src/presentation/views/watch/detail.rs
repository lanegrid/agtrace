//! Agent detail screen: header, context history and totals, then four focusable
//! sections (Instructions, Now, Result, Timeline) with wrapped, scrollable text.
//!
//! ```text
//! ┏ ▶ Detail · audit-A ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
//! ┃ audit-A · teammate of s-lead (team audit) · general-purpose · ○ idle 3m     ┃
//! ┃ ctx ██░░░░░░░░ 12% of 200k [table]  ▁▁▂   in 24k / out 10 · 1 turns · 1 tools┃
//! ┃── [i] Instructions (1) ───────────────────────────────────────────────────── ┃
//! ┃[12:00 spawned by s-lead · NEW_TASK] review parser               … press i    ┃
//! ┃── ▶ [t] Timeline (12) ────────────────────────────────────────────────────── ┃
//! ┃12:00 ▸ Bash  mise run test                                                   ┃
//! ```
//!
//! The focused section gets the room it needs; the others are shown in full when
//! they fit, else partly (with spare rows) or collapsed to a one-line summary that
//! names the key focusing them.
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
/// Sections of at most this many lines are shown in full rather than collapsed.
const SHORT: usize = 3;
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
        let collapsed = !focused && rows == 1 && total > 1;
        // A summary line has no range to show.
        let (shown_rows, shown_total) = if collapsed { (0, 0) } else { (rows, total) };
        let heading = heading_line(
            *section,
            d,
            focused,
            start,
            shown_rows,
            shown_total,
            body.width as usize,
        );
        let mut lines = vec![heading];
        if collapsed {
            lines.push(summary_line(*section, d, body.width as usize));
        } else {
            lines.extend(p.contents[i].iter().skip(start).take(rows).cloned());
        }
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

/// Rows per section (heading included). The focused section comes first: its
/// heading and a line, then every other heading and a line (a one-line summary
/// when collapsed; short sections in full), then the focused section grows to its content, the other
/// sections are shown in full where they fit (shortest first), and spare rows go
/// to the timeline, the instructions, the now and the result sections, in turn.
fn allocate(total: usize, contents: &[Vec<Line<'static>>; 4], focus: DetailSection) -> [usize; 4] {
    let desired: [usize; 4] = std::array::from_fn(|i| 1 + contents[i].len());
    let mut h = [0usize; 4];
    let mut left = total;
    let f = focus.index();
    let give = |h: &mut [usize; 4], i: usize, n: usize, left: &mut usize| {
        let n = n.min(desired[i].saturating_sub(h[i])).min(*left);
        h[i] += n;
        *left -= n;
    };
    give(&mut h, f, 2, &mut left);
    for i in (0..4).filter(|i| *i != f) {
        give(&mut h, i, 1, &mut left);
    }
    // Short sections (up to SHORT lines) are cheaper to show than to summarize.
    for i in (0..4).filter(|i| *i != f) {
        let n = if desired[i] <= 1 + SHORT { SHORT } else { 1 };
        give(&mut h, i, n, &mut left);
    }
    give(&mut h, f, usize::MAX, &mut left);
    let mut others: Vec<usize> = (0..4).filter(|i| *i != f).collect();
    others.sort_by_key(|i| desired[*i]);
    for i in others {
        if desired[i] - h[i] <= left {
            give(&mut h, i, usize::MAX, &mut left);
        }
    }
    for s in [
        DetailSection::Timeline,
        DetailSection::Instructions,
        DetailSection::Now,
        DetailSection::Result,
    ] {
        give(&mut h, s.index(), usize::MAX, &mut left);
    }
    // Spare rows stay blank below the last section.
    h
}

/// One line standing for a collapsed section, ending with the key that opens it.
fn summary_line(s: DetailSection, d: &DetailVm, width: usize) -> Line<'static> {
    let text = one_line(&summary(s, d));
    let long = format!(" … press {}", s.key());
    // A clipped text already ends with `…`.
    let hint = if text_width(&text) + text_width(&long) > width {
        format!(" press {}", s.key())
    } else {
        long
    };
    let room = width.saturating_sub(text_width(&hint));
    let text = clip(&text, room);
    let pad = " ".repeat(room.saturating_sub(text_width(&text)));
    Line::from(vec![
        Span::styled(text, dim()),
        Span::raw(pad),
        Span::styled(hint, Style::default().fg(FOCUS_COLOR)),
    ])
}

/// Whitespace runs (newlines included) collapsed to one space.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Plain-text summary of a section: the latest instruction, the running tool (or
/// task list, or latest text), the result, the newest timeline row.
fn summary(s: DetailSection, d: &DetailVm) -> String {
    match s {
        DetailSection::Instructions => match d.instructions.last() {
            Some(i) => {
                let latest = if d.instructions.len() > 1 {
                    "latest "
                } else {
                    ""
                };
                let body = i
                    .text
                    .as_deref()
                    .or(i.note.as_deref())
                    .unwrap_or(if i.encrypted { "[encrypted]" } else { "" });
                format!("{latest}[{} {}] {body}", i.time, i.header)
            }
            None => "(no instructions seen in the log)".to_string(),
        },
        DetailSection::Now => {
            if let Some(ActivityVm::Tool {
                name,
                summary,
                elapsed_secs,
                ..
            }) = &d.now.tool
            {
                return format!("▸ {name} {summary} ({})", elapsed(*elapsed_secs));
            }
            let p = &d.now.plan;
            if !p.tasks.is_empty() {
                let done = p
                    .tasks
                    .iter()
                    .filter(|t| t.status == TaskStatusVm::Completed)
                    .count();
                let current = p
                    .tasks
                    .iter()
                    .find(|t| t.status == TaskStatusVm::InProgress)
                    .map(|t| format!(" · ▸ {}", t.text))
                    .unwrap_or_default();
                return format!("tasks {done}/{} done{current}", p.tasks.len());
            }
            match &d.now.said {
                Some(said) => {
                    let what = if d.now.said_is_reasoning {
                        "thinking"
                    } else {
                        "said"
                    };
                    format!("{what}: {said}")
                }
                None => status_word(d.now.status).to_string(),
            }
        }
        DetailSection::Result => match &d.result {
            ResultVm::Reported {
                time, tag, text, ..
            } => format!(
                "[{time} {tag}] {}",
                text.as_deref().unwrap_or("[encrypted]")
            ),
            ResultVm::LastMessage { time, text, .. } => format!("[{time} last message] {text}"),
            ResultVm::Pending { .. } => "(no result yet)".to_string(),
            ResultVm::Ended { status, .. } => format!("({} without a result)", ended_word(*status)),
        },
        DetailSection::Timeline => match d.timeline.last() {
            Some(r) => format!("{} {} {} {}", r.time, r.icon, r.label, r.text),
            None => "(no events yet)".to_string(),
        },
    }
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
    let left = format!("── {marker}[{}] {}{count} ", s.key(), s.title());
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

/// `text` wrapped under an item header, indented; runs of blank lines are
/// collapsed to one and leading / trailing ones dropped.
fn body(out: &mut Vec<Line<'static>>, text: &str, width: usize, style: Style) {
    let lines = wrap(text.trim(), width.saturating_sub(INDENT.len()));
    let mut blank = false;
    for l in lines {
        if l.is_empty() {
            if !blank {
                out.push(Line::raw(""));
            }
            blank = true;
            continue;
        }
        blank = false;
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
        // Now and Result fit in full; the timeline is left collapsed to a summary.
        assert_eq!(h[1], 3);
        assert_eq!(h[2], 2);
        assert_eq!(h[3], 2, "timeline collapsed to heading + summary");
        assert_eq!(h[0], 23);
        let h = allocate(30, &c, DetailSection::Timeline);
        assert_eq!(
            h,
            [2, 3, 2, 23],
            "short sections in full, instructions summarized"
        );
        // Tiny terminal: the focused heading and a line first.
        let h = allocate(3, &c, DetailSection::Timeline);
        assert_eq!(h, [1, 0, 0, 2]);
    }

    #[test]
    fn small_focus_leaves_the_rest_to_the_timeline() {
        // A running agent opens on "Now" (3 lines): the timeline gets the spare rows.
        let c = [lines(40), lines(3), lines(1), lines(300)];
        let h = allocate(30, &c, DetailSection::Now);
        assert_eq!(h[1], 4, "focused Now in full");
        assert_eq!(h[2], 2, "Result in full");
        assert_eq!(h[0], 2, "instructions collapsed");
        assert_eq!(h[3], 22, "timeline takes the rest");
    }
}
