use crate::context::ExecutionContext;
use crate::reactor::{Reaction, Reactor, ReactorContext, SessionState, Severity};
use crate::reactors::{SafetyGuard, StallDetector, TuiRenderer};
use crate::streaming::{SessionWatcher, StreamEvent};
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;
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

pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()> {
    let (provider, log_root, explicit_target) = match target {
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
    let mut reactors: Vec<Box<dyn Reactor>> = vec![
        Box::new(TuiRenderer::new()),
        Box::new(StallDetector::new(60)), // 60 seconds idle threshold
        Box::new(SafetyGuard::new()),
    ];

    // Session state (initialized on first event)
    let mut session_state: Option<SessionState> = None;

    // Event loop - receive and display events
    // IMPORTANT: Keep watcher alive to maintain file system monitoring
    loop {
        match watcher.receiver().recv() {
            Ok(event) => match event {
                StreamEvent::Attached { path, session_id } => {
                    let display_name = session_id.as_deref().unwrap_or_else(|| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_else(|| path.to_str().unwrap_or("unknown"))
                    });
                    println!(
                        "{}  {}\n",
                        "✨ Attached to active session:".bright_green(),
                        display_name
                    );
                }
                StreamEvent::Update(update) => {
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
                        // Update session state from assembled session
                        if session_state.is_none() {
                            session_state = Some(SessionState::new(
                                assembled_session.session_id.to_string(),
                                project_root.clone(),
                                assembled_session.start_time,
                            ));
                        }
                    }

                    for event in update.new_events {
                        // Initialize session state if not yet set
                        if session_state.is_none() {
                            session_state = Some(SessionState::new(
                                event.trace_id.to_string(),
                                project_root.clone(),
                                event.timestamp,
                            ));
                        }

                        let state = session_state
                            .as_mut()
                            .expect("session_state must be Some after initialization");
                        update_session_state(state, &event);

                        // Run all reactors
                        let ctx = ReactorContext {
                            event: &event,
                            state,
                        };

                        for reactor in &mut reactors {
                            match reactor.handle(ctx) {
                                Ok(reaction) => {
                                    if let Err(e) = handle_reaction(reaction) {
                                        eprintln!("{} {}", "❌ Reaction error:".red(), e);
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "{} {} failed: {}",
                                        "❌ Reactor".red(),
                                        reactor.name(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                StreamEvent::SessionRotated { old_path, new_path } => {
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
                    // Reset session state for new session
                    session_state = None;
                }
                StreamEvent::Waiting { message } => {
                    println!("{} {}", "[⏳ Waiting]".bright_yellow(), message);
                }
                StreamEvent::Error(msg) => {
                    eprintln!("{} {}", "❌ Error:".red(), msg);
                    // Check if this is a fatal error
                    if msg.starts_with("FATAL:") {
                        eprintln!("{}", "Watch stream terminated due to fatal error".red());
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
        EventPayload::TokenUsage(usage) => {
            state.total_input_tokens += usage.input_tokens;
            state.total_output_tokens += usage.output_tokens;
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
