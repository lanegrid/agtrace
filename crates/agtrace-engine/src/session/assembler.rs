use std::collections::HashMap;

use super::stats::calculate_session_stats;
use super::turn_builder::TurnBuilder;
use super::types::*;
use agtrace_types::{AgentEvent, EventPayload, SpawnContext, StreamId};

/// Assemble all streams from events into separate sessions.
///
/// Returns a Vec of AgentSession, one per distinct StreamId found in the events.
/// Each session contains only events from its respective stream.
/// For sidechain sessions, attempts to link back to the parent turn/step via spawned_by.
pub fn assemble_sessions(events: &[AgentEvent]) -> Vec<AgentSession> {
    if events.is_empty() {
        return Vec::new();
    }

    // Group events by stream_id
    let mut streams: HashMap<StreamId, Vec<AgentEvent>> = HashMap::new();
    for event in events {
        streams
            .entry(event.stream_id.clone())
            .or_default()
            .push(event.clone());
    }

    // First, assemble the main session to build spawn context map
    let main_events = streams.get(&StreamId::Main).cloned().unwrap_or_default();
    let spawn_map = build_spawn_context_map(&main_events);

    // Assemble each stream into a session
    streams
        .into_iter()
        .filter_map(|(stream_id, stream_events)| {
            let spawned_by = match &stream_id {
                StreamId::Sidechain { agent_id } => spawn_map.get(agent_id).cloned(),
                _ => None,
            };
            assemble_session_for_stream(&stream_events, stream_id, spawned_by)
        })
        .collect()
}

/// Build a map from agent_id to SpawnContext by scanning ToolResult events with agent_id.
fn build_spawn_context_map(events: &[AgentEvent]) -> HashMap<String, SpawnContext> {
    let mut spawn_map = HashMap::new();

    // Build turns first to get proper indices
    let turns = build_turns(events);

    for (turn_idx, turn) in turns.iter().enumerate() {
        for (step_idx, step) in turn.steps.iter().enumerate() {
            for tool in &step.tools {
                if let Some(ref result) = tool.result {
                    if let Some(ref agent_id) = result.content.agent_id {
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
    }

    spawn_map
}

/// Assemble the main stream from events into a session.
///
/// This is the backward-compatible function that filters to Main stream only.
/// For multi-stream support, use `assemble_sessions()` instead.
pub fn assemble_session(events: &[AgentEvent]) -> Option<AgentSession> {
    if events.is_empty() {
        return None;
    }

    // Filter to main stream events only
    let main_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.stream_id, StreamId::Main))
        .cloned()
        .collect();

    if main_events.is_empty() {
        return None;
    }

    assemble_session_for_stream(&main_events, StreamId::Main, None)
}

/// Internal: Assemble a session from events belonging to a single stream.
fn assemble_session_for_stream(
    events: &[AgentEvent],
    stream_id: StreamId,
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
        stream_id,
        spawned_by,
        start_time,
        end_time,
        turns,
        stats,
    })
}

fn build_turns(events: &[AgentEvent]) -> Vec<AgentTurn> {
    let mut turns = Vec::new();
    let mut current_turn: Option<TurnBuilder> = None;

    for event in events {
        match &event.payload {
            EventPayload::User(user) => {
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
                    },
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
