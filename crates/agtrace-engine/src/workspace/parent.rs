//! Parent rule for Claude teammates, shared by the live workspace view
//! ([`super::WorkspaceView`]) and the index agent tree (`session show`, MCP
//! `get_agent_tree`), so both views put a teammate under the same agent.
//!
//! **Rule:** a teammate's parent is the agent whose log contains its `AgentSpawn`
//! (e.g. a subagent of the lead session that created the teammate) when that spawn
//! is known; otherwise the team lead from the team config (`leadSessionId`).
//!
//! `leadSessionId` is the lead's *runtime* session id. After a resume or a bg daemon
//! respawn that id names no transcript: the process writes into an existing
//! transcript whose records carry it as `session_id`. The Claude decoder reports
//! those ids as `AgentAttribute(RuntimeSessionId)`; [`RuntimeAliases`] collects them
//! and [`team_lead_agent`] resolves a lead id through them.

use std::collections::BTreeMap;

use agtrace_types::{AgentAttributeKey, AgentEvent, AgentHandle, AgentId, AgentKind, EventPayload};
use chrono::{DateTime, Utc};

/// Runtime session id → transcript agent that wrote records under it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeAliases(BTreeMap<String, AgentId>);

impl RuntimeAliases {
    pub fn new() -> Self {
        Self::default()
    }

    /// Aliases reported in `events` (attributed by `event.agent`).
    pub fn from_events<'a>(events: impl IntoIterator<Item = &'a AgentEvent>) -> Self {
        let mut aliases = Self::new();
        for ev in events {
            aliases.record(ev);
        }
        aliases
    }

    /// Record the alias carried by `ev`, if any. Returns true if it is new or moved.
    pub fn record(&mut self, ev: &AgentEvent) -> bool {
        match &ev.payload {
            EventPayload::AgentAttribute(a) if a.key == AgentAttributeKey::RuntimeSessionId => {
                self.insert(&a.value, &ev.agent)
            }
            _ => false,
        }
    }

    /// Map `runtime_session_id` to `agent`. Returns true if it is new or moved.
    pub fn insert(&mut self, runtime_session_id: &str, agent: &AgentId) -> bool {
        if agent.native_session_id() == runtime_session_id {
            return false;
        }
        self.0
            .insert(runtime_session_id.to_string(), agent.clone())
            .as_ref()
            != Some(agent)
    }

    pub fn get(&self, runtime_session_id: &str) -> Option<&AgentId> {
        self.0.get(runtime_session_id)
    }

    /// Every runtime session id seen.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Runtime session ids that map to `agent`.
    pub fn of<'a>(&'a self, agent: &'a AgentId) -> impl Iterator<Item = &'a str> + 'a {
        self.0
            .iter()
            .filter(move |(_, a)| *a == agent)
            .map(|(sid, _)| sid.as_str())
    }
}

/// The agent of a team lead named by `lead_session_id` (team config
/// `leadSessionId`, possibly a runtime session id). `kind_of` gives the kind of a
/// known agent (None = not known).
///
/// 1. The transcript with that id, when it is known and can lead (not a teammate:
///    a runtime id can coincide with some teammate's transcript id).
/// 2. Else the transcript that wrote records under that runtime id (alias).
/// 3. Else the transcript id itself while it is unknown (it may be discovered
///    later); None when it is known but cannot lead.
pub fn team_lead_agent(
    lead_session_id: &str,
    kind_of: impl Fn(&AgentId) -> Option<AgentKind>,
    aliases: &RuntimeAliases,
) -> Option<AgentId> {
    let direct = AgentId::claude_session(lead_session_id);
    let direct_kind = kind_of(&direct);
    if direct_kind.is_some_and(|k| k != AgentKind::Teammate) {
        return Some(direct);
    }
    if let Some(alias) = aliases.get(lead_session_id) {
        return Some(alias.clone());
    }
    direct_kind.is_none().then_some(direct)
}

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

    #[test]
    fn team_lead_resolves_runtime_session_id_through_aliases() {
        let transcript = AgentId::claude_session("s-transcript");
        let mut aliases = RuntimeAliases::new();
        assert!(aliases.insert("s-runtime", &transcript));
        assert!(!aliases.insert("s-runtime", &transcript));
        // Own id is not an alias.
        assert!(!aliases.insert("s-transcript", &transcript));

        let main = |id: &AgentId| (*id == transcript).then_some(AgentKind::Main);
        let lead = |sid| team_lead_agent(sid, main, &aliases);
        assert_eq!(lead("s-runtime"), Some(transcript.clone()));
        assert_eq!(lead("s-transcript"), Some(transcript.clone()));
        // Unknown and unaliased: the id itself (it may be discovered later).
        assert_eq!(lead("s-none"), Some(AgentId::claude_session("s-none")));

        // A known main transcript with the id wins over an alias ...
        let other = AgentId::claude_session("s-runtime");
        let all_main = |_: &AgentId| Some(AgentKind::Main);
        assert_eq!(
            team_lead_agent("s-runtime", all_main, &aliases),
            Some(other)
        );
        // ... but a teammate transcript with that id never leads: the alias does.
        let all_mates = |_: &AgentId| Some(AgentKind::Teammate);
        assert_eq!(
            team_lead_agent("s-runtime", all_mates, &aliases),
            Some(transcript.clone())
        );
        assert_eq!(team_lead_agent("s-none", all_mates, &aliases), None);
        assert_eq!(
            aliases.of(&transcript).collect::<Vec<_>>(),
            vec!["s-runtime"]
        );
    }
}
