use crate::reactor::{Reaction, Reactor, ReactorContext, SessionState};
use crate::streaming::{SessionUpdate, SessionWatcher, StreamEvent};
use agtrace_engine::extract_state_updates;
use agtrace_types::{AgentEvent, EventPayload};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
#[derive(Debug)]
pub enum RuntimeEvent {
    SessionAttached {
        display_name: String,
    },
    StateUpdated {
        state: Box<SessionState>,
        new_events: Vec<AgentEvent>,
    },
    ReactionTriggered {
        reactor_name: String,
        reaction: Reaction,
    },
    SessionRotated {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    Waiting {
        message: String,
    },
    FatalError(String),
}

pub struct RuntimeConfig {
    pub provider: Arc<dyn agtrace_providers::LogProvider>,
    pub reactors: Vec<Box<dyn Reactor>>,
    pub watch_path: PathBuf,
    pub explicit_target: Option<String>,
    pub project_root: Option<PathBuf>,
    pub poll_interval: Duration,
}

pub struct Runtime {
    rx: Receiver<RuntimeEvent>,
    _handle: JoinHandle<()>,
}

impl Runtime {
    pub fn start(config: RuntimeConfig) -> Result<Self> {
        let (tx, rx) = channel();
        let mut reactors = config.reactors;
        let watch_path = config.watch_path.clone();
        let explicit_target = config.explicit_target.clone();
        let project_root = config.project_root.clone();
        let provider = config.provider.clone();

        let handle = std::thread::Builder::new()
            .name("agtrace-runtime".to_string())
            .spawn(move || {
                let watcher = match SessionWatcher::new(
                    watch_path,
                    provider,
                    explicit_target,
                    project_root.clone(),
                ) {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.send(RuntimeEvent::FatalError(e.to_string()));
                        return;
                    }
                };

                let mut session_state: Option<SessionState> = None;
                let mut just_attached = false;

                loop {
                    match watcher.receiver().recv() {
                        Ok(event) => match event {
                            StreamEvent::Attached { path, session_id } => {
                                just_attached = true;
                                let display_name =
                                    format_session_display_name(&path, session_id.as_deref());
                                let _ = tx.send(RuntimeEvent::SessionAttached { display_name });
                            }
                            StreamEvent::Update(update) => {
                                if let Err(e) = handle_update(
                                    &update,
                                    &mut session_state,
                                    &mut reactors,
                                    project_root.clone(),
                                    &tx,
                                    just_attached,
                                ) {
                                    let _ = tx.send(RuntimeEvent::FatalError(e.to_string()));
                                    return;
                                }
                                just_attached = false;
                            }
                            StreamEvent::SessionRotated { old_path, new_path } => {
                                session_state = None;
                                let _ =
                                    tx.send(RuntimeEvent::SessionRotated { old_path, new_path });
                            }
                            StreamEvent::Waiting { message } => {
                                let _ = tx.send(RuntimeEvent::Waiting { message });
                            }
                            StreamEvent::Error(msg) => {
                                if msg.starts_with("FATAL:") {
                                    let _ = tx.send(RuntimeEvent::FatalError(msg));
                                    return;
                                }
                            }
                        },
                        Err(_) => {
                            let _ = tx.send(RuntimeEvent::FatalError(
                                "Watch stream ended unexpectedly".to_string(),
                            ));
                            return;
                        }
                    }
                }
            })?;

        Ok(Self {
            rx,
            _handle: handle,
        })
    }

    pub fn receiver(&self) -> &Receiver<RuntimeEvent> {
        &self.rx
    }
}

fn format_session_display_name(path: &Path, session_id: Option<&str>) -> String {
    session_id
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("unknown"))
        })
        .to_string()
}

fn handle_update(
    update: &SessionUpdate,
    session_state: &mut Option<SessionState>,
    reactors: &mut [Box<dyn Reactor>],
    project_root: Option<PathBuf>,
    tx: &std::sync::mpsc::Sender<RuntimeEvent>,
    just_attached: bool,
) -> Result<()> {
    if let Some(assembled_session) = &update.session {
        initialize_session_state(
            session_state,
            assembled_session.session_id.to_string(),
            project_root.clone(),
            assembled_session.start_time,
        );
    }

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
        update_session_state(state, event)?;

        let ctx = ReactorContext { event, state };
        for reactor in reactors.iter_mut() {
            match reactor.handle(ctx) {
                Ok(reaction) => {
                    let _ = tx.send(RuntimeEvent::ReactionTriggered {
                        reactor_name: reactor.name().to_string(),
                        reaction: reaction.clone(),
                    });
                }
                Err(e) => {
                    let _ = tx.send(RuntimeEvent::FatalError(e.to_string()));
                    return Ok(());
                }
            }
        }
    }

    if let Some(state) = session_state.as_ref() {
        if just_attached {
            // No-op currently; future initial summary hook lives here.
        }
        let _ = tx.send(RuntimeEvent::StateUpdated {
            state: Box::new(state.clone()),
            new_events: update.new_events.clone(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{Reactor, ReactorContext};
    use crate::streaming::SessionUpdate;
    use agtrace_types::{AgentEvent, EventPayload, UserPayload};
    use chrono::Utc;
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use uuid::Uuid;

    struct WarnReactor;
    impl Reactor for WarnReactor {
        fn name(&self) -> &str {
            "WarnReactor"
        }
        fn handle(&mut self, _ctx: ReactorContext) -> Result<Reaction> {
            Ok(Reaction::Warn("alert".into()))
        }
    }

    fn user_event() -> AgentEvent {
        AgentEvent {
            id: Uuid::nil(),
            trace_id: Uuid::nil(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::User(UserPayload { text: "hi".into() }),
            metadata: None,
        }
    }

    #[test]
    fn handle_update_emits_reaction_triggered() {
        let (tx_events, rx_events) = channel();

        let mut session_state = None;
        let mut reactors: Vec<Box<dyn Reactor>> = vec![Box::new(WarnReactor)];

        let update = SessionUpdate {
            session: None,
            new_events: vec![user_event()],
            orphaned_events: vec![],
            total_events: 1,
        };

        handle_update(
            &update,
            &mut session_state,
            &mut reactors,
            None,
            &tx_events,
            false,
        )
        .unwrap();

        let mut got_reaction = false;
        for _ in 0..3 {
            if let Ok(RuntimeEvent::ReactionTriggered { reaction, .. }) =
                rx_events.recv_timeout(Duration::from_secs(1))
            {
                assert!(matches!(reaction, Reaction::Warn(_)));
                got_reaction = true;
                break;
            }
        }
        assert!(got_reaction);
    }
}

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

fn update_session_state(state: &mut SessionState, event: &AgentEvent) -> Result<()> {
    state.last_activity = event.timestamp;
    state.event_count += 1;

    let updates = extract_state_updates(event);

    if updates.is_new_turn {
        state.turn_count += 1;
        state.error_count = 0;
    }

    if let EventPayload::ToolResult(result) = &event.payload {
        if updates.is_error && result.is_error {
            state.error_count += 1;
        } else {
            state.error_count = 0;
        }
    }

    if let Some(model) = updates.model {
        if state.model.is_none() {
            state.model = Some(model);
        }
    }

    if let Some(limit) = updates.context_window_limit {
        if state.context_window_limit.is_none() {
            state.context_window_limit = Some(limit);
        }
    }

    if let Some(usage) = updates.usage {
        state.current_usage = usage;
        state.current_reasoning_tokens = updates.reasoning_tokens.unwrap_or(0);
    } else if let Some(reasoning_tokens) = updates.reasoning_tokens {
        state.current_reasoning_tokens = reasoning_tokens;
    }

    Ok(())
}
