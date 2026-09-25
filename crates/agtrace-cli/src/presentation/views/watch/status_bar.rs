//! One-line status bar: scope, agent counts, diagnostics, toggles, key hint.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::style::dim;
use crate::presentation::view_models::watch::{FeedFilter, WatchScreenVm};

pub fn render(f: &mut Frame, area: Rect, vm: &WatchScreenVm) {
    let s = &vm.status;
    let sep = || Span::styled(" · ", dim());
    let mut spans = Vec::new();
    if !s.scope.is_empty() {
        spans.push(Span::raw(format!(" scope: {}", s.scope)));
        spans.push(sep());
    } else {
        spans.push(Span::raw(" "));
    }
    spans.push(Span::raw(format!(
        "{} agents ({} running, {} idle)",
        s.agents, s.running, s.idle
    )));
    if s.hidden > 0 {
        spans.push(Span::styled(format!(" {} hidden", s.hidden), dim()));
    }
    if s.diagnostics > 0 {
        spans.push(sep());
        spans.push(Span::styled(
            format!("{} diag", s.diagnostics),
            Style::default().fg(Color::Yellow),
        ));
    }
    if s.errors > 0 {
        spans.push(sep());
        let msg = s.last_error.as_deref().unwrap_or_default();
        spans.push(Span::styled(
            format!("{} err: {msg}", s.errors),
            Style::default().fg(Color::Red),
        ));
    }
    let mut toggles = Vec::new();
    if s.feed_filter == FeedFilter::Selected {
        toggles.push("feed:selected");
    }
    if s.hide_done {
        toggles.push("hide done");
    }
    if s.auto_select {
        toggles.push("auto");
    }
    let collapsed = (s.collapsed > 0).then(|| format!("{} collapsed", s.collapsed));
    if let Some(c) = &collapsed {
        toggles.push(c);
    }
    let changed = !toggles.is_empty();
    if changed {
        spans.push(sep());
        spans.push(Span::styled(
            toggles.join(" "),
            Style::default().fg(Color::Cyan),
        ));
    }
    spans.push(sep());
    let hint = if changed {
        "Esc:reset ?:help q:quit"
    } else {
        "?:help q:quit"
    };
    spans.push(Span::styled(hint, dim()));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
