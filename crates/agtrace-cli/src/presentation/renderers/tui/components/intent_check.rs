use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;
use crate::presentation::view_models::EventPayloadViewModel;

pub(crate) struct IntentCheckComponent;

impl Component for IntentCheckComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        if state.intent_events.is_empty() {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                " INTENT CHECK ",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ));

        let mut lines = Vec::new();

        for (idx, event) in state.intent_events.iter().enumerate() {
            let time_str = if let Some(start) = state.session_start_time {
                let duration = event.timestamp.signed_duration_since(start);
                let seconds = duration.num_seconds();
                if seconds < 60 {
                    format!("[+{}s]", seconds)
                } else {
                    let minutes = seconds / 60;
                    let secs = seconds % 60;
                    format!("[+{}m{}s]", minutes, secs)
                }
            } else {
                String::new()
            };

            let line_spans = match &event.payload {
                EventPayloadViewModel::User { text } => {
                    let display_text = truncate_text(text, 80);
                    vec![
                        Span::styled("👤 User: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("\"{}\"", display_text)),
                    ]
                }
                EventPayloadViewModel::Reasoning { text } => {
                    let display_text = truncate_text(text, 70);
                    vec![
                        Span::styled(
                            format!("{} 🧠 ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(display_text, Style::default().fg(Color::White)),
                    ]
                }
                EventPayloadViewModel::ToolCall { name, .. } => {
                    let (icon, color) = categorize_tool(name);
                    vec![
                        Span::styled(
                            format!("{} ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(format!("{} ", icon)),
                        Span::styled(
                            name.clone(),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                    ]
                }
                EventPayloadViewModel::Message { text } => {
                    let display_text = truncate_text(text, 70);
                    vec![
                        Span::styled(
                            format!("{} ", time_str),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("💬 ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_text),
                    ]
                }
                _ => vec![],
            };

            if !line_spans.is_empty() {
                lines.push(Line::from(line_spans));
            }

            // Limit to 5 lines to keep panel compact
            if idx >= 4 {
                break;
            }
        }

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
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
        "Task" | "task" => ("🤖", Color::LightMagenta),
        "Glob" | "glob" => ("📁", Color::LightBlue),
        _ => ("🛠️", Color::White),
    }
}
