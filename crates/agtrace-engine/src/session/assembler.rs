use std::collections::HashMap;

use super::stats::calculate_session_stats;
use super::turn_builder::TurnBuilder;
use super::types::*;
use agtrace_types::{AgentEvent, EventPayload, StreamId};

/// Assemble all streams from events into separate sessions.
///
/// Returns a Vec of AgentSession, one per distinct StreamId found in the events.
/// Each session contains only events from its respective stream.
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

    // Assemble each stream into a session
    streams
        .into_iter()
        .filter_map(|(stream_id, stream_events)| {
            assemble_session_for_stream(&stream_events, stream_id)
        })
        .collect()
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

    assemble_session_for_stream(&main_events, StreamId::Main)
}

/// Internal: Assemble a session from events belonging to a single stream.
fn assemble_session_for_stream(events: &[AgentEvent], stream_id: StreamId) -> Option<AgentSession> {
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
