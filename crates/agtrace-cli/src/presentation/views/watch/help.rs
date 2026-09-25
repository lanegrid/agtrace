//! `?` help overlay (design §6.2 keybindings).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub const KEYS: &[(&str, &str)] = &[
    ("↑↓ j k", "navigator: move (content follows); else scroll"),
    ("→ l", "expand; again: first child; on an agent: read it"),
    ("← h", "collapse, else parent; content/messages: back"),
    ("Enter", "read the selection (focus the content)"),
    ("Esc", "clear filter, else go to the top node"),
    ("i n r t", "instructions / now / result / timeline"),
    ("Tab/S-Tab", "focus navigator → content → messages"),
    ("PgUp/PgDn", "page in the focused pane"),
    ("C-u/C-d", "half page"),
    ("g / G", "top / end (messages: follow)"),
    ("/", "find agents by name (Esc clears)"),
    ("d", "show / fold finished & killed agents"),
    ("space", "expand / collapse the selected node"),
    ("+ - ] [", "activity window: 15m 60m 4h all"),
    ("s", "hide / show the navigator (below 80 cols)"),
    ("A", "follow the most recently active agent"),
    ("R", "rescan now"),
    ("?", "toggle this help"),
    ("q / C-c", "quit"),
];

pub fn render(f: &mut Frame, area: Rect) {
    let w = 64.min(area.width);
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
