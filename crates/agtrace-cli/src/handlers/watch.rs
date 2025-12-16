use crate::context::ExecutionContext;
use crate::reactor::{Reaction, Reactor, ReactorContext, SessionState, Severity};
use crate::reactors::{SafetyGuard, StallDetector, TokenUsageMonitor, TuiRenderer};
use crate::streaming::{SessionWatcher, StreamEvent};
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

/// Print session rotated message
fn print_session_rotated(old_path: &Path, new_path: &Path) {
    let old_name = old_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| old_path.display().to_string());
    let new_name = new_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| new_path.display().to_string());
    println!(
        "\n{} {} → {}\n",
        "✨ Session rotated:".bright_green(),
        old_name.dimmed(),
        new_name
    );
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

/// Handle StreamEvent::Waiting
fn handle_waiting_event(message: &str) {
    println!("{} {}", "[⏳ Waiting]".bright_yellow(), message);
}

/// Handle StreamEvent::Error, returns true if fatal
fn handle_error_event(msg: &str) -> bool {
    eprintln!("{} {}", "❌ Error:".red(), msg);
    let is_fatal = msg.starts_with("FATAL:");
    if is_fatal {
        eprintln!("{}", "Watch stream terminated due to fatal error".red());
    }
    is_fatal
}

/// Handle initial Update after Attached: Initialize SessionState and display summary
fn handle_initial_update(
    update: crate::streaming::SessionUpdate,
    session_state: &mut Option<SessionState>,
    project_root: Option<PathBuf>,
) {
    use crate::output::{format_session_compact, CompactFormatOpts};
    use crate::token_limits::TokenLimits;

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
        update_session_state(state, event);
    }

    // Display summary: last 5 turns + token usage
    if let Some(assembled_session) = &update.session {
        if !assembled_session.turns.is_empty() {
            let num_turns = assembled_session.turns.len().min(5);
            let start_idx = assembled_session.turns.len().saturating_sub(num_turns);

            println!("{}  Last {} turn(s):\n", "📜".dimmed(), num_turns);

            let opts = CompactFormatOpts {
                enable_color: true,
                relative_time: false,
            };

            let mut recent_session = assembled_session.clone();
            recent_session.turns = assembled_session.turns[start_idx..].to_vec();

            let lines = format_session_compact(&recent_session, &opts);
            for line in lines {
                println!("  {}", line);
            }
            println!();
        }
    }

    // Display token usage summary
    if let Some(state) = session_state {
        let total = state.total_context_window_tokens() as u64;

        if total > 0 {
            if state.model.is_some() {
                let token_limits = TokenLimits::new();
                if let Some((input_pct, output_pct, total_pct)) =
                    token_limits.get_usage_percentage_from_state(state)
                {
                    let model = state.model.as_ref().unwrap();
                    let bar = create_progress_bar(total_pct);
                    let color_fn: fn(&str) -> String = if total_pct >= 95.0 {
                        |s: &str| s.red().to_string()
                    } else if total_pct >= 80.0 {
                        |s: &str| s.yellow().to_string()
                    } else {
                        |s: &str| s.green().to_string()
                    };

                    println!(
                        "{}  {} {} {:.1}% (in: {:.1}%, out: {:.1}%) - {}/{} tokens",
                        "📊".dimmed(),
                        "Current usage:".bright_black(),
                        color_fn(&bar),
                        total_pct,
                        input_pct,
                        output_pct,
                        total,
                        token_limits.get_limit(model).unwrap().total_limit
                    );
                }
            }
        }

        println!(
            "{}  {} total turns processed\n",
            "📝".dimmed(),
            state.turn_count
        );
    }
}

fn create_progress_bar(percentage: f64) -> String {
    let bar_width = 20;
    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Process StreamEvent::Update
fn process_update_event(
    update: crate::streaming::SessionUpdate,
    session_state: &mut Option<SessionState>,
    reactors: &mut [Box<dyn Reactor>],
    project_root: Option<PathBuf>,
) {
    // Log orphaned events if any (pre-session noise)
    if !update.orphaned_events.is_empty() {
        eprintln!(
            "{} {} orphaned events (pre-session noise), {} total events in file",
            "[DEBUG]".dimmed(),
            update.orphaned_events.len(),
            update.total_events
        );
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
    for event in update.new_events {
        initialize_session_state(
            session_state,
            event.trace_id.to_string(),
            project_root.clone(),
            event.timestamp,
        );

        let state = session_state
            .as_mut()
            .expect("session_state must be Some after initialization");
        update_session_state(state, &event);

        // Run all reactors
        run_reactors(&event, state, reactors);
    }
}

pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()> {
    let (provider, log_root, explicit_target): (
        Arc<dyn agtrace_providers::LogProvider>,
        PathBuf,
        Option<String>,
    ) = match target {
        WatchTarget::Provider { name } => {
            let (provider, log_root) = ctx.resolve_provider(&name)?;
            println!(
                "{} {} ({})",
                "[👀 Watching]".bright_cyan(),
                log_root.display(),
                name
            );
            (Arc::from(provider), log_root, None)
        }
        WatchTarget::Session { id } => {
            let provider_name = ctx.default_provider()?;
            let (provider, log_root) = ctx.resolve_provider(&provider_name)?;
            println!(
                "{} session {} in {}",
                "[👀 Watching]".bright_cyan(),
                id,
                log_root.display()
            );
            (Arc::from(provider), log_root, Some(id))
        }
        WatchTarget::File { path: _ } => {
            anyhow::bail!("Direct file watching not yet implemented");
        }
    };

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
                    println!(
                        "{}  {}\n",
                        "✨ Attached to active session:".bright_green(),
                        display_name
                    );
                }
                StreamEvent::Update(update) => {
                    if just_attached {
                        // Initial snapshot: Initialize SessionState and display summary only
                        just_attached = false;
                        handle_initial_update(update, &mut session_state, project_root.clone());
                    } else {
                        // Normal update: Process events through reactors
                        process_update_event(
                            update,
                            &mut session_state,
                            &mut reactors,
                            project_root.clone(),
                        );
                    }
                }
                StreamEvent::SessionRotated { old_path, new_path } => {
                    print_session_rotated(&old_path, &new_path);
                    // Reset session state for new session
                    session_state = None;
                }
                StreamEvent::Waiting { message } => {
                    handle_waiting_event(&message);
                }
                StreamEvent::Error(msg) => {
                    if handle_error_event(&msg) {
                        break;
                    }
                }
            },
            Err(_) => {
                // Channel disconnected - worker thread terminated
                eprintln!(
                    "{}",
                    "⚠️  Watch stream ended (worker thread terminated)".yellow()
                );
                break;
            }
        }
    }

    Ok(())
}

/// Update session state based on incoming event
fn update_session_state(state: &mut SessionState, event: &AgentEvent) {
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
            state.total_input_tokens += usage.input_tokens;
            state.total_output_tokens += usage.output_tokens;

            if let Some(details) = &usage.details {
                if let Some(cache_creation) = details.cache_creation_input_tokens {
                    state.cache_creation_tokens += cache_creation;
                }
                if let Some(cached) = details.cache_read_input_tokens {
                    state.cache_read_tokens += cached;
                }
                if let Some(thinking) = details.reasoning_output_tokens {
                    state.reasoning_tokens += thinking;
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
}

/// Run all reactors and handle their reactions
fn run_reactors(event: &AgentEvent, state: &mut SessionState, reactors: &mut [Box<dyn Reactor>]) {
    let ctx = ReactorContext { event, state };

    for reactor in reactors {
        match reactor.handle(ctx) {
            Ok(reaction) => {
                if let Err(e) = handle_reaction(reaction) {
                    eprintln!("{} {}", "❌ Reaction error:".red(), e);
                }
            }
            Err(e) => {
                eprintln!("{} {} failed: {}", "❌ Reactor".red(), reactor.name(), e);
            }
        }
    }
}

/// Handle reactor reaction
fn handle_reaction(reaction: Reaction) -> Result<()> {
    match reaction {
        Reaction::Continue => {}
        Reaction::Warn(message) => {
            eprintln!("{} {}", "⚠️  Warning:".yellow(), message);
        }
        Reaction::Intervene { reason, severity } => match severity {
            Severity::Notification => {
                eprintln!("{} {}", "🚨 ALERT:".red().bold(), reason);
                // Future: send desktop notification
            }
            Severity::Kill => {
                eprintln!("{} {}", "🚨 EMERGENCY STOP:".red().bold(), reason);
                // Future: kill child process (v0.2.0)
                // For now, just log the alert
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::v2::{TokenUsageDetails, TokenUsagePayload, ToolResultPayload, UserPayload};
    use chrono::Utc;
    use std::str::FromStr;

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

        update_session_state(&mut state, &event);

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

        update_session_state(&mut state, &event);

        assert_eq!(state.total_input_tokens, 100);
        assert_eq!(state.total_output_tokens, 50);
        assert_eq!(state.cache_read_tokens, 20);
        assert_eq!(state.reasoning_tokens, 10);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_tool_result_success() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        state.error_count = 5;

        let tool_call_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap();
        let event = create_test_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id,
            output: "success".to_string(),
            is_error: false,
        }));

        update_session_state(&mut state, &event);

        assert_eq!(state.error_count, 0);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_tool_result_error() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());

        let tool_call_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap();
        let event = create_test_event(EventPayload::ToolResult(ToolResultPayload {
            tool_call_id,
            output: "error".to_string(),
            is_error: true,
        }));

        update_session_state(&mut state, &event);

        assert_eq!(state.error_count, 1);
        assert_eq!(state.event_count, 1);
    }

    #[test]
    fn test_update_session_state_token_usage_with_model() {
        let mut state = SessionState::new("test".to_string(), None, Utc::now());
        assert!(state.model.is_none());

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

        update_session_state(&mut state, &event);

        assert_eq!(state.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(state.total_input_tokens, 100);
        assert_eq!(state.total_output_tokens, 50);
    }

    #[test]
    fn test_handle_reaction_continue() {
        let result = handle_reaction(Reaction::Continue);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_warn() {
        let result = handle_reaction(Reaction::Warn("test warning".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_intervene_notification() {
        let result = handle_reaction(Reaction::Intervene {
            reason: "test alert".to_string(),
            severity: Severity::Notification,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_reaction_intervene_kill() {
        let result = handle_reaction(Reaction::Intervene {
            reason: "emergency".to_string(),
            severity: Severity::Kill,
        });
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
        let is_fatal = handle_error_event("FATAL: database corrupted");
        assert!(is_fatal);
    }

    #[test]
    fn test_handle_error_event_non_fatal() {
        let is_fatal = handle_error_event("WARNING: slow response");
        assert!(!is_fatal);
    }

    #[test]
    fn test_process_update_event_initializes_state() {
        use crate::streaming::SessionUpdate;

        let mut session_state = None;
        let mut reactors = create_reactors();

        let user_event = create_test_event(EventPayload::User(UserPayload {
            text: "test".to_string(),
        }));

        let update = SessionUpdate {
            session: None,
            new_events: vec![user_event.clone()],
            orphaned_events: vec![],
            total_events: 1,
        };

        process_update_event(update, &mut session_state, &mut reactors, None);

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

        process_update_event(update, &mut session_state, &mut reactors, None);

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

        process_update_event(update, &mut session_state, &mut reactors, None);

        assert!(session_state.is_some());
        let state = session_state.unwrap();
        assert_eq!(state.event_count, 3);
        assert_eq!(state.turn_count, 2);
        assert_eq!(state.total_input_tokens, 50);
        assert_eq!(state.total_output_tokens, 30);
    }
}
