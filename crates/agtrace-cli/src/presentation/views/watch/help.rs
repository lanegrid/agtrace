//! `?` help overlay (design §6.2 keybindings).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub const KEYS: &[(&str, &str)] = &[
    ("0 / 1 / 2", "sessions / overview / agents screen"),
    ("Enter → l", "open the agent's detail (sessions: focus it)"),
    ("a", "all sessions (drop the session focus)"),
    ("Esc ← h", "back: help/detail/tree/filter/reset/focus"),
    ("i n r t", "detail: instructions / now / result / timeline"),
    ("Tab/S-Tab", "cycle panes (agents) / sections (detail)"),
    ("j/k ↓/↑", "move selection, or scroll pane / section"),
    ("J / K", "next / previous agent (also in the detail)"),
    ("PgUp/PgDn", "page scroll (overview: move selection)"),
    ("C-u/C-d", "half-page scroll"),
    ("G / g", "jump to end (follow) / top"),
    ("/", "find agents by name (↵ open, Esc clear)"),
    ("+ - ] [", "overview activity window: 15m 60m 4h all"),
    ("space", "fold / unfold node (sessions: older ones)"),
    ("f", "messages: all ↔ selected agent"),
    ("d", "show / fold finished & killed agents"),
    ("A", "auto-select most active agent"),
    ("R", "rescan now"),
    ("?", "toggle this help"),
    ("q / C-c", "quit"),
];

pub fn render(f: &mut Frame, area: Rect) {
    let w = 62.min(area.width);
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
