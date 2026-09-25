//! Left pane: the navigator (scope, sessions, agent trees, folded groups).
//!
//! ```text
//! ┏ ▶ Navigator ━━━━━━━━━━━━━┓
//! ┃ ◆ yohaku-studio  3 live  ┃
//! ┃ ▸ ● PR 1693 の継続       ┃
//! ┃▶▾ ○ Projects制作のボト… ┃
//! ┃   ├ ✓ S explore call s… ┃
//! ┃   └ ▸ ⊘ 20 killed (d)   ┃
//! ┃ ▸ ✓ 1 older session      ┃
//! ```
//!
//! `▶` marks the selection, `▸` / `▾` a collapsed / expanded node.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::{
    FOCUS_COLOR, clip, ctx_style, dim, filter_title, more_marks, pane_block, status_glyph,
    status_style, text_width,
};
use crate::presentation::view_models::watch::{NavKind, NavRowVm, Pane, WatchScreenVm};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let focused = vm.focus_pane == Pane::Navigator;
    let mut title = vec![Span::raw(" Navigator ")];
    title.extend(filter_title(vm));
    let block = pane_block(focused, title);
    let inner = block.inner(area);
    let height = inner.height as usize;
    let rows = &vm.nav.rows;
    // Keep the selection visible: scroll only when it would fall off the bottom.
    let sel = vm.selected_index().unwrap_or(0);
    let start = if height == 0 || sel < height {
        0
    } else {
        sel + 1 - height
    };
    let width = inner.width as usize;
    let mut lines: Vec<Line> = rows
        .iter()
        .skip(start)
        .take(height)
        .map(|r| row(r, width, focused))
        .collect();
    if rows.len() == 1 && height > 1 {
        let text = if vm.status.filter.is_empty() {
            " no sessions yet"
        } else {
            " no match"
        };
        lines.push(Line::styled(text, dim()));
    }
    let block = match more_marks(start, lines.len().min(rows.len()), rows.len()) {
        Some(marks) => block.title_bottom(marks),
        None => block,
    };
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}

/// Tree prefix of a row below the sessions: guides and the connector.
fn prefix(r: &NavRowVm) -> String {
    let mut out = String::new();
    if r.depth >= 2 {
        out.push(' ');
        for g in &r.guides {
            out.push_str(if *g { "│ " } else { "  " });
        }
        out.push_str(if r.is_last_sibling { "└ " } else { "├ " });
    }
    out
}

fn row(r: &NavRowVm, width: usize, focused: bool) -> Line<'static> {
    let mut spans = vec![Span::styled(
        if r.selected { "▶" } else { " " },
        Style::default().fg(FOCUS_COLOR),
    )];
    let label_style = if r.selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    // Filter matches stand out among the ancestors kept for context.
    let label_style = if r.matched {
        label_style.fg(FOCUS_COLOR)
    } else {
        label_style
    };
    match r.kind {
        NavKind::Top => {
            spans.push(Span::styled("◆ ", Style::default().fg(FOCUS_COLOR)));
            spans.push(Span::styled(
                r.label.clone(),
                label_style.add_modifier(Modifier::BOLD),
            ));
            // The live count only when it fits whole (the name matters more).
            let live = format!("  {} live", r.live);
            if r.live > 0 && 3 + text_width(&r.label) + text_width(&live) <= width {
                spans.push(Span::styled(live, dim()));
            }
        }
        _ => {
            spans.push(Span::styled(prefix(r), dim()));
            let expand = match (r.expandable, r.expanded) {
                (true, true) => "▾ ",
                (true, false) => "▸ ",
                // Top-level sessions keep their glyphs aligned with the expandable ones.
                (false, _) if r.kind == NavKind::Session && r.depth == 1 => "  ",
                (false, _) => "",
            };
            spans.push(Span::styled(expand, dim()));
            match r.kind {
                NavKind::Fold => {
                    spans.push(Span::styled(r.label.clone(), status_style(r.status)));
                }
                NavKind::Older => {
                    spans.push(Span::styled("✓ ", dim()));
                    spans.push(Span::styled(r.label.clone(), dim()));
                }
                _ => {
                    spans.push(Span::styled(
                        format!("{} ", status_glyph(r.status)),
                        status_style(r.status),
                    ));
                    if let Some(b) = r.badge {
                        spans.push(Span::styled(format!("{b} "), dim()));
                    }
                    if r.kind == NavKind::Session && r.provider == "codex" {
                        spans.push(Span::styled("codex ", dim()));
                    }
                    spans.push(Span::styled(r.label.clone(), label_style));
                }
            }
        }
    }
    // Context % right-aligned (the label is clipped first) when the pane has room.
    let pct = r
        .ctx_pct
        .filter(|_| width >= 26)
        .map(|p| (format!(" {p:>3}%"), p));
    let room = width.saturating_sub(pct.as_ref().map_or(0, |(s, _)| text_width(s)));
    let mut line = fit(spans, room);
    if let Some((pct, p)) = pct {
        let used: usize = line.spans.iter().map(|s| text_width(&s.content)).sum();
        line.spans
            .push(Span::raw(" ".repeat(room.saturating_sub(used))));
        line.spans.push(Span::styled(pct, ctx_style(p)));
    }
    // The selection stays visible while another pane has focus.
    match (r.selected, focused) {
        (true, true) => line.style(Style::default().add_modifier(Modifier::REVERSED)),
        (true, false) => line.style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        ),
        (false, _) => line,
    }
}

/// Spans cut to `width` columns (the last one clipped with `…`).
fn fit(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let mut out = Vec::new();
    let mut w = 0;
    for s in spans {
        let sw = text_width(&s.content);
        if w + sw > width {
            let rest = width.saturating_sub(w);
            if rest > 0 {
                out.push(Span::styled(clip(&s.content, rest), s.style));
            }
            break;
        }
        w += sw;
        out.push(s);
    }
    Line::from(out)
}
