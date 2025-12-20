use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;
use crate::presentation::view_models::EventPayloadViewModel;

pub(crate) struct TimelineComponent;

impl Component for TimelineComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let mut items: Vec<ListItem> = Vec::new();

        for msg in &state.system_messages {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                msg.clone(),
                Style::default().fg(Color::Gray),
            )])));
        }

        for event in &state.events_buffer {
            let time_str = if let Some(start) = state.session_start_time {
                let duration = event.timestamp.signed_duration_since(start);
                let seconds = duration.num_seconds();
                if seconds < 60 {
                    format!("[+{:02}s  ]", seconds)
                } else {
                    let minutes = seconds / 60;
                    let secs = seconds % 60;
                    format!("[+{}m {:02}s]", minutes, secs)
                }
            } else {
                let ts = event
                    .timestamp
                    .with_timezone(&chrono::Local)
                    .format("%H:%M:%S");
                format!("[{}]", ts)
            };

            let line = match &event.payload {
                EventPayloadViewModel::User { text } => {
                    if text.trim().is_empty() {
                        Line::from("")
                    } else {
                        let txt = truncate_text(text, 100);
                        Line::from(vec![
                            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled("👤 User:", Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw(format!(" [T{}] \"{}\"", state.turn_count + 1, txt)),
                        ])
                    }
                }
                EventPayloadViewModel::Reasoning { text } => {
                    if text.trim().is_empty() {
                        Line::from("")
                    } else {
                        let txt = truncate_text(text, 50);
                        Line::from(vec![
                            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled("🧠 Thnk:", Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled(txt, Style::default().fg(Color::DarkGray)),
                        ])
                    }
                }
                EventPayloadViewModel::ToolCall { name, .. } => {
                    let (icon, color) = categorize_tool(name);
                    Line::from(vec![
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::raw(icon),
                        Span::raw(" "),
                        Span::styled(name.clone(), Style::default().fg(color)),
                    ])
                }
                EventPayloadViewModel::ToolResult { output, is_error } => {
                    if *is_error {
                        let output_text = truncate_text(output, 100);
                        Line::from(vec![
                            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled("  ↳ Error:", Style::default().fg(Color::Red)),
                            Span::raw(" "),
                            Span::styled(output_text, Style::default().fg(Color::Red)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled("  ↳ OK", Style::default().fg(Color::Green)),
                        ])
                    }
                }
                EventPayloadViewModel::Message { text } => {
                    let txt = truncate_text(text, 100);
                    Line::from(vec![
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled("💬 Msg:", Style::default().fg(Color::Cyan)),
                        Span::raw(" "),
                        Span::raw(txt),
                    ])
                }
                EventPayloadViewModel::TokenUsage { .. } => Line::from(""),
                EventPayloadViewModel::Notification { text, level } => {
                    let color = match level.as_deref() {
                        Some("error") => Color::Red,
                        Some("warning") => Color::Yellow,
                        _ => Color::Blue,
                    };
                    Line::from(vec![
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(format!("ℹ️  {}", text), Style::default().fg(color)),
                    ])
                }
            };

            items.push(ListItem::new(line));
        }

        let events_list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(events_list, area, &mut state.list_state);
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

fn categorize_tool(name: &str) -> (&'static str, Color) {
    match name {
        "Read" | "read_file" => ("📖", Color::Blue),
        "Write" | "write_file" => ("✏️", Color::Green),
        "Edit" | "edit_file" => ("✂️", Color::Yellow),
        "Bash" | "bash" | "shell" => ("🔧", Color::Magenta),
        "Grep" | "grep" | "search" => ("🔍", Color::Cyan),
        "WebFetch" | "web_fetch" => ("🌐", Color::Blue),
        _ => ("🛠️", Color::White),
    }
}
