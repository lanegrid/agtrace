//! Bottom-right pane: inter-agent messages, spawns and lifecycle changes scoped
//! to the selection (all, a session, an agent, a folded group).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use super::style::{col_width, dim, pane_block};
use crate::presentation::view_models::watch::{FeedRowKind, FeedRowVm, Pane, WatchScreenVm};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let mut title = format!(" Messages · {} ", super::style::clip(&vm.feed_scope, 30));
    if !vm.feed_scroll.is_follow() {
        title.push_str("· scrolled ");
    }
    let block = pane_block(vm.focus_pane == Pane::Feed, vec![Span::raw(title)]);
    if vm.feed.is_empty() {
        let p = Paragraph::new(Line::styled(" no messages here yet", dim())).block(block);
        f.render_widget(p, area);
        return;
    }
    let height = block.inner(area).height as usize;
    let start = vm.feed_scroll.start(vm.feed.len(), height);
    let visible: Vec<&FeedRowVm> = vm.feed.iter().skip(start).take(height).collect();
    let routes: Vec<String> = visible.iter().map(|r| route(r)).collect();
    let route_w = col_width(routes.iter().map(String::as_str), 30);
    let tag_w = col_width(visible.iter().map(|r| r.tag.as_str()), 14);
    let rows: Vec<Row> = visible
        .iter()
        .zip(routes)
        .map(|(r, route)| row(r, route))
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(route_w),
            Constraint::Length(tag_w),
            Constraint::Min(0),
        ],
    )
    .column_spacing(1)
    .block(block);
    f.render_widget(table, area);
}

fn route(r: &FeedRowVm) -> String {
    if r.to.is_empty() {
        r.from.clone()
    } else {
        format!("{} → {}", r.from, r.to.join(", "))
    }
}

fn row(r: &FeedRowVm, route: String) -> Row<'static> {
    let tag_style = match r.kind {
        FeedRowKind::Message => Style::default().fg(Color::Magenta),
        FeedRowKind::Spawn => Style::default().fg(Color::Blue),
        FeedRowKind::Lifecycle => dim(),
    };
    let text = match (&r.text, r.encrypted) {
        (_, true) => Span::styled("[encrypted]", dim()),
        (Some(t), false) if r.kind == FeedRowKind::Message => Span::raw(format!("\"{t}\"")),
        (Some(t), false) => Span::raw(t.clone()),
        (None, false) => Span::raw(""),
    };
    Row::new(vec![
        Cell::from(Span::styled(r.time.clone(), dim())),
        Cell::from(Span::raw(route)),
        Cell::from(Span::styled(r.tag.clone(), tag_style)),
        Cell::from(text),
    ])
}
