use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
use crate::token_limits::TokenLimits;
use crate::ui::traits::WatchView;
use crate::views::session::{format_event_with_start, format_token_summary};
use agtrace_runtime::reactor::{Reaction, SessionState};
use agtrace_types::{AgentEvent, EventPayload};
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
    fn render_watch_start(&self, start: &crate::ui::models::WatchStart) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let message = match start {
            crate::ui::models::WatchStart::Provider { name, log_root } => {
                format!("👀 Watching {} ({})", log_root.display(), name)
            }
            crate::ui::models::WatchStart::Session { id, log_root } => {
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
        inner.events_buffer.push_back(format!("✨ Attached to active session: {}", display_name));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_initial_summary(
        &self,
        _summary: &crate::ui::models::WatchSummary,
    ) -> Result<()> {
        // Initial summary already shown by render_watch_start
        Ok(())
    }

    fn on_watch_rotated(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let old_name = old_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| old_path.display().to_string());
        let new_name = new_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| new_path.display().to_string());
        inner.events_buffer.push_back(format!("✨ Session rotated: {} → {}", old_name, new_name));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_waiting(&self, message: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.events_buffer.push_back(format!("⏳ Waiting: {}", message));
        drop(inner);
        self.render()?;
        Ok(())
    }

    fn on_watch_error(&self, message: &str, _fatal: bool) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.events_buffer.push_back(format!("❌ Error: {}", message));
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

    fn on_watch_reaction(&self, _reaction: &Reaction) -> Result<()> {
        Ok(())
    }

    fn render_stream_update(&self, state: &SessionState, new_events: &[AgentEvent]) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();

        // Update tracking state
        if inner.session_start_time.is_none() {
            inner.session_start_time = Some(state.start_time);
        }
        inner.turn_count = state.turn_count;
        inner.project_root = state.project_root.clone();

        // Format and buffer new events
        for event in new_events {
            if let Some(line) = format_event_with_start(
                event,
                inner.turn_count,
                inner.project_root.as_deref(),
                inner.session_start_time,
            ) {
                inner.events_buffer.push_back(line);

                // Keep buffer size manageable (last 1000 events)
                if inner.events_buffer.len() > 1000 {
                    inner.events_buffer.pop_front();
                }
            }

            // Update footer on TokenUsage events
            if matches!(event.payload, EventPayload::TokenUsage(_)) {
                let token_limits = TokenLimits::new();
                let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));

                let limit = state
                    .context_window_limit
                    .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

                let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

                let summary = TokenSummaryDisplay {
                    input: state.total_input_side_tokens(),
                    output: state.total_output_side_tokens(),
                    cache_creation: state.current_usage.cache_creation.0,
                    cache_read: state.current_usage.cache_read.0,
                    total: state.total_context_window_tokens(),
                    limit,
                    model: state.model.clone(),
                    compaction_buffer_pct,
                };

                let opts = DisplayOptions {
                    enable_color: true,
                    relative_time: false,
                    truncate_text: None,
                };

                inner.footer_lines = format_token_summary(&summary, &opts);
            }
        }

        // Drop the lock before rendering
        drop(inner);

        self.render()?;
        Ok(())
    }
}
