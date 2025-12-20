use super::traits::WatchView;
use crate::presentation::formatters::token::TokenUsageView;
use crate::presentation::view_models::DisplayOptions;
use crate::presentation::view_models::{
    EventPayloadViewModel, EventViewModel, ReactionViewModel, StreamStateViewModel, WatchStart,
    WatchSummary,
};
use crate::presentation::views::EventView;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Events that the TUI can handle
#[derive(Debug, Clone)]
pub(crate) enum TuiEvent {
    /// User keyboard input (reserved for future scroll/filter features)
    #[allow(dead_code)]
    Input(KeyEvent),
    /// Periodic tick for updates (reserved for future features)
    #[allow(dead_code)]
    Tick,
    /// Watch service events forwarded from WatchView trait
    WatchStart(WatchStart),
    WatchAttached(String),
    WatchRotated(String, String), // old_name, new_name
    WatchWaiting(String),
    WatchError(String, bool), // message, fatal
    StreamUpdate(StreamStateViewModel, Vec<EventViewModel>),
}

/// The TUI view for watch command
/// This is the public API that implements WatchView trait
pub struct TuiWatchView {
    /// Channel to send events to the event loop
    tx: Sender<TuiEvent>,
}

impl TuiWatchView {
    /// Create a new TUI view
    /// Returns the view (for WatchView trait) and a receiver (for event loop)
    pub(crate) fn new() -> Result<(Self, Receiver<TuiEvent>)> {
        let (tx, rx) = mpsc::channel();
        Ok((Self { tx }, rx))
    }

    /// Get a clone of the sender (useful for passing to background threads)
    #[allow(dead_code)]
    pub(crate) fn sender(&self) -> Sender<TuiEvent> {
        self.tx.clone()
    }

    /// Run the TUI event loop
    /// This is the main entry point for the TUI, called from the handler
    pub(crate) fn run(rx: Receiver<TuiEvent>) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Set up Ctrl+C handler to restore terminal
        ctrlc::set_handler(move || {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            std::process::exit(0);
        })?;

        // Application state
        let mut events_buffer: VecDeque<String> = VecDeque::new();
        let mut footer_lines: Vec<String> = Vec::new();
        let mut session_start_time: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut _turn_count: usize = 0; // Will be used when implementing scroll features
        let mut should_quit = false;

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = std::time::Instant::now();

        // Event loop
        while !should_quit {
            // Handle terminal drawing
            terminal.draw(|f| {
                ui(f, &events_buffer, &footer_lines);
            })?;

            // Poll for events with timeout
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            // Check for keyboard input
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit = true;
                        }
                        _ => {}
                    }
                }
            }

            // Check for events from WatchService
            while let Ok(tui_event) = rx.try_recv() {
                match tui_event {
                    TuiEvent::WatchStart(start) => {
                        let message = match start {
                            WatchStart::Provider { name, log_root } => {
                                format!("👀 Watching {} ({})", log_root.display(), name)
                            }
                            WatchStart::Session { id, log_root } => {
                                format!("👀 Watching session {} in {}", id, log_root.display())
                            }
                        };
                        events_buffer.push_back(message);
                    }
                    TuiEvent::WatchAttached(display_name) => {
                        events_buffer
                            .push_back(format!("✨ Attached to active session: {}", display_name));
                    }
                    TuiEvent::WatchRotated(old_name, new_name) => {
                        events_buffer
                            .push_back(format!("✨ Session rotated: {} → {}", old_name, new_name));
                    }
                    TuiEvent::WatchWaiting(message) => {
                        events_buffer.push_back(format!("⏳ Waiting: {}", message));
                    }
                    TuiEvent::WatchError(message, fatal) => {
                        events_buffer.push_back(format!("❌ Error: {}", message));
                        if fatal {
                            should_quit = true;
                        }
                    }
                    TuiEvent::StreamUpdate(state, new_events) => {
                        // Update tracking state
                        if session_start_time.is_none() {
                            session_start_time = Some(state.start_time);
                        }
                        _turn_count = state.turn_count;

                        // Format and buffer new events
                        for event in new_events {
                            let opts = DisplayOptions {
                                enable_color: true,
                                relative_time: true,
                                truncate_text: None,
                            };

                            let event_view = EventView {
                                event: &event,
                                options: &opts,
                                session_start: session_start_time,
                                turn_context: _turn_count,
                            };

                            let formatted = format!("{}", event_view);
                            if !formatted.is_empty() {
                                events_buffer.push_back(formatted);

                                // Keep buffer size manageable
                                if events_buffer.len() > 1000 {
                                    events_buffer.pop_front();
                                }
                            }

                            // Update footer on TokenUsage events
                            if matches!(event.payload, EventPayloadViewModel::TokenUsage { .. }) {
                                let opts = DisplayOptions {
                                    enable_color: true,
                                    relative_time: false,
                                    truncate_text: None,
                                };

                                let token_view = TokenUsageView::from_usage_data(
                                    state.current_usage.fresh_input,
                                    state.current_usage.cache_creation,
                                    state.current_usage.cache_read,
                                    state.current_usage.output,
                                    state.current_reasoning_tokens,
                                    state.model.clone(),
                                    state.token_limit,
                                    state.compaction_buffer_pct,
                                    opts,
                                );
                                let footer_output = format!("{}", token_view);
                                footer_lines =
                                    footer_output.lines().map(|s| s.to_string()).collect();
                            }
                        }
                    }
                    TuiEvent::Input(_) | TuiEvent::Tick => {
                        // Handled separately
                    }
                }
            }

            // Send tick if needed
            if last_tick.elapsed() >= tick_rate {
                last_tick = std::time::Instant::now();
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }
}

/// Render UI using Ratatui widgets
fn ui(f: &mut Frame, events: &VecDeque<String>, footer: &[String]) {
    // Split screen into main area and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),                             // Main content area
            Constraint::Length(footer.len().max(1) as u16), // Footer
        ])
        .split(f.area());

    // Render events as a List
    let items: Vec<ListItem> = events
        .iter()
        .map(|line| ListItem::new(Line::from(line.as_str())))
        .collect();

    let events_list = List::new(items).block(Block::default().borders(Borders::NONE));

    f.render_widget(events_list, chunks[0]);

    // Render footer
    let footer_text: Vec<Line> = footer
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();

    let footer_widget = Paragraph::new(Text::from(footer_text)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(footer_widget, chunks[1]);
}

/// WatchView trait implementation - sends events to the event loop
impl WatchView for TuiWatchView {
    fn render_watch_start(&self, start: &WatchStart) -> Result<()> {
        self.tx
            .send(TuiEvent::WatchStart(start.clone()))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }

    fn on_watch_attached(&self, display_name: &str) -> Result<()> {
        self.tx
            .send(TuiEvent::WatchAttached(display_name.to_string()))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }

    fn on_watch_initial_summary(&self, _summary: &WatchSummary) -> Result<()> {
        // Initial summary already shown by render_watch_start
        Ok(())
    }

    fn on_watch_rotated(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let old_name = old_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.display().to_string());
        let new_name = new_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());

        self.tx
            .send(TuiEvent::WatchRotated(old_name, new_name))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }

    fn on_watch_waiting(&self, message: &str) -> Result<()> {
        self.tx
            .send(TuiEvent::WatchWaiting(message.to_string()))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }

    fn on_watch_error(&self, message: &str, fatal: bool) -> Result<()> {
        self.tx
            .send(TuiEvent::WatchError(message.to_string(), fatal))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }

    fn on_watch_orphaned(&self, _orphaned: usize, _total_events: usize) -> Result<()> {
        Ok(())
    }

    fn on_watch_token_warning(&self, _warning: &str) -> Result<()> {
        Ok(())
    }

    fn on_watch_reactor_error(&self, _reactor_name: &str, _error: &str) -> Result<()> {
        Ok(())
    }

    fn on_watch_reaction_error(&self, _error: &str) -> Result<()> {
        Ok(())
    }

    fn on_watch_reaction(&self, _reaction: &ReactionViewModel) -> Result<()> {
        Ok(())
    }

    fn render_stream_update(
        &self,
        state: &StreamStateViewModel,
        new_events: &[EventViewModel],
    ) -> Result<()> {
        self.tx
            .send(TuiEvent::StreamUpdate(state.clone(), new_events.to_vec()))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }
}
