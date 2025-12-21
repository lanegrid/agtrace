mod app;
mod components;
mod mapper;
mod tui_event;
mod ui;

use super::traits::WatchView;
use crate::presentation::view_models::{
    EventViewModel, ReactionViewModel, WatchStart, WatchSummary,
};
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
pub use tui_event::TuiEvent;

pub struct TuiWatchView {
    tx: Sender<TuiEvent>,
}

impl TuiWatchView {
    pub fn new() -> Result<(Self, Receiver<TuiEvent>)> {
        let (tx, rx) = mpsc::channel();
        Ok((Self { tx }, rx))
    }

    #[allow(dead_code)]
    pub(crate) fn sender(&self) -> Sender<TuiEvent> {
        self.tx.clone()
    }

    pub fn run(rx: Receiver<TuiEvent>) -> Result<()> {
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
                ui::draw(f, &mut app_state);
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
                        KeyCode::Down | KeyCode::Char('j') => {
                            app_state.select_next();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app_state.select_previous();
                        }
                        _ => {}
                    }
                }
            }

            while let Ok(tui_event) = rx.try_recv() {
                match tui_event {
                    TuiEvent::WatchStart(start) => {
                        let message = match &start {
                            WatchStart::Provider { name, log_root } => {
                                app_state.session_title = name.clone();
                                app_state.provider_name = Some(name.clone());
                                format!("👀 Watching {} ({})", log_root.display(), name)
                            }
                            WatchStart::Session { id, log_root } => {
                                app_state.session_title = id.clone();
                                app_state.attached_session_id = Some(id.clone());
                                format!("👀 Watching session {} in {}", id, log_root.display())
                            }
                        };
                        app_state.add_system_message(message);
                    }
                    TuiEvent::WatchAttached(display_name) => {
                        app_state.reset_session_state(display_name.clone());
                        app_state.add_system_message(format!(
                            "✨ Attached to active session: {}",
                            display_name
                        ));
                    }
                    TuiEvent::WatchRotated(old_name, new_name) => {
                        app_state.reset_session_state(new_name.clone());
                        app_state.add_system_message(format!(
                            "✨ Session rotated: {} → {}",
                            old_name, new_name
                        ));
                    }
                    TuiEvent::WatchWaiting(message) => {
                        app_state.add_system_message(format!("⏳ Waiting: {}", message));
                    }
                    TuiEvent::WatchError(message, fatal) => {
                        app_state.add_system_message(format!("❌ Error: {}", message));
                        if fatal {
                            should_quit = true;
                        }
                    }
                    TuiEvent::StreamUpdate(state, new_events, turns_data) => {
                        use crate::presentation::view_models::{
                            EventPayloadViewModel, TurnUsageViewModel,
                        };

                        if app_state.session_start_time.is_none() {
                            app_state.session_start_time = Some(state.start_time);
                        }
                        app_state.turn_count = state.turn_count;
                        app_state.model = state.model.clone();
                        app_state.compaction_buffer_pct = state.compaction_buffer_pct;

                        if app_state.max_context.is_none() && state.token_limit.is_some() {
                            app_state.max_context = Some(state.token_limit.unwrap() as u32);
                        }

                        // Update turns data from assembled session (if provided)
                        if let Some(turns) = turns_data {
                            app_state.turns_usage = turns;

                            // Set current_turn_start_tokens and previous_token_total from the last turn
                            if let Some(last_turn) = app_state.turns_usage.last() {
                                let last_total = last_turn.prev_total + last_turn.delta;
                                app_state.previous_token_total = last_total;
                                app_state.current_turn_start_tokens = last_total;
                            }
                        }

                        let is_initial_load = false; // No longer needed

                        for event in new_events {
                            app_state.add_event(&event);
                            app_state.current_step_number += 1;
                            app_state.last_activity = Some(event.timestamp);
                            app_state.activity_timestamps.push_back(event.timestamp);
                            if app_state.activity_timestamps.len() > 20 {
                                app_state.activity_timestamps.pop_front();
                            }

                            match &event.payload {
                                EventPayloadViewModel::User { text } => {
                                    app_state.intent_events.push_back(event.clone());
                                    if app_state.intent_events.len() > 5 {
                                        app_state.intent_events.pop_front();
                                    }

                                    if !is_initial_load {
                                        app_state.current_user_message = text.clone();
                                        let input_total = (state.current_usage.fresh_input
                                            + state.current_usage.cache_creation
                                            + state.current_usage.cache_read)
                                            as u32;
                                        app_state.current_turn_start_tokens = input_total;
                                    }
                                }
                                EventPayloadViewModel::Reasoning { .. }
                                | EventPayloadViewModel::Message { .. }
                                | EventPayloadViewModel::ToolCall { .. } => {
                                    app_state.intent_events.push_back(event.clone());
                                    if app_state.intent_events.len() > 5 {
                                        app_state.intent_events.pop_front();
                                    }
                                }
                                EventPayloadViewModel::TokenUsage { .. } => {
                                    let total_used = state.current_usage.fresh_input
                                        + state.current_usage.cache_creation
                                        + state.current_usage.cache_read
                                        + state.current_usage.output;

                                    let input_total = (state.current_usage.fresh_input
                                        + state.current_usage.cache_creation
                                        + state.current_usage.cache_read)
                                        as u32;

                                    if !is_initial_load {
                                        let delta = input_total
                                            .saturating_sub(app_state.current_turn_start_tokens);

                                        if delta > 0 && !app_state.current_user_message.is_empty() {
                                            // Heavy threshold: 10% of max context, or fallback to 15k tokens
                                            let heavy_threshold = app_state
                                                .max_context
                                                .map(|mc| mc / 10)
                                                .unwrap_or(15000);

                                            let turn_usage = TurnUsageViewModel {
                                                turn_id: app_state.turns_usage.len() + 1,
                                                title: truncate_text(
                                                    &app_state.current_user_message,
                                                    60,
                                                ),
                                                prev_total: app_state.current_turn_start_tokens,
                                                delta,
                                                is_heavy: delta >= heavy_threshold,
                                                is_active: true,
                                                recent_steps: Vec::new(),
                                                start_time: Some(chrono::Utc::now()),
                                            };

                                            app_state.turns_usage.push(turn_usage);

                                            if app_state.turns_usage.len() > 50 {
                                                app_state.turns_usage.remove(0);
                                            }
                                        }

                                        app_state.current_user_message.clear();
                                    }

                                    app_state.previous_token_total = total_used as u32;

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
                                        fresh_input: state.current_usage.fresh_input,
                                        cache_creation: state.current_usage.cache_creation,
                                        cache_read: state.current_usage.cache_read,
                                        output: state.current_usage.output,
                                    });
                                }
                                _ => {}
                            }
                        }

                        app_state.on_tick();
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
        turns: Option<&[crate::presentation::view_models::TurnUsageViewModel]>,
    ) -> Result<()> {
        self.tx
            .send(TuiEvent::StreamUpdate(
                state.clone(),
                new_events.to_vec(),
                turns.map(|t| t.to_vec()),
            ))
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))
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

pub(crate) fn build_turns_from_session(
    session: &agtrace_engine::AgentSession,
    max_context: Option<u32>,
) -> Vec<crate::presentation::view_models::TurnUsageViewModel> {
    use crate::presentation::view_models::{StepItemViewModel, TurnUsageViewModel};

    let mut turns = Vec::new();
    let mut cumulative_input = 0u32;

    // Heavy threshold: 10% of max context, or fallback to 15k tokens
    let heavy_threshold = max_context.map(|mc| mc / 10).unwrap_or(15000);
    let total_turns = session.turns.len();

    for (idx, turn) in session.turns.iter().enumerate() {
        // Get the last step's cumulative token count for this turn
        let turn_end_cumulative: u32 = turn
            .steps
            .iter()
            .rev()
            .find_map(|step| step.usage.as_ref())
            .map(|usage| {
                (usage.input_tokens
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_creation_input_tokens)
                        .unwrap_or(0)
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_read_input_tokens)
                        .unwrap_or(0)) as u32
            })
            .unwrap_or(cumulative_input);

        let delta = turn_end_cumulative.saturating_sub(cumulative_input);
        let prev_total = cumulative_input;

        let user_message = &turn.user.content.text;
        let title = truncate_text(user_message, 60);

        // Check if this turn is active based on the last step's status
        let is_active = if idx == total_turns.saturating_sub(1) {
            // For the last turn, check if the last step is InProgress
            turn.steps
                .last()
                .map(|step| matches!(step.status, agtrace_engine::session::types::StepStatus::InProgress))
                .unwrap_or(false)
        } else {
            false
        };

        let recent_steps = if is_active {
            turn.steps
                .iter()
                .rev()
                .take(5)
                .rev()
                .map(|step| {
                    let (emoji, description) = if let Some(reasoning) = &step.reasoning {
                        ("🤔".to_string(), truncate_text(&reasoning.content.text, 40))
                    } else if let Some(message) = &step.message {
                        ("💬".to_string(), truncate_text(&message.content.text, 40))
                    } else if !step.tools.is_empty() {
                        let tool_name = &step.tools[0].call.content.name;
                        ("🔧".to_string(), format!("Tool: {}", tool_name))
                    } else {
                        ("•".to_string(), "Event".to_string())
                    };

                    let token_usage = step.usage.as_ref().map(|u| {
                        (u.input_tokens
                            + u.details
                                .as_ref()
                                .and_then(|d| d.cache_creation_input_tokens)
                                .unwrap_or(0)
                            + u.details
                                .as_ref()
                                .and_then(|d| d.cache_read_input_tokens)
                                .unwrap_or(0)) as u32
                    });

                    StepItemViewModel {
                        timestamp: step.timestamp,
                        emoji,
                        description,
                        token_usage,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let start_time = turn.steps.first().map(|s| s.timestamp);

        let turn_usage = TurnUsageViewModel {
            turn_id: idx + 1,
            title,
            prev_total,
            delta,
            is_heavy: delta >= heavy_threshold,
            is_active,
            recent_steps,
            start_time,
        };

        turns.push(turn_usage);
        cumulative_input += delta;
    }

    turns
}
