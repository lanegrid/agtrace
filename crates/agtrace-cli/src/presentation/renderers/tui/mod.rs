mod app;
mod components;
mod tui_event;
mod ui;

use super::traits::WatchView;
use crate::presentation::formatters::token::TokenUsageView;
use crate::presentation::view_models::DisplayOptions;
use crate::presentation::view_models::{
    EventPayloadViewModel, EventViewModel, ReactionViewModel, WatchStart, WatchSummary,
};
use crate::presentation::views::EventView;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use app::{AppState, ContextUsageState};
use tui_event::TuiEvent;

pub struct TuiWatchView {
    tx: Sender<TuiEvent>,
}

impl TuiWatchView {
    pub(crate) fn new() -> Result<(Self, Receiver<TuiEvent>)> {
        let (tx, rx) = mpsc::channel();
        Ok((Self { tx }, rx))
    }

    #[allow(dead_code)]
    pub(crate) fn sender(&self) -> Sender<TuiEvent> {
        self.tx.clone()
    }

    pub(crate) fn run(rx: Receiver<TuiEvent>) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        ctrlc::set_handler(move || {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            std::process::exit(0);
        })?;

        let mut app_state = AppState::new();
        let mut should_quit = false;

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = std::time::Instant::now();

        while !should_quit {
            terminal.draw(|f| {
                ui::draw(f, &app_state);
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

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

            while let Ok(tui_event) = rx.try_recv() {
                match tui_event {
                    TuiEvent::WatchStart(start) => {
                        let message = match start {
                            WatchStart::Provider { name, log_root } => {
                                app_state.session_title = name.clone();
                                format!("👀 Watching {} ({})", log_root.display(), name)
                            }
                            WatchStart::Session { id, log_root } => {
                                app_state.session_title = id.clone();
                                format!("👀 Watching session {} in {}", id, log_root.display())
                            }
                        };
                        app_state.events_buffer.push_back(message);
                    }
                    TuiEvent::WatchAttached(display_name) => {
                        app_state
                            .events_buffer
                            .push_back(format!("✨ Attached to active session: {}", display_name));
                    }
                    TuiEvent::WatchRotated(old_name, new_name) => {
                        app_state
                            .events_buffer
                            .push_back(format!("✨ Session rotated: {} → {}", old_name, new_name));
                    }
                    TuiEvent::WatchWaiting(message) => {
                        app_state
                            .events_buffer
                            .push_back(format!("⏳ Waiting: {}", message));
                    }
                    TuiEvent::WatchError(message, fatal) => {
                        app_state
                            .events_buffer
                            .push_back(format!("❌ Error: {}", message));
                        if fatal {
                            should_quit = true;
                        }
                    }
                    TuiEvent::StreamUpdate(state, new_events) => {
                        if app_state.session_start_time.is_none() {
                            app_state.session_start_time = Some(state.start_time);
                        }
                        app_state.turn_count = state.turn_count;

                        for event in new_events {
                            let opts = DisplayOptions {
                                enable_color: true,
                                relative_time: true,
                                truncate_text: None,
                            };

                            let event_view = EventView {
                                event: &event,
                                options: &opts,
                                session_start: app_state.session_start_time,
                                turn_context: app_state.turn_count,
                            };

                            let formatted = format!("{}", event_view);
                            if !formatted.is_empty() {
                                app_state.events_buffer.push_back(formatted);

                                if app_state.events_buffer.len() > 1000 {
                                    app_state.events_buffer.pop_front();
                                }
                            }

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
                                app_state.footer_lines =
                                    footer_output.lines().map(|s| s.to_string()).collect();

                                let total_used = state.current_usage.fresh_input
                                    + state.current_usage.cache_creation
                                    + state.current_usage.cache_read
                                    + state.current_usage.output;

                                let input_total = state.current_usage.fresh_input
                                    + state.current_usage.cache_creation
                                    + state.current_usage.cache_read;
                                let input_pct = if total_used > 0 {
                                    input_total as f64 / total_used as f64
                                } else {
                                    0.0
                                };
                                let output_pct = if total_used > 0 {
                                    state.current_usage.output as f64 / total_used as f64
                                } else {
                                    0.0
                                };

                                app_state.context_usage = Some(ContextUsageState {
                                    used: total_used as u64,
                                    limit: state.token_limit.unwrap_or(0),
                                    input_pct,
                                    output_pct,
                                });
                            }
                        }
                    }
                    TuiEvent::Input(_) | TuiEvent::Tick => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = std::time::Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }
}

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
        state: &crate::presentation::view_models::StreamStateViewModel,
        new_events: &[EventViewModel],
    ) -> Result<()> {
        self.tx
            .send(TuiEvent::StreamUpdate(state.clone(), new_events.to_vec()))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
    }
}
