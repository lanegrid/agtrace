//! Right pane: header, current activity and timeline of the selected agent.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use super::style::{col_width, ctx_style, dim, elapsed, pane_block, status_style};
use crate::presentation::presenters::agent_watch::tokens;
use crate::presentation::view_models::agent_watch::{
    ActivityVm, FocusVm, Pane, RowKind, TimelineRowVm, WatchScreenVm,
};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm, height: usize) {
    let focus = &vm.focus;
    let block = pane_block(vm.focus_pane == Pane::Timeline).title(title(focus));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let [act, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    f.render_widget(Paragraph::new(activity_line(focus)), act);

    let start = vm.timeline_scroll.start(focus.rows.len(), height);
    let visible: Vec<&TimelineRowVm> = focus.rows.iter().skip(start).take(height).collect();
    let label_w = col_width(visible.iter().map(|r| r.label.as_str()), 14);
    let rows: Vec<Row> = visible.iter().map(|r| row(r)).collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(label_w),
            Constraint::Min(0),
        ],
    )
    .column_spacing(1);
    f.render_widget(table, body);
}

fn title(focus: &FocusVm) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!(" {} ", focus.title),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if let Some(m) = &focus.model {
        spans.push(Span::styled(format!("· {m} "), dim()));
    }
    if let Some(c) = &focus.ctx {
        spans.push(Span::styled(format!("· {}%", c.pct), ctx_style(c.pct)));
        spans.push(Span::raw(format!(
            " of {} [{}] ",
            tokens(c.window_tokens),
            c.provenance
        )));
    }
    if !focus.follow {
        spans.push(Span::styled(
            "· scrolled ",
            Style::default().fg(Color::Yellow),
        ));
    }
    Line::from(spans)
}

fn activity_line(focus: &FocusVm) -> Line<'static> {
    let status = Span::styled("now   ", status_style(focus.status));
    match &focus.activity {
        Some(ActivityVm::Tool {
            name,
            summary,
            elapsed_secs,
            more,
        }) => {
            let mut spans = vec![
                status,
                Span::styled(format!("▸ {name} "), Style::default().fg(Color::Green)),
                Span::raw(summary.clone()),
                Span::styled(format!("  ({})", elapsed(*elapsed_secs)), dim()),
            ];
            if *more > 0 {
                spans.push(Span::styled(format!(" +{more} open"), dim()));
            }
            Line::from(spans)
        }
        Some(ActivityVm::TurnEnded { outcome, ago_secs }) => Line::from(vec![
            status,
            Span::styled(format!("turn {outcome} {} ago", elapsed(*ago_secs)), dim()),
        ]),
        None if focus.agent_id.is_none() => Line::styled("", dim()),
        None => Line::from(vec![status, Span::styled("no activity yet", dim())]),
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

fn row(r: &TimelineRowVm) -> Row<'static> {
    let style = kind_style(r.kind);
    Row::new(vec![
        Cell::from(Span::styled(r.time.clone(), dim())),
        Cell::from(Span::styled(r.icon, style)),
        Cell::from(Span::styled(r.label.clone(), style)),
        Cell::from(Span::raw(r.text.clone())),
    ])
}
