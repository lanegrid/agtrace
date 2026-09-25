//! `?` help overlay (design §6.2 keybindings).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub const KEYS: &[(&str, &str)] = &[
    ("1 / 2", "overview / agents screen"),
    ("Enter → l", "open the selected agent's detail"),
    ("Esc ← h", "back: close help / detail / to tree / reset"),
    ("j/k ↓/↑", "move selection, or scroll pane / section"),
    ("Tab/S-Tab", "cycle panes (agents) / sections (detail)"),
    ("+ - ] [", "overview activity window: 15m 60m 4h all"),
    ("space", "fold / unfold selected node"),
    ("PgUp/PgDn", "scroll focused pane / section"),
    ("C-u/C-d", "half-page scroll"),
    ("G / g", "jump to end (follow) / top"),
    ("f", "messages: all ↔ selected agent"),
    ("d", "hide / show done & killed"),
    ("a", "auto-select most active agent"),
    ("r", "rescan now"),
    ("?", "toggle this help"),
    ("q / C-c", "quit"),
];

pub fn render(f: &mut Frame, area: Rect) {
    let w = 58.min(area.width);
    let h = (KEYS.len() as u16 + 2).min(area.height);
    let popup = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    let lines: Vec<Line> = KEYS
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(
                    format!(" {k:<12}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(*d),
            ])
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Keys ");
    f.render_widget(Clear, popup);
    f.render_widget(Paragraph::new(lines).block(block), popup);
}
