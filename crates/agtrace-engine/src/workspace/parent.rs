//! Parent rule for Claude teammates, shared by the live workspace view
//! ([`super::WorkspaceView`]) and the index agent tree (`session show`, MCP
//! `get_agent_tree`), so both views put a teammate under the same agent.
//!
//! **Rule:** a teammate's parent is the agent whose log contains its `AgentSpawn`
//! (e.g. a subagent of the lead session that created the teammate) when that spawn
//! is known; otherwise the team lead from the team config (`leadSessionId`).

use agtrace_types::{AgentEvent, AgentHandle, AgentId, AgentKind, EventPayload};
use chrono::{DateTime, Utc};

/// The parent of teammate `me`: the spawning agent when known, else the team lead.
/// An agent is never its own parent.
pub fn teammate_parent(
    me: &AgentId,
    spawner: Option<&AgentId>,
    team_lead: Option<&AgentId>,
) -> Option<AgentId> {
    spawner
        .filter(|p| *p != me)
        .or(team_lead.filter(|p| *p != me))
        .cloned()
}

/// A teammate spawn (`AgentSpawn` with a team-member child) found in an agent's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeammateSpawn {
    /// Agent whose log contains the spawn.
    pub spawner: AgentId,
    pub team: Option<String>,
    pub name: String,
    /// Provider call id of the spawning tool call in the spawner's log.
    pub spawn_call_id: Option<String>,
    pub at: DateTime<Utc>,
}

/// All teammate spawns in `events` (events of any agents, attributed by `event.agent`).
pub fn teammate_spawns<'a>(events: impl IntoIterator<Item = &'a AgentEvent>) -> Vec<TeammateSpawn> {
    events
        .into_iter()
        .filter_map(|ev| match &ev.payload {
            EventPayload::AgentSpawn(s) if s.kind == AgentKind::Teammate => match &s.child {
                AgentHandle::TeamMember { team, name } => Some(TeammateSpawn {
                    spawner: ev.agent.clone(),
                    team: team.clone(),
                    name: name.clone(),
                    spawn_call_id: s.spawn_call_id.clone(),
                    at: ev.timestamp,
                }),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// The spawn of teammate `name` (in `team`, when both sides know the team). A name
/// can be reused, so the latest spawn not after the teammate's start wins (the
/// latest overall when the start is unknown).
pub fn find_teammate_spawn<'a>(
    spawns: &'a [TeammateSpawn],
    team: Option<&str>,
    name: &str,
    started_at: Option<DateTime<Utc>>,
) -> Option<&'a TeammateSpawn> {
    let matching = || {
        spawns.iter().filter(move |s| {
            s.name == name
                && match (s.team.as_deref(), team) {
                    (Some(a), Some(b)) => a == b,
                    _ => true,
                }
        })
    };
    started_at
        .and_then(|t| matching().filter(|s| s.at <= t).max_by_key(|s| s.at))
        .or_else(|| matching().max_by_key(|s| s.at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawner_wins_over_team_lead() {
        let me = AgentId::claude_session("s-mate");
        let lead = AgentId::claude_session("s-lead");
        let sub = AgentId::claude_subagent("s-lead", "a01");
        assert_eq!(
            teammate_parent(&me, Some(&sub), Some(&lead)),
            Some(sub.clone())
        );
        assert_eq!(teammate_parent(&me, None, Some(&lead)), Some(lead.clone()));
        assert_eq!(teammate_parent(&me, Some(&me), Some(&lead)), Some(lead));
        assert_eq!(teammate_parent(&me, None, None), None);
    }
}
