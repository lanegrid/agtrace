use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct TurnHistoryComponent;

impl Component for TurnHistoryComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        if state.turns_usage.is_empty() || state.max_context.is_none() {
            let empty_list = List::new(vec![ListItem::new(Line::from(vec![Span::styled(
                "Waiting for turn data...",
                Style::default().fg(Color::DarkGray),
            )]))]);
            f.render_widget(empty_list, area);
            return;
        }

        let max_context = state.max_context.unwrap() as f64;

        let items: Vec<ListItem> = state
            .turns_usage
            .iter()
            .flat_map(|turn| {
                let mut lines = Vec::new();

                let title_line = Line::from(vec![
                    Span::styled(
                        format!("#{}  ", turn.turn_id),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("User: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("\"{}\"", truncate_text(&turn.title, 60))),
                ]);
                lines.push(ListItem::new(title_line));

                let bar_line = create_stacked_bar(
                    turn.prev_total,
                    turn.delta,
                    max_context as u32,
                    turn.is_heavy,
                    area.width as usize,
                );
                lines.push(ListItem::new(bar_line));

                let pct = (turn.delta as f64 / max_context) * 100.0;
                let pct_text = if turn.is_heavy {
                    format!("+{:.1}% (HEAVY!)", pct)
                } else {
                    format!("+{:.1}%", pct)
                };

                let stats_line = Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        pct_text,
                        Style::default().fg(if turn.is_heavy {
                            Color::Red
                        } else {
                            Color::White
                        }),
                    ),
                ]);
                lines.push(ListItem::new(stats_line));

                lines.push(ListItem::new(Line::from("")));

                lines
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, area);
    }
}

fn create_stacked_bar(
    prev_total: u32,
    delta: u32,
    max_context: u32,
    is_heavy: bool,
    bar_width: usize,
) -> Line<'static> {
    let bar_width = bar_width.min(80);

    let prev_ratio = prev_total as f64 / max_context as f64;
    let delta_ratio = delta as f64 / max_context as f64;
    let total_ratio = (prev_total + delta) as f64 / max_context as f64;

    let prev_chars = (prev_ratio * bar_width as f64) as usize;
    let delta_chars =
        (delta_ratio * bar_width as f64).max(if delta > 0 { 1.0 } else { 0.0 }) as usize;
    let remaining_chars = bar_width.saturating_sub(prev_chars + delta_chars);

    let history_bar = "█".repeat(prev_chars);
    let delta_bar = "▓".repeat(delta_chars);
    let void_bar = "░".repeat(remaining_chars);

    let delta_color = if is_heavy { Color::Red } else { Color::Cyan };

    let mut spans = vec![Span::raw("      [")];

    if !history_bar.is_empty() {
        spans.push(Span::styled(
            history_bar,
            Style::default().fg(Color::DarkGray),
        ));
    }

    if !delta_bar.is_empty() {
        spans.push(Span::styled(delta_bar, Style::default().fg(delta_color)));
    }

    if !void_bar.is_empty() {
        spans.push(Span::styled(void_bar, Style::default().fg(Color::Black)));
    }

    spans.push(Span::raw(format!("] {:.0}%", total_ratio * 100.0)));

    Line::from(spans)
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
