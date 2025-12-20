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
    cursor, execute, queue, terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

struct TuiWatchViewInner {
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
        // Enter alternate screen so we don't mess up the user's shell history
        execute!(io::stdout(), EnterAlternateScreen)?;

        // Set up Ctrl+C handler to restore terminal
        ctrlc::set_handler(move || {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            std::process::exit(0);
        })?;

        Ok(Self {
            inner: Mutex::new(TuiWatchViewInner {
                events_buffer: VecDeque::new(),
                footer_lines: Vec::new(),
                session_start_time: None,
                turn_count: 0,
                project_root: None,
            }),
        })
    }

    fn render(&self) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let (term_width, term_height) = terminal::size()?;
        let term_height = term_height as usize;

        // Reserve bottom lines for footer
        let footer_height = inner.footer_lines.len().max(1);
        let content_height = term_height.saturating_sub(footer_height + 1); // +1 for separator

        // Clear screen and move cursor to top
        execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Render content area (recent events)
        let start_idx = inner.events_buffer.len().saturating_sub(content_height);
        for (i, line) in inner.events_buffer.iter().skip(start_idx).enumerate() {
            queue!(
                io::stdout(),
                cursor::MoveTo(0, i as u16),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            print!("{}", line);
        }

        // Render separator line
        let separator_row = content_height as u16;
        queue!(
            io::stdout(),
            cursor::MoveTo(0, separator_row),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )?;
        println!("{}", "─".repeat(term_width as usize));

        // Render footer
        for (i, line) in inner.footer_lines.iter().enumerate() {
            let row = (separator_row + 1 + i as u16).min(term_height as u16 - 1);
            queue!(
                io::stdout(),
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            print!("{}", line);
        }

        io::stdout().flush()?;
        Ok(())
    }
}

impl Drop for TuiWatchView {
    fn drop(&mut self) {
        // Restore terminal state when view is dropped
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
