//! `AgentRef` builders.

use std::path::PathBuf;

use agtrace_types::{AgentId, AgentKind, AgentRef};

use super::events::ts;

/// Builder for synthetic [`AgentRef`]s.
///
/// ```
/// use agtrace_testing::synth::AgentBuilder;
/// let lead = AgentBuilder::claude_main("s-lead").started(0).build();
/// let sub = AgentBuilder::claude_subagent("s-lead", "a01").name("explore").build();
/// assert_eq!(sub.parent, Some(lead.id.clone()));
/// ```
#[derive(Debug, Clone)]
pub struct AgentBuilder(AgentRef);

impl AgentBuilder {
    fn base(id: AgentId, native_session_id: &str, file: String) -> AgentRef {
        let mut r = AgentRef::root(id, native_session_id, PathBuf::from(file));
        r.cwd = Some(PathBuf::from("/work/demo-project"));
        r
    }

    /// Claude main transcript `claude:<session>`.
    pub fn claude_main(session: &str) -> Self {
        let id = AgentId::claude_session(session);
        Self(Self::base(
            id,
            session,
            format!("/work/claude/projects/-work-demo-project/{session}.jsonl"),
        ))
    }

    /// Claude Agent Teams member (own top-level transcript, not yet linked).
    pub fn claude_teammate(session: &str, name: &str, team: &str) -> Self {
        let mut b = Self::claude_main(session);
        b.0.kind = AgentKind::Teammate;
        b.0.name = Some(name.to_string());
        b.0.team = Some(team.to_string());
        b.0.native_agent_id = Some(format!("{name}@{team}"));
        b
    }

    /// Claude async subagent `claude:<session>/<aid>` (header links it to the session).
    pub fn claude_subagent(session: &str, aid: &str) -> Self {
        let id = AgentId::claude_subagent(session, aid);
        let parent = AgentId::claude_session(session);
        let mut r = Self::base(
            id,
            session,
            format!(
                "/work/claude/projects/-work-demo-project/{session}/subagents/agent-{aid}.jsonl"
            ),
        );
        r.kind = AgentKind::Subagent;
        r.root = parent.clone();
        r.parent = Some(parent);
        r.native_agent_id = Some(aid.to_string());
        r.depth = 1;
        Self(r)
    }

    /// Claude fork (same layout as a subagent, kind `Fork`).
    pub fn claude_fork(session: &str, aid: &str) -> Self {
        Self::claude_subagent(session, aid).kind(AgentKind::Fork)
    }

    /// Codex root thread `codex:<thread>` with path `/root`.
    pub fn codex_root(thread: &str) -> Self {
        let id = AgentId::codex_thread(thread);
        let mut r = Self::base(
            id,
            thread,
            format!("/work/codex/sessions/2026/09/20/rollout-{thread}.jsonl"),
        );
        r.path = Some("/root".to_string());
        Self(r)
    }

    /// Codex child thread (`thread_spawn`) under `root_thread`, parent `parent_thread`.
    pub fn codex_child(thread: &str, root_thread: &str, parent_thread: &str, path: &str) -> Self {
        let mut b = Self::codex_root(thread);
        b.0.kind = AgentKind::CodexThread;
        b.0.root = AgentId::codex_thread(root_thread);
        b.0.parent = Some(AgentId::codex_thread(parent_thread));
        b.0.path = Some(path.to_string());
        b.0.name = path.rsplit('/').next().map(str::to_string);
        b.0.depth = 1;
        b
    }

    pub fn kind(mut self, kind: AgentKind) -> Self {
        self.0.kind = kind;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.0.name = Some(name.to_string());
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        self.0.path = Some(path.to_string());
        self
    }

    pub fn agent_type(mut self, agent_type: &str) -> Self {
        self.0.agent_type = Some(agent_type.to_string());
        self
    }

    pub fn team(mut self, team: &str) -> Self {
        self.0.team = Some(team.to_string());
        self
    }

    pub fn parent(mut self, parent: &AgentId) -> Self {
        self.0.parent = Some(parent.clone());
        self
    }

    /// Header without parent linkage (e.g. a Codex child whose parent is unknown).
    pub fn no_parent(mut self) -> Self {
        self.0.parent = None;
        self
    }

    pub fn root(mut self, root: &AgentId) -> Self {
        self.0.root = root.clone();
        self
    }

    pub fn spawn_call_id(mut self, id: &str) -> Self {
        self.0.spawn_call_id = Some(id.to_string());
        self
    }

    /// `started_at` = [`ts`]`(secs)`.
    pub fn started(mut self, secs: i64) -> Self {
        self.0.started_at = Some(ts(secs));
        self
    }

    pub fn id(&self) -> AgentId {
        self.0.id.clone()
    }

    pub fn build(self) -> AgentRef {
        self.0
    }
}

impl From<AgentBuilder> for AgentRef {
    fn from(b: AgentBuilder) -> Self {
        b.0
    }
}
