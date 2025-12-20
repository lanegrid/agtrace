use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct FooterComponent;

impl Component for FooterComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let footer_text: Vec<Line> = state
            .footer_lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect();

        let footer_widget = Paragraph::new(Text::from(footer_text)).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        f.render_widget(footer_widget, area);
    }
}
