//! Left pane: live agent tree.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use super::style::{ctx_style, dim, pane_block, status_glyph, status_style, status_word};
use crate::presentation::view_models::agent_watch::{AgentRowVm, Pane, WatchScreenVm};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm, height: usize) {
    let focused = vm.focus_pane == Pane::Tree;
    let block = pane_block(focused).title(" Agents ");
    if vm.tree.is_empty() {
        let p = Paragraph::new(Line::styled(" no agents yet", dim())).block(block);
        f.render_widget(p, area);
        return;
    }
    // Keep the selection visible: scroll only when it would fall off the bottom.
    let sel = vm.selected_index().unwrap_or(0);
    let start = if height == 0 || sel < height {
        0
    } else {
        sel + 1 - height
    };
    let rows: Vec<Row> = vm
        .tree
        .iter()
        .skip(start)
        .take(height.max(1))
        .map(|r| row(r, focused))
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Min(4),
            Constraint::Length(6),
            Constraint::Length(4),
        ],
    )
    .column_spacing(1)
    .block(block);
    f.render_widget(table, area);
}

fn row(r: &AgentRowVm, focused: bool) -> Row<'static> {
    let mut spans = Vec::new();
    spans.push(Span::raw(if r.selected { "▶ " } else { "  " }));
    let mut prefix = String::new();
    for g in &r.guides {
        prefix.push_str(if *g { "│ " } else { "  " });
    }
    if r.depth > 0 {
        prefix.push_str(if r.is_last_sibling { "└ " } else { "├ " });
    }
    spans.push(Span::styled(prefix, dim()));
    if r.collapsed {
        spans.push(Span::styled("▸", dim()));
    }
    if let Some(b) = r.badge {
        spans.push(Span::styled(format!("{b} "), dim()));
    }
    if r.depth == 0 && r.provider == "codex" {
        spans.push(Span::styled("codex ", dim()));
    }
    let label_style = if r.selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    spans.push(Span::styled(r.label.clone(), label_style));
    if r.collapsed && r.hidden_descendants > 0 {
        spans.push(Span::styled(format!(" +{}", r.hidden_descendants), dim()));
    }
    let status = Line::from(vec![
        Span::styled(status_glyph(r.status), status_style(r.status)),
        Span::raw(" "),
        Span::styled(status_word(r.status), status_style(r.status)),
    ]);
    let pct = match r.ctx_pct {
        Some(p) => Line::styled(format!("{p}%"), ctx_style(p)).right_aligned(),
        None => Line::raw(""),
    };
    let row = Row::new(vec![
        Cell::from(Line::from(spans)),
        Cell::from(status),
        Cell::from(pct),
    ]);
    if r.selected && focused {
        row.style(Style::default().add_modifier(Modifier::REVERSED))
    } else {
        row
    }
}
