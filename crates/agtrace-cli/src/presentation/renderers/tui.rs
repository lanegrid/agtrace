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
use std::io::{self, Stdout};
use std::path::Path;
use std::sync::Mutex;

struct TuiWatchViewInner {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    events_buffer: VecDeque<String>,
    footer_lines: Vec<String>,
    session_start_time: Option<chrono::DateTime<chrono::Utc>>,
    turn_count: usize,
    project_root: Option<std::path::PathBuf>,
}

pub struct TuiWatchView {
    inner: Mutex<TuiWatchViewInner>,
}

impl TuiWatchView {
    pub fn new() -> Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Set up Ctrl+C handler to restore terminal
        ctrlc::set_handler(move || {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            std::process::exit(0);
        })?;

        Ok(Self {
            inner: Mutex::new(TuiWatchViewInner {
                terminal,
                events_buffer: VecDeque::new(),
                footer_lines: Vec::new(),
                session_start_time: None,
                turn_count: 0,
                project_root: None,
            }),
        })
    }

    fn render(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();

        // Clone data for rendering to avoid borrow checker issues
        let events = inner.events_buffer.clone();
        let footer = inner.footer_lines.clone();

        // Draw using Ratatui
        inner.terminal.draw(|f| {
            ui(f, &events, &footer);
        })?;

        Ok(())
    }
}

/// Render UI using Ratatui widgets
fn ui(f: &mut Frame, events: &VecDeque<String>, footer: &[String]) {
    // Split screen into main area and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),           // Main content area
            Constraint::Length(footer.len().max(1) as u16), // Footer
        ])
        .split(f.area());

    // Render events as a List
    let items: Vec<ListItem> = events
        .iter()
        .map(|line| ListItem::new(Line::from(line.as_str())))
        .collect();

    let events_list = List::new(items)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(events_list, chunks[0]);

    // Render footer
    let footer_text: Vec<Line> = footer
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();

    let footer_widget = Paragraph::new(Text::from(footer_text))
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(footer_widget, chunks[1]);
}

impl Drop for TuiWatchView {
    fn drop(&mut self) {
        // Restore terminal state when view is dropped
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

impl WatchView for TuiWatchView {
    fn render_watch_start(&self, start: &WatchStart) -> Result<()> {
        use WatchStart as WS;
        let mut inner = self.inner.lock().unwrap();
        let message = match start {
            WS::Provider { name, log_root } => {
                format!("👀 Watching {} ({})", log_root.display(), name)
            }
            WS::Session { id, log_root } => {
                format!("👀 Watching session {} in {}", id, log_root.display())
            }
        };
        inner.events_buffer.push_back(message);
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_attached(&self, display_name: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .events_buffer
            .push_back(format!("✨ Attached to active session: {}", display_name));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_initial_summary(&self, _summary: &WatchSummary) -> Result<()> {
        // Initial summary already shown by render_watch_start
        Ok(())
    }

    fn on_watch_rotated(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let old_name = old_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.display().to_string());
        let new_name = new_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());
        inner
            .events_buffer
            .push_back(format!("✨ Session rotated: {} → {}", old_name, new_name));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_waiting(&self, message: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .events_buffer
            .push_back(format!("⏳ Waiting: {}", message));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_error(&self, message: &str, _fatal: bool) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .events_buffer
            .push_back(format!("❌ Error: {}", message));
        drop(inner);
        self.render()?;
        Ok(())
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
        let mut inner = self.inner.lock().unwrap();

        // Update tracking state
        if inner.session_start_time.is_none() {
            inner.session_start_time = Some(state.start_time);
        }
        inner.turn_count = state.turn_count;
        inner.project_root = state.project_root.as_ref().map(|s| s.into());

        // Format and buffer new events
        for event in new_events {
            let opts = DisplayOptions {
                enable_color: true,
                relative_time: true,
                truncate_text: None,
            };

            let event_view = EventView {
                event,
                options: &opts,
                session_start: inner.session_start_time,
                turn_context: inner.turn_count,
            };

            let formatted = format!("{}", event_view);
            if !formatted.is_empty() {
                inner.events_buffer.push_back(formatted);

                // Keep buffer size manageable (last 1000 events)
                if inner.events_buffer.len() > 1000 {
                    inner.events_buffer.pop_front();
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
                inner.footer_lines = footer_output.lines().map(|s| s.to_string()).collect();
            }
        }

        // Drop the lock before rendering
        drop(inner);

        self.render()?;
        Ok(())
    }
}
