use crate::context::ExecutionContext;
use crate::reactor::{Reaction, Reactor, ReactorContext, SessionState};
use crate::reactors::{SafetyGuard, StallDetector, TokenUsageMonitor, TuiRenderer};
use crate::streaming::{SessionWatcher, StreamEvent};
use crate::token_limits::TokenLimits;
use crate::token_usage::ContextWindowUsage;
use crate::ui::models::{WatchStart, WatchSummary, WatchTokenUsage};
use crate::ui::TraceView;
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub enum WatchTarget {
    Provider {
        name: String,
    },
    Session {
        id: String,
    },
    #[allow(dead_code)]
    File {
        path: PathBuf,
    },
}

/// Create default reactors for watch mode
fn create_reactors() -> Vec<Box<dyn Reactor>> {
    vec![
        Box::new(TuiRenderer::new()),
        Box::new(StallDetector::new(60)), // 60 seconds idle threshold
        Box::new(SafetyGuard::new()),
        Box::new(TokenUsageMonitor::default_thresholds()), // 80% warning, 95% critical
    ]
}

/// Format session display name from path and optional session_id
fn format_session_display_name(path: &Path, session_id: Option<&str>) -> String {
    session_id
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("unknown"))
        })
        .to_string()
}

/// Initialize session state if not already set
fn initialize_session_state(
    session_state: &mut Option<SessionState>,
    session_id: String,
    project_root: Option<PathBuf>,
    timestamp: chrono::DateTime<chrono::Utc>,
) {
    if session_state.is_none() {
        *session_state = Some(SessionState::new(session_id, project_root, timestamp));
    }
}

/// Handle StreamEvent::Error, returns true if fatal
fn handle_error_event(msg: &str, view: &dyn TraceView) -> Result<bool> {
    let is_fatal = msg.starts_with("FATAL:");
    view.on_watch_error(msg, is_fatal)?;
    Ok(is_fatal)
}

/// Handle initial Update after Attached: Initialize SessionState and display summary
fn handle_initial_update(
    update: &crate::streaming::SessionUpdate,
    session_state: &mut Option<SessionState>,
    project_root: Option<PathBuf>,
    view: &dyn TraceView,
) -> Result<WatchSummary> {
    // Initialize SessionState from assembled session if available
    if let Some(assembled_session) = &update.session {
        initialize_session_state(
            session_state,
            assembled_session.session_id.to_string(),
            project_root.clone(),
            assembled_session.start_time,
        );
    }

    // Process all events to update SessionState (without displaying them)
    for event in &update.new_events {
        initialize_session_state(
            session_state,
            event.trace_id.to_string(),
            project_root.clone(),
            event.timestamp,
        );

        let state = session_state
            .as_mut()
            .expect("session_state must be Some after initialization");
        update_session_state(state, event, view)?;
    }

    let mut recent_lines = Vec::new();
    if let Some(assembled_session) = &update.session {
        if !assembled_session.turns.is_empty() {
            let num_turns = assembled_session.turns.len().min(5);
            let start_idx = assembled_session.turns.len().saturating_sub(num_turns);

            let mut recent_session = assembled_session.clone();
            recent_session.turns = assembled_session.turns[start_idx..].to_vec();

            let display = crate::display_model::SessionDisplay::from_agent_session(&recent_session);
            let opts = crate::display_model::DisplayOptions {
                enable_color: true,
                relative_time: false,
                truncate_text: None,
            };

            recent_lines = crate::views::session::format_compact(&display, &opts);
        }
    }

    let summary = if let Some(state) = session_state {
        let total = state.total_context_window_tokens() as u64;
        let token_limits = TokenLimits::new();
        let limit = state.context_window_limit.or_else(|| {
            state
                .model
                .as_ref()
                .and_then(|m| token_limits.get_limit(m).map(|l| l.total_limit))
        });

        let usage = if let Some(limit) = limit {
            let percentages = token_limits.get_usage_percentage_from_state(state);
            let (input_pct, output_pct, total_pct) = percentages
                .map(|(input, output, total)| (Some(input), Some(output), Some(total)))
                .unwrap_or((None, None, None));

            Some(WatchTokenUsage {
                total_tokens: total,
                limit: Some(limit),
                input_pct,
                output_pct,
                total_pct,
            })
        } else {
            Some(WatchTokenUsage {
                total_tokens: total,
                limit: None,
                input_pct: None,
                output_pct: None,
                total_pct: None,
            })
        };

        WatchSummary {
            recent_lines,
            token_usage: usage,
            turn_count: state.turn_count,
        }
    } else {
        WatchSummary {
            recent_lines,
            token_usage: None,
            turn_count: 0,
        }
    };

    Ok(summary)
}

/// Process StreamEvent::Update
fn process_update_event(
    update: &crate::streaming::SessionUpdate,
    session_state: &mut Option<SessionState>,
    reactors: &mut [Box<dyn Reactor>],
    project_root: Option<PathBuf>,
    view: &dyn TraceView,
) -> Result<()> {
    // Log orphaned events if any (pre-session noise)
    if !update.orphaned_events.is_empty() {
        view.on_watch_orphaned(update.orphaned_events.len(), update.total_events)?;
    }

    // Use pre-assembled session if available
    if let Some(assembled_session) = &update.session {
        initialize_session_state(
            session_state,
            assembled_session.session_id.to_string(),
            project_root.clone(),
            assembled_session.start_time,
        );
    }

    // Process new events
    for event in &update.new_events {
        initialize_session_state(
            session_state,
            event.trace_id.to_string(),
            project_root.clone(),
            event.timestamp,
        );

        let state = session_state
            .as_mut()
            .expect("session_state must be Some after initialization");
        update_session_state(state, event, view)?;

        // Run all reactors
        run_reactors(event, state, reactors, view)?;
    }

    if let Some(state) = session_state.as_ref() {
        view.render_stream_update(state, &update.new_events)?;
    }

    Ok(())
}

pub fn handle(ctx: &ExecutionContext, target: WatchTarget, view: &dyn TraceView) -> Result<()> {
    let (provider, log_root, explicit_target, start_event): (
        Arc<dyn agtrace_providers::LogProvider>,
        PathBuf,
        Option<String>,
        WatchStart,
    ) = match target {
        WatchTarget::Provider { name } => {
            let (provider, log_root) = ctx.resolve_provider(&name)?;
            (
                Arc::from(provider),
                log_root.clone(),
                None,
                WatchStart::Provider {
                    name,
                    log_root: log_root.clone(),
                },
            )
        }
        WatchTarget::Session { id } => {
            let provider_name = ctx.default_provider()?;
            let (provider, log_root) = ctx.resolve_provider(&provider_name)?;
            (
                Arc::from(provider),
                log_root.clone(),
                Some(id.clone()),
                WatchStart::Session { id, log_root },
            )
        }
        WatchTarget::File { path: _ } => {
            anyhow::bail!("Direct file watching not yet implemented");
        }
    };

    view.render_watch_start(&start_event)?;

    let project_root = if explicit_target.is_some() {
        None
    } else {
        ctx.project_root.clone()
    };

    // Create session watcher with provider and optional project context
    let watcher = SessionWatcher::new(
        log_root.to_path_buf(),
        provider,
        explicit_target,
        project_root.clone(),
    )?;

    // Initialize reactors
    let mut reactors = create_reactors();

    // Session state (initialized on first event)
    let mut session_state: Option<SessionState> = None;

    // Track if we just attached to a session
    let mut just_attached = false;

    // Event loop - receive and display events
    // IMPORTANT: Keep watcher alive to maintain file system monitoring
    loop {
        match watcher.receiver().recv() {
            Ok(event) => match event {
                StreamEvent::Attached { path, session_id } => {
                    just_attached = true;
                    let display_name = format_session_display_name(&path, session_id.as_deref());
                    view.on_watch_attached(&display_name)?;
                }
                StreamEvent::Update(update) => {
                    if just_attached {
                        // Initial snapshot: Initialize SessionState and display summary only
                        just_attached = false;
                        let summary =
                            handle_initial_update(
                                &update,
                                &mut session_state,
                                project_root.clone(),
                                view,
                            )?;
                        view.on_watch_initial_summary(&summary)?;
                    } else {
                        // Normal update: Process events through reactors
                        process_update_event(
                            &update,
                            &mut session_state,
                            &mut reactors,
                            project_root.clone(),
                            view,
                        )?;
                    }
                }
                StreamEvent::SessionRotated { old_path, new_path } => {
                    view.on_watch_rotated(&old_path, &new_path)?;
                    // Reset session state for new session
                    session_state = None;
                }
                StreamEvent::Waiting { message } => {
                    view.on_watch_waiting(&message)?;
                }
                StreamEvent::Error(msg) => {
                    if handle_error_event(&msg, view)? {
                        break;
                    }
                }
            },
            Err(_) => {
                // Channel disconnected - worker thread terminated
                view.render_warning(&format!(
                    "{}",
                    "⚠️  Watch stream ended (worker thread terminated)".yellow()
                ))?;
                break;
            }
        }
    }

    Ok(())
}

/// Update session state based on incoming event
fn update_session_state(
    state: &mut SessionState,
    event: &AgentEvent,
    view: &dyn TraceView,
) -> Result<()> {
    // Update last activity timestamp
    state.last_activity = event.timestamp;
    state.event_count += 1;

    // Update state based on event type
    match &event.payload {
        EventPayload::User(_) => {
            state.turn_count += 1;
            // Reset error count on new user input
            state.error_count = 0;
        }
        EventPayload::Message(_) => {
            // Extract model name from metadata (Claude: metadata.message.model)
            if state.model.is_none() {
                if let Some(metadata) = &event.metadata {
                    // Try metadata.message.model (Claude format)
                    if let Some(model) = metadata
                        .get("message")
                        .and_then(|m| m.get("model"))
                        .and_then(|v| v.as_str())
                    {
                        state.model = Some(model.to_string());
                    }
                    // Fallback: Try metadata.model (if flat structure)
                    else if let Some(model) = metadata.get("model").and_then(|v| v.as_str()) {
                        state.model = Some(model.to_string());
                    }
                }
            }
        }
        EventPayload::TokenUsage(usage) => {
            // NOTE: Use assignment (=), NOT accumulation (+=)
            //
            // TokenUsage events are snapshots of the current turn, not deltas.
            // Each turn the LLM receives the full history, so:
            // - Turn 1: 1000 tokens → event reports 1000
            // - Turn 2: 1200 tokens (history grew) → event reports 1200
            // Using += would give 2200, severely overreporting context window usage.
            //
            // This applies to ALL token fields including cache tokens:
            // - cache_read_input_tokens: how many tokens reused THIS turn
            // - cache_creation_input_tokens: how many new tokens cached THIS turn

            // Update using type-safe ContextWindowUsage
            let cache_creation = usage
                .details
                .as_ref()
                .and_then(|d| d.cache_creation_input_tokens)
                .unwrap_or(0);
            let cache_read = usage
                .details
                .as_ref()
                .and_then(|d| d.cache_read_input_tokens)
                .unwrap_or(0);

            state.current_usage = ContextWindowUsage::from_raw(
                usage.input_tokens,
                cache_creation,
                cache_read,
                usage.output_tokens,
            );

            if let Some(details) = &usage.details {
                state.current_reasoning_tokens = details.reasoning_output_tokens.unwrap_or(0);
            } else {
                state.current_reasoning_tokens = 0;
            }

            // Validate token counts for current turn
            if let Some(model) = &state.model {
                let token_limits = TokenLimits::new();
                let model_limit = token_limits.get_limit(model).map(|l| l.total_limit);

                if let Err(err) = state.validate_tokens(model_limit) {
                    view.on_watch_token_warning(&format!("{}", err))?;
                }
            }

            // Extract model from metadata if not yet set
            if state.model.is_none() {
                if let Some(metadata) = &event.metadata {
                    if let Some(model) = metadata
                        .get("message")
                        .and_then(|m| m.get("model"))
                        .and_then(|v| v.as_str())
                    {
                        state.model = Some(model.to_string());
                    } else if let Some(model) = metadata.get("model").and_then(|v| v.as_str()) {
                        state.model = Some(model.to_string());
                    }
                }
            }

            // Extract provider-reported context window if present
            if state.context_window_limit.is_none() {
                if let Some(metadata) = &event.metadata {
                    let limit = metadata
                        .get("info")
                        .and_then(|info| info.get("model_context_window"))
                        .and_then(|v| v.as_u64());
                    if let Some(limit) = limit {
                        state.context_window_limit = Some(limit);
                    }
                }
            }
        }
        EventPayload::ToolResult(result) => {
            if result.is_error {
                state.error_count += 1;
            } else {
                state.error_count = 0;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Run all reactors and handle their reactions
fn run_reactors(
    event: &AgentEvent,
    state: &mut SessionState,
    reactors: &mut [Box<dyn Reactor>],
    view: &dyn TraceView,
) -> Result<()> {
    let ctx = ReactorContext { event, state };

    for reactor in reactors {
        match reactor.handle(ctx) {
            Ok(reaction) => {
                if let Err(e) = handle_reaction(reaction, view) {
                    view.on_watch_reaction_error(&format!("{}", e))?;
                }
            }
            Err(e) => {
                view.on_watch_reactor_error(reactor.name(), &format!("{}", e))?;
            }
        }
    }

    Ok(())
}

/// Handle reactor reaction
fn handle_reaction(reaction: Reaction, view: &dyn TraceView) -> Result<()> {
    view.on_watch_reaction(&reaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::Severity;
    use agtrace_types::v2::{TokenUsageDetails, TokenUsagePayload, ToolResultPayload, UserPayload};
    use chrono::Utc;
    use std::str::FromStr;
    use crate::ui::ConsoleTraceView;

    fn test_view() -> ConsoleTraceView {
        ConsoleTraceView::new()
    }

    fn create_test_event(payload: EventPayload) -> AgentEvent {
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let trace_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();

        AgentEvent {
            id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            payload,
            metadata: None,
        }
    }

    #[test]
    fn test_format_session_display_name_with_session_id() {
        let path = PathBuf::from("/path/to/session.jsonl");
        let result = format_session_display_name(&path, Some("test-session-123"));
        assert_eq!(result, "test-session-123");
    }

    #[test]
    fn test_format_session_display_name_without_session_id() {
        let path = PathBuf::from("/path/to/session.jsonl");
        let result = format_session_display_name(&path, None);
        assert_eq!(result, "session.jsonl");
    }

    #[test]
    fn test_format_session_display_name_fallback() {
        let path = PathBuf::from("");
        let result = format_session_display_name(&path, None);
        // Empty path results in empty string (edge case)
        assert_eq!(result, "");
    }

    #[test]
    fn test_create_reactors() {
        let reactors = create_reactors();
        assert_eq!(reactors.len(), 4);
        assert_eq!(reactors[0].name(), "TuiRenderer");
        assert_eq!(reactors[1].name(), "StallDetector");
        assert_eq!(reactors[2].name(), "SafetyGuard");
        assert_eq!(reactors[3].name(), "TokenUsageMonitor");
    }

    #[test]
    fn test_update_session_state_user_event() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        let event = create_test_event(EventPayload::User(UserPayload {
            text: "test".to_string(),
        }));
        let view = test_view();

        update_session_state(&mut state, &event, &view).unwrap();

        assert_eq!(state.turn_count, 1);
        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_token_usage() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        let event = create_test_event(EventPayload::TokenUsage(TokenUsagePayload {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            details: Some(TokenUsageDetails {
                cache_creation_input_tokens: None,
                cache_read_input_tokens: Some(20),
                reasoning_output_tokens: Some(10),
                audio_input_tokens: None,
                audio_output_tokens: None,
            }),
        }));
        let view = test_view();

        update_session_state(&mut state, &event, &view).unwrap();

        // Tokens are snapshots, not cumulative
        assert_eq!(state.current_usage.fresh_input.0, 100);
        assert_eq!(state.current_usage.output.0, 50);
        assert_eq!(state.current_usage.cache_read.0, 20);
        assert_eq!(state.current_reasoning_tokens, 10);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_tool_result_success() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.error_count = 5;
        let view = test_view();

        let tool_call_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap();
        let event = create_test_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id,
            output: "success".to_string(),
            is_error: false,
        }));

        update_session_state(&mut state, &event, &view).unwrap();

        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_tool_result_error() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        let view = test_view();

        let tool_call_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap();
        let event = create_test_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id,
            output: "error".to_string(),
            is_error: true,
        }));

        update_session_state(&mut state, &event, &view).unwrap();

        assert_eq!(state.error_count, 1);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_token_usage_with_model() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        assert!(state.model.is_none());
        let view = test_view();

        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000005").unwrap();
        let trace_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000006").unwrap();

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "model".to_string(),
            serde_json::json!("claude-3-5-sonnet-20241022"),
        );

        let event = AgentEvent {
            id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
                details: None,
            }),
            metadata: Some(serde_json::Value::Object(metadata)),
        };

        update_session_state(&mut state, &event, &view).unwrap();

        assert_eq!(state.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(state.current_usage.fresh_input.0, 100);
        assert_eq!(state.current_usage.output.0, 50);
    }

    #[test]
    fn test_handle_reaction_continue() {
        let view = test_view();
        let result = handle_reaction(Reaction::Continue, &view);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_warn() {
        let view = test_view();
        let result = handle_reaction(Reaction::Warn("test warning".to_string()), &view);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_intervene_notification() {
        let view = test_view();
        let result = handle_reaction(Reaction::Intervene {
            reason: "test alert".to_string(),
            severity: Severity::Notification,
        }, &view);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_intervene_kill() {
        let view = test_view();
        let result = handle_reaction(Reaction::Intervene {
            reason: "emergency".to_string(),
            severity: Severity::Kill,
        }, &view);
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_session_state_creates_new_state() {
        let mut session_state = None;
        let timestamp = Utc::now();

        initialize_session_state(&mut session_state, "test-id".to_string(), None, timestamp);

        assert!(session_state.is_some());
        let state = session_state.unwrap();
        assert_eq!(state.session_id, "test-id");
        assert_eq!(state.start_time, timestamp);
    }

    #[test]
    fn test_initialize_session_state_preserves_existing_state() {
        let initial_timestamp = Utc::now();
        let mut session_state = Some(SessionState::new(
            "original-id".to_string(),
            None,
            initial_timestamp,
        ));

        let later_timestamp = initial_timestamp + chrono::Duration::seconds(10);
        initialize_session_state(
            &mut session_state,
            "new-id".to_string(),
            None,
            later_timestamp,
        );

        let state = session_state.unwrap();
        assert_eq!(state.session_id, "original-id");
        assert_eq!(state.start_time, initial_timestamp);
    }

    #[test]
    fn test_handle_error_event_fatal() {
        let view = test_view();
        let is_fatal = handle_error_event("FATAL: database corrupted", &view).unwrap();
        assert!(is_fatal);
    }

    #[test]
    fn test_handle_error_event_non_fatal() {
        let view = test_view();
        let is_fatal = handle_error_event("WARNING: slow response", &view).unwrap();
        assert!(!is_fatal);
    }

    #[test]
    fn test_process_update_event_initializes_state() {
        use crate::streaming::SessionUpdate;

        let mut session_state = None;
        let mut reactors = create_reactors();
        let view = test_view();

        let user_event = create_test_event(EventPayload::User(UserPayload {
            text: "test".to_string(),
        }));

        let update = SessionUpdate {
            session: None,
            new_events: vec![user_event.clone()],
            orphaned_events: vec![],
            total_events: 1,
        };

        process_update_event(&update, &mut session_state, &mut reactors, None, &view).unwrap();

        assert!(session_state.is_some());
        let state = session_state.unwrap();
        assert_eq!(state.event_count, 1);
        assert_eq!(state.turn_count, 1);
    }

    #[test]
    fn test_process_update_event_with_assembled_session() {
        use crate::streaming::SessionUpdate;
        use agtrace_engine::{AgentSession, SessionStats};

        let mut session_state = None;
        let mut reactors = create_reactors();
        let view = test_view();

        let start_time = Utc::now();
        let session_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000010").unwrap();
        let assembled_session = AgentSession {
            session_id,
            start_time,
            end_time: Some(start_time),
            turns: vec![],
            stats: SessionStats {
                total_turns: 0,
                duration_seconds: 0,
                total_tokens: 0,
            },
        };

        let user_event = create_test_event(EventPayload::User(UserPayload {
            text: "test".to_string(),
        }));

        let update = SessionUpdate {
            session: Some(assembled_session),
            new_events: vec![user_event.clone()],
            orphaned_events: vec![],
            total_events: 1,
        };

        process_update_event(&update, &mut session_state, &mut reactors, None, &view).unwrap();

        assert!(session_state.is_some());
        let state = session_state.unwrap();
        assert_eq!(state.session_id, session_id.to_string());
        assert_eq!(state.start_time, start_time);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_process_update_event_multiple_events() {
        use crate::streaming::SessionUpdate;

        let mut session_state = None;
        let mut reactors = create_reactors();
        let view = test_view();

        let events = vec![
            create_test_event(EventPayload::User(UserPayload {
                text: "first".to_string(),
            })),
            create_test_event(EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens: 50,
                output_tokens: 30,
                total_tokens: 80,
                details: None,
            })),
            create_test_event(EventPayload::User(UserPayload {
                text: "second".to_string(),
            })),
        ];

        let update = SessionUpdate {
            session: None,
            new_events: events,
            orphaned_events: vec![],
            total_events: 3,
        };

        process_update_event(&update, &mut session_state, &mut reactors, None, &view).unwrap();

        assert!(session_state.is_some());
        let state = session_state.unwrap();
        assert_eq!(state.event_count, 3);
        assert_eq!(state.turn_count, 2);
        assert_eq!(state.current_usage.fresh_input.0, 50);
        assert_eq!(state.current_usage.output.0, 30);
    }
}
