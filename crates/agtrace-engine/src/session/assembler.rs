use std::collections::HashMap;

use super::stats::calculate_session_stats;
use super::turn_builder::TurnBuilder;
use super::types::*;
use agtrace_types::{
    AgentEvent, AgentId, EventPayload, SpawnContext, SystemGeneratedReason, TurnOrigin, UserPayload,
};

/// Whether an agent is the file-owner "main" timeline of a session
/// (i.e. not a Claude subagent living under the session).
fn is_main_agent(agent: &AgentId) -> bool {
    !agent.is_claude_subagent()
}

/// Assemble all agents from events into separate sessions.
///
/// Returns a Vec of AgentSession, one per distinct AgentId found in the events.
/// Each session contains only events from its respective agent (input order kept).
/// For subagent sessions, attempts to link back to the parent turn/step via spawned_by.
pub fn assemble_sessions(events: &[AgentEvent]) -> Vec<AgentSession> {
    if events.is_empty() {
        return Vec::new();
    }

    // Group events by agent
    let mut agents: HashMap<AgentId, Vec<AgentEvent>> = HashMap::new();
    for event in events {
        agents
            .entry(event.agent.clone())
            .or_default()
            .push(event.clone());
    }

    // First, assemble the main agents to build the spawn context map
    let main_events: Vec<AgentEvent> = agents
        .iter()
        .filter(|(agent, _)| is_main_agent(agent))
        .flat_map(|(_, events)| events.iter().cloned())
        .collect();
    let spawn_map = build_spawn_context_map(&main_events);

    // Assemble each agent into a session
    let mut sessions: Vec<AgentSession> = agents
        .into_iter()
        .filter_map(|(agent, agent_events)| {
            let spawned_by = agent
                .native_agent_id()
                .and_then(|aid| spawn_map.get(aid).cloned());
            assemble_session_for_agent(&agent_events, agent, spawned_by)
        })
        .collect();

    // Sort sessions: main agents first, then subagents by start_time
    sessions.sort_by(|a, b| {
        use std::cmp::Ordering;
        match (is_main_agent(&a.agent), is_main_agent(&b.agent)) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.start_time.cmp(&b.start_time),
        }
    });

    sessions
}

/// Build a map from agent_id to SpawnContext by scanning ToolResult events with agent_id.
fn build_spawn_context_map(events: &[AgentEvent]) -> HashMap<String, SpawnContext> {
    let mut spawn_map = HashMap::new();

    // Build turns first to get proper indices
    let turns = build_turns(events);

    for (turn_idx, turn) in turns.iter().enumerate() {
        for (step_idx, step) in turn.steps.iter().enumerate() {
            for tool in &step.tools {
                if let Some(ref result) = tool.result
                    && let Some(ref agent_id) = result.content.agent_id
                {
                    spawn_map.insert(
                        agent_id.clone(),
                        SpawnContext {
                            turn_index: turn_idx,
                            step_index: step_idx,
                        },
                    );
                }
            }
        }
    }

    spawn_map
}

/// Assemble the main agent from events into a session.
///
/// This is the backward-compatible function that keeps the main (non-subagent)
/// agent only. For multi-agent support, use `assemble_sessions()` instead.
pub fn assemble_session(events: &[AgentEvent]) -> Option<AgentSession> {
    let main_agent = events
        .iter()
        .find(|e| is_main_agent(&e.agent))?
        .agent
        .clone();

    // Filter to the main agent's events only
    let main_events: Vec<_> = events
        .iter()
        .filter(|e| e.agent == main_agent)
        .cloned()
        .collect();

    assemble_session_for_agent(&main_events, main_agent, None)
}

/// Internal: Assemble a session from events belonging to a single agent.
fn assemble_session_for_agent(
    events: &[AgentEvent],
    agent: AgentId,
    spawned_by: Option<SpawnContext>,
) -> Option<AgentSession> {
    if events.is_empty() {
        return None;
    }

    let session_id = events.first()?.session_id;
    let start_time = events.first()?.timestamp;
    let end_time = events.last().map(|e| e.timestamp);

    let turns = build_turns(events);
    let stats = calculate_session_stats(&turns, start_time, end_time);

    Some(AgentSession {
        session_id,
        agent,
        spawned_by,
        start_time,
        end_time,
        turns,
        stats,
    })
}

/// Detect the origin of a user message.
///
/// Identifies system-generated messages (like context compaction continuation)
/// vs user-typed messages.
fn detect_turn_origin(content: &UserPayload) -> TurnOrigin {
    // Context compaction pattern from Claude Code
    if content.text.starts_with("This session is being continued") {
        return TurnOrigin::SystemGenerated {
            reason: SystemGeneratedReason::ContextCompaction,
        };
    }

    TurnOrigin::User
}

fn build_turns(events: &[AgentEvent]) -> Vec<AgentTurn> {
    let mut turns = Vec::new();
    let mut current_turn: Option<TurnBuilder> = None;

    for event in events {
        match &event.payload {
            EventPayload::User(user) => {
                // "[Request interrupted by user]" is a termination marker, not a new turn.
                // It indicates the previous turn was interrupted - just finalize that turn.
                if user.text.starts_with("[Request interrupted") {
                    if let Some(builder) = current_turn.take()
                        && let Some(turn) = builder.build()
                    {
                        turns.push(turn);
                    }
                    // Don't start a new turn - wait for actual user input
                    continue;
                }

                // If current turn was started by SlashCommand and has no steps yet,
                // merge this User content into it (slash command expansion)
                if let Some(ref mut builder) = current_turn
                    && builder.is_slash_command_pending()
                {
                    builder.set_expanded_content(user.clone());
                    continue;
                }

                if let Some(builder) = current_turn.take()
                    && let Some(turn) = builder.build()
                {
                    turns.push(turn);
                }

                current_turn = Some(TurnBuilder::new(
                    event.id,
                    event.timestamp,
                    UserMessage {
                        event_id: event.id,
                        content: user.clone(),
                        slash_command: None,
                        origin: detect_turn_origin(user),
                    },
                ));
            }
            EventPayload::SlashCommand(cmd) => {
                if let Some(builder) = current_turn.take()
                    && let Some(turn) = builder.build()
                {
                    turns.push(turn);
                }

                current_turn = Some(TurnBuilder::new_slash_command(
                    event.id,
                    event.timestamp,
                    cmd.clone(),
                ));
            }
            _ => {
                if let Some(ref mut builder) = current_turn {
                    builder.add_event(event);
                }
            }
        }
    }

    if let Some(builder) = current_turn
        && let Some(turn) = builder.build()
    {
        turns.push(turn);
    }

    turns
}
