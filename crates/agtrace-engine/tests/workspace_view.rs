//! Agent graph / live state fold (design §4.3) on synthetic events.

use agtrace_engine::workspace::{
    AgentStatus, CatalogResolver, ContextEvidence, FeedKind, FeedParty, NoWindow, ProcessStatus,
    SideStateUpdate, StatusSource, TeamMember, TimelineItem, WorkspaceEvent, WorkspaceView,
};
use agtrace_testing::synth::{AgentBuilder, EventLog, handle, ts};
use agtrace_types::{
    AgentAttributeKey, AgentEvent, AgentHandle, AgentId, AgentKind, AgentMessageKind, AgentRef,
    AgentSpawnPayload, ContextSource, LifecycleTransition, MessageDirection, ModelCatalog,
    Provider, TurnOutcome,
};
use chrono::{DateTime, Utc};

/// Test harness: a view plus a clock (seconds relative to `ts(0)`).
struct Ws {
    view: WorkspaceView,
    now: DateTime<Utc>,
}

impl Ws {
    fn new() -> Self {
        Self {
            view: WorkspaceView::new(),
            now: ts(0),
        }
    }

    fn apply(&mut self, ev: WorkspaceEvent) {
        self.view.apply(ev, &NoWindow, self.now);
    }

    fn discover(&mut self, r: impl Into<AgentRef>) -> AgentId {
        let r = r.into();
        let id = r.id.clone();
        self.apply(WorkspaceEvent::AgentDiscovered(r));
        id
    }

    fn feed(&mut self, events: Vec<AgentEvent>) {
        let agent = events[0].agent.clone();
        self.apply(WorkspaceEvent::Events {
            agent,
            events,
            reset: false,
        });
    }

    fn one(&mut self, ev: AgentEvent) {
        self.feed(vec![ev]);
    }

    fn side(&mut self, u: SideStateUpdate) {
        self.apply(WorkspaceEvent::SideState(u));
    }

    fn tick(&mut self, secs: i64) -> bool {
        self.now = ts(secs);
        self.view.tick(self.now)
    }

    fn status(&self, id: &AgentId) -> AgentStatus {
        self.view.agents[id].status
    }

    fn source(&self, id: &AgentId) -> StatusSource {
        self.view.agents[id].status_source
    }

    fn parent(&self, id: &AgentId) -> Option<AgentId> {
        self.view.agents[id].agent.parent.clone()
    }

    fn tree_parent(&self, id: &AgentId) -> Option<AgentId> {
        self.view.agents[id].tree_parent.clone()
    }
}

// ------------------------------------------------------------------ graph

#[test]
fn subagent_header_links_under_main_and_roots_are_newest_first() {
    let mut ws = Ws::new();
    let old = ws.discover(AgentBuilder::claude_main("s-old").started(0));
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(100));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01").started(110));

    assert_eq!(ws.view.roots, vec![lead.clone(), old.clone()]);
    assert_eq!(ws.view.agents[&lead].children, vec![sub.clone()]);
    assert_eq!(ws.tree_parent(&sub), Some(lead.clone()));
    let tree: Vec<(String, usize)> = ws
        .view
        .tree()
        .into_iter()
        .map(|(id, d)| (id.to_string(), d))
        .collect();
    assert_eq!(
        tree,
        vec![
            ("claude:s-lead".to_string(), 0),
            ("claude:s-lead/a01".to_string(), 1),
            ("claude:s-old".to_string(), 0),
        ]
    );
}

#[test]
fn spawn_before_child_discovery_links_on_late_discovery() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mut log = EventLog::new(&lead);
    let spawn = log.at(10).spawn_with(AgentSpawnPayload {
        child: handle::member(Some("team-1"), "audit-A"),
        kind: AgentKind::Teammate,
        name: Some("audit-A".into()),
        agent_type: Some("general-purpose".into()),
        requested_model: Some("opus".into()),
        resolved_model: None,
        description: None,
        spawn_call_id: Some("toolu_synthetic_1".into()),
        tool_call_id: None,
    });
    ws.one(spawn);
    assert_eq!(ws.view.pending_links(), 1);
    // The feed shows the spawn with an unresolved child label meanwhile.
    let entry = ws.view.feed.last().unwrap();
    assert_eq!(entry.kind, FeedKind::Spawn(AgentKind::Teammate));
    assert!(matches!(&entry.to[0], FeedParty::Unresolved { label, .. } if label == "audit-A"));

    // Teammate transcript appears later, with no parent in its header.
    let mate =
        ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").started(11));
    assert_eq!(ws.view.pending_links(), 0);
    assert_eq!(ws.parent(&mate), Some(lead.clone()));
    let a = &ws.view.agents[&mate].agent;
    assert_eq!(a.root, lead);
    assert_eq!(a.depth, 1);
    assert_eq!(a.spawn_call_id.as_deref(), Some("toolu_synthetic_1"));
    assert_eq!(ws.view.agents[&mate].model.as_deref(), Some("opus"));
    assert_eq!(ws.view.roots, vec![lead.clone()]);
    // Feed party re-resolved.
    assert_eq!(
        ws.view.feed.last().unwrap().to,
        vec![FeedParty::Agent(mate.clone())]
    );

    // A later header update without parent info keeps the graph link.
    ws.apply(WorkspaceEvent::AgentUpdated(
        AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").build(),
    ));
    assert_eq!(ws.parent(&mate), Some(lead));
}

#[test]
fn teammate_is_linked_by_team_config_without_spawn_event() {
    let mut ws = Ws::new();
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-B", "team-2").started(5));
    assert_eq!(ws.view.roots, vec![mate.clone()]);

    ws.side(SideStateUpdate::ClaudeTeam {
        team: "team-2".into(),
        lead_session_id: "s-lead".into(),
        members: vec![],
    });
    // Lead not discovered yet: linked, but shown top-level until the lead appears.
    assert_eq!(ws.parent(&mate), Some(AgentId::claude_session("s-lead")));
    assert_eq!(ws.tree_parent(&mate), None);

    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    assert_eq!(ws.tree_parent(&mate), Some(lead.clone()));
    assert_eq!(ws.view.roots, vec![lead]);
}

/// Shared parent rule (`teammate_parent`): the spawning agent (here a subagent of
/// the lead session) wins over the team lead, whichever arrives first.
#[test]
fn teammate_parent_is_the_spawning_agent_else_the_team_lead() {
    for config_first in [false, true] {
        let mut ws = Ws::new();
        let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
        let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01").started(1));
        let mate =
            ws.discover(AgentBuilder::claude_teammate("s-mate", "docs", "team-1").started(3));
        let config = SideStateUpdate::ClaudeTeam {
            team: "team-1".into(),
            lead_session_id: "s-lead".into(),
            members: vec![],
        };
        let spawn = EventLog::new(&sub).at(2).spawn(
            handle::member(Some("team-1"), "docs"),
            AgentKind::Teammate,
            Some("docs"),
        );
        if config_first {
            ws.side(config);
            assert_eq!(ws.parent(&mate), Some(lead.clone()), "fallback: team lead");
            ws.one(spawn);
        } else {
            ws.one(spawn);
            ws.side(config);
        }
        assert_eq!(
            ws.parent(&mate),
            Some(sub.clone()),
            "config_first={config_first}"
        );
        assert_eq!(ws.tree_parent(&mate), Some(sub.clone()));
    }
}

#[test]
fn team_lead_handle_resolves_to_team_lead() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").started(1));
    ws.side(SideStateUpdate::ClaudeTeam {
        team: "team-1".into(),
        lead_session_id: "s-lead".into(),
        members: vec![],
    });
    // Teammate sends to "team-lead" without naming the team (context = its own team).
    let mut log = EventLog::new(&mate);
    ws.one(log.at(3).message(
        MessageDirection::Outgoing,
        handle::id(&mate),
        vec![handle::member(None, "team-lead")],
        AgentMessageKind::Message,
        "done",
    ));
    let e = ws.view.feed.last().unwrap();
    assert_eq!(e.from, FeedParty::Agent(mate));
    assert_eq!(e.to, vec![FeedParty::Agent(lead)]);
}

#[test]
fn unlinked_codex_child_is_shown_under_its_root() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root").started(0));
    // Parent thread never discovered (e.g. outside the watch window).
    let child = ws.discover(
        AgentBuilder::codex_child("t-child", "t-root", "t-missing", "/root/a/b").started(5),
    );
    assert_eq!(ws.parent(&child), Some(AgentId::codex_thread("t-missing")));
    assert_eq!(ws.tree_parent(&child), Some(root.clone()));
    assert_eq!(ws.view.agents[&root].children, vec![child]);
    assert_eq!(ws.view.roots, vec![root]);
}

#[test]
fn codex_path_handles_resolve_within_the_same_root() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root").started(0));
    let judge = ws.discover(AgentBuilder::codex_child(
        "t-judge",
        "t-root",
        "t-root",
        "/root/judge",
    ));
    // Another tree with the same path must not match.
    let _other_root = ws.discover(AgentBuilder::codex_root("t-root2").started(1));
    let _other = ws.discover(AgentBuilder::codex_child(
        "t-judge2",
        "t-root2",
        "t-root2",
        "/root/judge",
    ));

    let mut log = EventLog::new(&root);
    ws.one(log.at(2).encrypted_message(
        MessageDirection::Outgoing,
        handle::path("/root"),
        vec![handle::path("/root/judge")],
        AgentMessageKind::NewTask,
    ));
    let e = ws.view.feed.last().unwrap();
    assert_eq!(e.from, FeedParty::Agent(root));
    assert_eq!(e.to, vec![FeedParty::Agent(judge)]);
}

#[test]
fn native_agent_id_resolves_in_session_and_across_resumed_sessions() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let resumed = ws.discover(AgentBuilder::claude_main("s-resumed").started(50));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a07").started(1));

    // Notification in the resumed session refers to a subagent of the original session.
    let mut log = EventLog::new(&resumed);
    ws.one(
        log.at(60)
            .lifecycle(handle::native("a07"), LifecycleTransition::Completed),
    );
    assert_eq!(ws.status(&sub), AgentStatus::Done);
    let _ = lead;
}

#[test]
fn spawn_cycles_do_not_hang_or_hide_agents() {
    let mut ws = Ws::new();
    let a = ws.discover(AgentBuilder::claude_main("s-a").started(0));
    let b = ws.discover(AgentBuilder::claude_main("s-b").started(1));
    let mut la = EventLog::new(&a);
    let mut lb = EventLog::new(&b);
    ws.one(la.spawn(handle::id(&b), AgentKind::Subagent, None));
    ws.one(lb.spawn(handle::id(&a), AgentKind::Subagent, None));
    // Second link would create a cycle and is ignored.
    assert_eq!(ws.parent(&b), Some(a.clone()));
    assert_eq!(ws.parent(&a), None);
    assert_eq!(ws.view.tree().len(), 2);
}

#[test]
fn events_before_discovery_create_a_placeholder_that_discovery_completes() {
    let mut ws = Ws::new();
    let sub_id = AgentId::claude_subagent("s-lead", "a09");
    let mut log = EventLog::new(&sub_id);
    ws.one(log.at(5).user("task"));
    let v = &ws.view.agents[&sub_id];
    assert!(!v.discovered);
    assert_eq!(v.agent.kind, AgentKind::Subagent);
    assert_eq!(v.status, AgentStatus::Running);

    ws.discover(AgentBuilder::claude_subagent("s-lead", "a09").name("explore"));
    let v = &ws.view.agents[&sub_id];
    assert!(v.discovered);
    assert_eq!(v.label(), "explore");
    // started_at filled from the first event.
    assert_eq!(v.agent.started_at, Some(ts(5)));
    assert_eq!(v.recent.len(), 1);
}

// ------------------------------------------------------------------ status

#[test]
fn claude_main_own_log_turns() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    assert_eq!(ws.status(&lead), AgentStatus::Unknown);
    let mut log = EventLog::new(&lead);
    ws.one(log.at(1).user("go"));
    assert_eq!(ws.status(&lead), AgentStatus::Running);
    assert_eq!(ws.source(&lead), StatusSource::OwnLog);
    ws.one(log.at(2).turn_end());
    assert_eq!(ws.status(&lead), AgentStatus::Idle);
    // Metadata after the turn does not wake it up.
    ws.one(log.at(3).attribute(AgentAttributeKey::Title, "demo"));
    ws.one(log.at(3).notification("away_summary", "summary"));
    assert_eq!(ws.status(&lead), AgentStatus::Idle);
    ws.one(log.at(4).user("more"));
    assert_eq!(ws.status(&lead), AgentStatus::Running);
    ws.one(log.at(5).turn_end_with(TurnOutcome::Interrupted));
    assert_eq!(ws.status(&lead), AgentStatus::Idle);
}

#[test]
fn registry_wins_while_alive_and_dead_pid_means_done_until_resumed() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let mut log = EventLog::new(&lead);
    ws.one(log.at(1).user("go"));
    let proc = |alive, status, at| SideStateUpdate::ClaudeProcess {
        session_id: "s-lead".into(),
        pid: 4242,
        alive,
        status,
        name: Some("demo".into()),
        updated_at: ts(at),
    };
    ws.side(proc(true, Some(ProcessStatus::Idle), 2));
    assert_eq!(ws.status(&lead), AgentStatus::Idle);
    assert_eq!(ws.source(&lead), StatusSource::Registry);
    assert_eq!(ws.view.registry_name(&lead), Some("demo"));

    ws.side(proc(true, Some(ProcessStatus::Busy), 3));
    assert_eq!(ws.status(&lead), AgentStatus::Running);
    // Alive registry suppresses staleness.
    assert!(!ws.tick(10_000));
    assert_eq!(ws.status(&lead), AgentStatus::Running);

    // An out-of-date report for the same pid is ignored.
    ws.side(proc(true, Some(ProcessStatus::Idle), 2));
    assert_eq!(ws.status(&lead), AgentStatus::Running);

    ws.side(proc(false, None, 20));
    assert_eq!(ws.status(&lead), AgentStatus::Done);
    assert_eq!(ws.source(&lead), StatusSource::Registry);

    // Resumed (new activity after the pid died): the own log decides again.
    ws.now = ts(30);
    ws.one(log.at(30).user("resume"));
    assert_eq!(ws.status(&lead), AgentStatus::Running);
    assert_eq!(ws.source(&lead), StatusSource::OwnLog);
}

#[test]
fn parent_side_terminal_is_sticky_regardless_of_file_order() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    let mut lead_log = EventLog::new(&lead);
    let mut sub_log = EventLog::new(&sub);

    // Parent file attached first: completion at t=10.
    ws.one(
        lead_log
            .at(10)
            .lifecycle(handle::native("a01"), LifecycleTransition::Completed),
    );
    assert_eq!(ws.status(&sub), AgentStatus::Done);
    assert_eq!(ws.source(&sub), StatusSource::ParentEvent);
    // Child file attached afterwards: its (older) records do not revive it.
    ws.feed(vec![
        sub_log.at(1).user("task"),
        sub_log.at(9).assistant("result"),
    ]);
    assert_eq!(ws.status(&sub), AgentStatus::Done);
    // New own activity after the terminal does.
    ws.one(sub_log.at(11).user("follow-up"));
    assert_eq!(ws.status(&sub), AgentStatus::Running);
}

#[test]
fn incoming_handback_marks_the_sender_done() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    ws.one(EventLog::new(&sub).at(1).user("task"));
    let mut log = EventLog::new(&lead);
    ws.one(log.at(5).message(
        MessageDirection::Incoming,
        handle::native("a01"),
        vec![handle::id(&lead)],
        AgentMessageKind::Handback,
        "report",
    ));
    assert_eq!(ws.status(&sub), AgentStatus::Done);
}

#[test]
fn teammate_lifecycle_from_lead_log() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1"));
    let mut mate_log = EventLog::new(&mate);
    let mut lead_log = EventLog::new(&lead);
    ws.one(mate_log.at(1).user("task"));
    assert_eq!(ws.status(&mate), AgentStatus::Running);

    // idle_notification(available) attributed to the sender: priority 2, not terminal.
    ws.one(lead_log.at(2).lifecycle(
        handle::member(Some("team-1"), "audit-A"),
        LifecycleTransition::Idle,
    ));
    assert_eq!(ws.status(&mate), AgentStatus::Idle);
    assert_eq!(ws.source(&mate), StatusSource::ParentEvent);
    // idle_notification(failed) ⇒ Failed, still overridable by own activity.
    ws.one(lead_log.at(3).lifecycle(
        handle::member(Some("team-1"), "audit-A"),
        LifecycleTransition::Failed,
    ));
    assert_eq!(ws.status(&mate), AgentStatus::Failed);
    ws.one(mate_log.at(4).user("retry"));
    assert_eq!(ws.status(&mate), AgentStatus::Running);

    // TaskStop ⇒ Killed (terminal).
    ws.one(
        lead_log
            .at(5)
            .lifecycle(handle::member(None, "audit-A"), LifecycleTransition::Killed),
    );
    assert_eq!(ws.status(&mate), AgentStatus::Killed);
    // A trailing turn end in its own log is not "new activity".
    ws.one(mate_log.at(6).turn_end_with(TurnOutcome::Interrupted));
    assert_eq!(ws.status(&mate), AgentStatus::Killed);
}

#[test]
fn all_background_killed_kills_running_children_only() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let running = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    let done = ws.discover(AgentBuilder::claude_subagent("s-lead", "a02"));
    let mut lead_log = EventLog::new(&lead);

    // Parent log first (order independence): kill at t=10.
    ws.one(
        lead_log
            .at(5)
            .lifecycle(handle::native("a02"), LifecycleTransition::Completed),
    );
    ws.one(lead_log.at(10).lifecycle(
        AgentHandle::Unknown("all".into()),
        LifecycleTransition::AllBackgroundKilled,
    ));
    ws.one(EventLog::new(&running).at(8).user("task"));
    ws.one(EventLog::new(&done).at(2).user("task"));
    assert_eq!(ws.status(&running), AgentStatus::Killed);
    assert_eq!(ws.status(&done), AgentStatus::Done);
    let e = ws.view.feed.last().unwrap();
    assert_eq!(
        e.kind,
        FeedKind::Lifecycle(LifecycleTransition::AllBackgroundKilled)
    );
    assert_eq!(e.from, FeedParty::Agent(lead));
}

#[test]
fn team_config_is_active_false_kills_and_true_revives() {
    let mut ws = Ws::new();
    ws.discover(AgentBuilder::claude_main("s-lead"));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-C", "team-3"));
    ws.one(EventLog::new(&mate).at(0).user("task"));
    let team = |active| SideStateUpdate::ClaudeTeam {
        team: "team-3".into(),
        lead_session_id: "s-lead".into(),
        members: vec![
            TeamMember {
                name: "team-lead".into(),
                agent_type: Some("team-lead".into()),
                model: None,
                is_active: None,
            },
            TeamMember {
                name: "audit-C".into(),
                agent_type: Some("general-purpose".into()),
                model: Some("claude-opus-5-5[1m]".into()),
                is_active: Some(active),
            },
        ],
    };
    ws.now = ts(5);
    ws.side(team(false));
    assert_eq!(ws.status(&mate), AgentStatus::Killed);
    // Team member model feeds the context evidence (external [1m] marker) and model.
    let v = &ws.view.agents[&mate];
    assert_eq!(v.context.external_marker, Some(1_000_000));
    assert_eq!(v.model.as_deref(), Some("claude-opus-5-5[1m]"));
    // Re-applying the same config does not move the terminal time.
    ws.now = ts(50);
    ws.side(team(false));
    ws.one(EventLog::new(&mate).at(10).user("after"));
    assert_eq!(ws.status(&mate), AgentStatus::Running);

    ws.side(team(false));
    ws.side(team(true));
    assert_eq!(ws.status(&mate), AgentStatus::Running);
}

#[test]
fn subagent_meta_stopped_by_user_is_killed() {
    let mut ws = Ws::new();
    ws.discover(AgentBuilder::claude_main("s-lead"));
    let sub_id = AgentId::claude_subagent("s-lead", "a03");
    // Meta read before the subagent file is tracked.
    ws.side(SideStateUpdate::ClaudeSubagentMeta {
        agent: sub_id.clone(),
        stopped_by_user: true,
        model: None,
    });
    ws.discover(AgentBuilder::claude_subagent("s-lead", "a03"));
    ws.one(EventLog::new(&sub_id).at(0).user("task"));
    assert_eq!(ws.status(&sub_id), AgentStatus::Killed);
}

#[test]
fn codex_thread_status_rules() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let child = ws.discover(AgentBuilder::codex_child(
        "t-child",
        "t-root",
        "t-root",
        "/root/judge",
    ));
    let mut child_log = EventLog::new(&child);
    let mut root_log = EventLog::new(&root);

    // task_started ⇒ own lifecycle Running.
    ws.one(
        child_log
            .at(1)
            .lifecycle(handle::id(&child), LifecycleTransition::Running),
    );
    assert_eq!(ws.status(&child), AgentStatus::Running);
    assert_eq!(ws.source(&child), StatusSource::OwnLog);
    // task_complete with error ⇒ Failed.
    ws.one(child_log.at(2).turn_end_with(TurnOutcome::Failed {
        error: Some("boom".into()),
    }));
    assert_eq!(ws.status(&child), AgentStatus::Failed);
    // Parent SubAgentActivity interacted ⇒ Running; interrupted ⇒ Idle.
    ws.one(
        root_log
            .at(3)
            .lifecycle(handle::id(&child), LifecycleTransition::Running),
    );
    assert_eq!(ws.status(&child), AgentStatus::Running);
    ws.one(
        root_log
            .at(4)
            .lifecycle(handle::id(&child), LifecycleTransition::Interrupted),
    );
    assert_eq!(ws.status(&child), AgentStatus::Idle);
    // completed (FINAL_ANSWER) ⇒ Done, re-tasked later ⇒ Running again.
    ws.one(
        root_log
            .at(5)
            .lifecycle(handle::id(&child), LifecycleTransition::Completed),
    );
    assert_eq!(ws.status(&child), AgentStatus::Done);
    ws.one(
        child_log
            .at(6)
            .lifecycle(handle::id(&child), LifecycleTransition::Running),
    );
    assert_eq!(ws.status(&child), AgentStatus::Running);
}

// ------------------------------------------------------------------ staleness

#[test]
fn claude_main_staleness_without_registry() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    ws.one(EventLog::new(&lead).at(0).user("go"));
    assert!(!ws.tick(9 * 60));
    assert_eq!(ws.status(&lead), AgentStatus::Running);
    assert!(ws.tick(10 * 60));
    assert_eq!(ws.status(&lead), AgentStatus::Idle);
    assert_eq!(ws.source(&lead), StatusSource::Staleness);
    assert!(ws.tick(2 * 3600));
    assert_eq!(ws.status(&lead), AgentStatus::Done);
}

#[test]
fn subagent_staleness_depends_on_parent() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    let mut lead_log = EventLog::new(&lead);
    ws.one(lead_log.at(0).user("go"));
    ws.one(EventLog::new(&sub).at(0).user("task"));
    ws.side(SideStateUpdate::ClaudeProcess {
        session_id: "s-lead".into(),
        pid: 1,
        alive: true,
        status: Some(ProcessStatus::Busy),
        name: None,
        updated_at: ts(0),
    });
    // Parent still running: a silent subagent stays Running.
    ws.tick(20 * 60);
    assert_eq!(ws.status(&sub), AgentStatus::Running);
    // Parent process gone ⇒ parent Done ⇒ stale subagent Done.
    ws.side(SideStateUpdate::ClaudeProcess {
        session_id: "s-lead".into(),
        pid: 1,
        alive: false,
        status: None,
        name: None,
        updated_at: ts(20 * 60),
    });
    assert_eq!(ws.status(&lead), AgentStatus::Done);
    assert_eq!(ws.status(&sub), AgentStatus::Done);
    assert_eq!(ws.source(&sub), StatusSource::Staleness);
}

#[test]
fn orphan_subagent_goes_done_after_ten_minutes() {
    let mut ws = Ws::new();
    // Parent file not tracked ("gone").
    let sub = ws.discover(AgentBuilder::claude_subagent("s-gone", "a01"));
    ws.one(EventLog::new(&sub).at(0).user("task"));
    ws.tick(5 * 60);
    assert_eq!(ws.status(&sub), AgentStatus::Running);
    ws.tick(10 * 60);
    assert_eq!(ws.status(&sub), AgentStatus::Done);
}

#[test]
fn codex_staleness() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let child = ws.discover(AgentBuilder::codex_child(
        "t-child", "t-root", "t-root", "/root/w",
    ));
    ws.one(EventLog::new(&root).at(0).user("go"));
    ws.one(EventLog::new(&child).at(0).user("task"));
    ws.tick(29 * 60);
    assert_eq!(ws.status(&child), AgentStatus::Running);
    ws.tick(30 * 60);
    assert_eq!(ws.status(&child), AgentStatus::Idle);
    assert_eq!(ws.status(&root), AgentStatus::Idle);
    ws.tick(2 * 3600);
    assert_eq!(ws.status(&root), AgentStatus::Done);
    // Children never go Done by staleness.
    assert_eq!(ws.status(&child), AgentStatus::Idle);
}

// ------------------------------------------------------------------ feed

#[test]
fn feed_merges_both_sides_of_a_message_preferring_plaintext() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let child = ws.discover(AgentBuilder::codex_child(
        "t-child",
        "t-root",
        "t-root",
        "/root/judge",
    ));
    let mut root_log = EventLog::new(&root);
    let mut child_log = EventLog::new(&child);

    // Sender side: encrypted.
    ws.one(root_log.at(10).encrypted_message(
        MessageDirection::Outgoing,
        handle::path("/root/judge"),
        vec![handle::path("/root")],
        AgentMessageKind::FinalAnswer,
    ));
    // Recipient side within 2 s: plaintext.
    ws.one(child_log.at(11).message(
        MessageDirection::Incoming,
        handle::path("/root/judge"),
        vec![handle::path("/root")],
        AgentMessageKind::FinalAnswer,
        "verdict: ok",
    ));
    let msgs: Vec<_> = ws
        .view
        .feed
        .iter()
        .filter(|e| matches!(e.kind, FeedKind::Message(_)))
        .collect();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].text.as_deref(), Some("verdict: ok"));
    assert!(!msgs[0].encrypted);

    // Same message, other side, 3 s later: a separate entry.
    ws.one(child_log.at(14).message(
        MessageDirection::Incoming,
        handle::path("/root/judge"),
        vec![handle::path("/root")],
        AgentMessageKind::FinalAnswer,
        "verdict: ok",
    ));
    // Two messages in the same log at the same second: both kept.
    ws.one(root_log.at(20).message(
        MessageDirection::Outgoing,
        handle::path("/root"),
        vec![handle::path("/root/judge")],
        AgentMessageKind::Message,
        "one",
    ));
    ws.one(root_log.at(20).message(
        MessageDirection::Outgoing,
        handle::path("/root"),
        vec![handle::path("/root/judge")],
        AgentMessageKind::Message,
        "two",
    ));
    assert_eq!(ws.view.feed.len(), 4);
}

#[test]
fn feed_dedupe_survives_late_discovery_and_replays() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mut lead_log = EventLog::new(&lead);
    ws.one(lead_log.at(0).spawn(
        handle::member(Some("team-1"), "audit-A"),
        AgentKind::Teammate,
        Some("audit-A"),
    ));
    // Outgoing SendMessage to the teammate before its transcript is discovered.
    let out = lead_log.at(10).message(
        MessageDirection::Outgoing,
        handle::id(&lead),
        vec![handle::member(Some("team-1"), "audit-A")],
        AgentMessageKind::Message,
        "please review",
    );
    ws.one(out.clone());
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").started(1));
    // Recipient side: "team-lead" resolves through the team's spawner.
    ws.one(EventLog::new(&mate).at(11).message(
        MessageDirection::Incoming,
        handle::member(Some("team-1"), "team-lead"),
        vec![handle::id(&mate)],
        AgentMessageKind::Message,
        "please review",
    ));
    // Replaying the original event (e.g. after a reset) adds nothing.
    ws.one(out);
    let msgs: Vec<_> = ws
        .view
        .feed
        .iter()
        .filter(|e| matches!(e.kind, FeedKind::Message(_)))
        .collect();
    assert_eq!(msgs.len(), 1, "{msgs:#?}");
    assert_eq!(msgs[0].from, FeedParty::Agent(lead));
    assert_eq!(msgs[0].to, vec![FeedParty::Agent(mate)]);
}

fn messages(ws: &Ws) -> Vec<&agtrace_engine::workspace::FeedEntry> {
    ws.view
        .feed
        .iter()
        .filter(|e| matches!(e.kind, FeedKind::Message(_)))
        .collect()
}

fn lifecycles(ws: &Ws) -> Vec<&agtrace_engine::workspace::FeedEntry> {
    ws.view
        .feed
        .iter()
        .filter(|e| matches!(e.kind, FeedKind::Lifecycle(_)))
        .collect()
}

/// A Claude teammate reads queued messages at its next turn: the incoming copy is
/// logged seconds to minutes after the sender's `SendMessage`. Both copies are one
/// feed entry, in both directions, whichever file is read first.
#[test]
fn feed_pairs_teammate_message_copies_across_delivery_delay() {
    for recipient_first in [false, true] {
        let mut ws = Ws::new();
        let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
        let mate =
            ws.discover(AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").started(1));
        ws.side(SideStateUpdate::ClaudeTeam {
            team: "team-1".into(),
            lead_session_id: "s-lead".into(),
            members: vec![],
        });
        let mut lead_log = EventLog::new(&lead);
        let mut mate_log = EventLog::new(&mate);
        let sent = lead_log.at(11).message(
            MessageDirection::Outgoing,
            handle::id(&lead),
            vec![handle::member(Some("team-1"), "audit-A")],
            AgentMessageKind::Message,
            "Please focus on error handling.",
        );
        let read = mate_log.at(21).message(
            MessageDirection::Incoming,
            handle::member(Some("team-1"), "team-lead"),
            vec![handle::id(&mate)],
            AgentMessageKind::Message,
            "Please focus on error handling.",
        );
        let reply = mate_log.at(26).message(
            MessageDirection::Outgoing,
            handle::id(&mate),
            vec![handle::member(None, "team-lead")],
            AgentMessageKind::Message,
            "Done, found 3 bugs.",
        );
        let reply_read = lead_log.at(90).message(
            MessageDirection::Incoming,
            handle::member(Some("team-1"), "audit-A"),
            vec![handle::id(&lead)],
            AgentMessageKind::Message,
            "Done, found 3 bugs.",
        );
        if recipient_first {
            ws.feed(vec![read, reply]);
            ws.feed(vec![sent, reply_read]);
        } else {
            ws.feed(vec![sent, reply_read]);
            ws.feed(vec![read, reply]);
        }
        let msgs = messages(&ws);
        assert_eq!(
            msgs.len(),
            2,
            "recipient_first={recipient_first}: {msgs:#?}"
        );
        assert_eq!(msgs[0].from, FeedParty::Agent(lead.clone()));
        assert_eq!(msgs[0].to, vec![FeedParty::Agent(mate.clone())]);
        assert_eq!(msgs[1].from, FeedParty::Agent(mate.clone()));
        assert_eq!(msgs[1].to, vec![FeedParty::Agent(lead.clone())]);
        assert!(msgs.iter().all(|m| m.merged.len() == 1));
    }
}

/// Repeated identical messages pair one-to-one (oldest unpaired copy first), and a
/// different body never pairs.
#[test]
fn feed_pairs_repeated_messages_one_to_one() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let child = ws.discover(AgentBuilder::codex_child(
        "t-c",
        "t-root",
        "t-root",
        "/root/judge",
    ));
    let mut root_log = EventLog::new(&root);
    let mut child_log = EventLog::new(&child);
    let send = |log: &mut EventLog, at: i64| {
        log.at(at).encrypted_message(
            MessageDirection::Outgoing,
            handle::path("/root"),
            vec![handle::path("/root/judge")],
            AgentMessageKind::Message,
        )
    };
    let recv = |log: &mut EventLog, at: i64, body: &str| {
        log.at(at).message(
            MessageDirection::Incoming,
            handle::path("/root"),
            vec![handle::path("/root/judge")],
            AgentMessageKind::Message,
            body,
        )
    };
    ws.feed(vec![send(&mut root_log, 10), send(&mut root_log, 20)]);
    ws.feed(vec![
        recv(&mut child_log, 30, "first"),
        recv(&mut child_log, 40, "second"),
    ]);
    let msgs = messages(&ws);
    assert_eq!(msgs.len(), 2, "{msgs:#?}");
    assert_eq!(msgs[0].text.as_deref(), Some("first"));
    assert_eq!(msgs[1].text.as_deref(), Some("second"));

    // Plaintext bodies that differ are different messages.
    let mut ws = Ws::new();
    let a = ws.discover(AgentBuilder::claude_main("s-a"));
    let b = ws.discover(AgentBuilder::claude_teammate("s-b", "bee", "team-1"));
    ws.one(EventLog::new(&a).at(1).message(
        MessageDirection::Outgoing,
        handle::id(&a),
        vec![handle::id(&b)],
        AgentMessageKind::Message,
        "one",
    ));
    ws.one(EventLog::new(&b).at(2).message(
        MessageDirection::Incoming,
        handle::id(&a),
        vec![handle::id(&b)],
        AgentMessageKind::Message,
        "two",
    ));
    assert_eq!(messages(&ws).len(), 2);
}

/// A forked Codex child logs its NEW_TASK (after the copied parent prefix) when it
/// starts, well after the parent's send: one entry, plaintext side kept.
#[test]
fn feed_pairs_codex_fork_new_task_with_the_parents_send() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let fork = ws.discover(
        AgentBuilder::codex_child("t-fork", "t-root", "t-root", "/root/forker")
            .kind(AgentKind::Fork),
    );
    ws.one(EventLog::new(&root).at(14).encrypted_message(
        MessageDirection::Outgoing,
        handle::path("/root"),
        vec![handle::path("/root/forker")],
        AgentMessageKind::NewTask,
    ));
    ws.one(EventLog::new(&fork).at(25).message(
        MessageDirection::Incoming,
        handle::path("/root"),
        vec![handle::path("/root/forker")],
        AgentMessageKind::NewTask,
        "analyse the fork",
    ));
    let msgs = messages(&ws);
    assert_eq!(msgs.len(), 1, "{msgs:#?}");
    assert_eq!(msgs[0].text.as_deref(), Some("analyse the fork"));
    assert!(!msgs[0].encrypted);
    assert_eq!(msgs[0].to, vec![FeedParty::Agent(fork)]);
}

/// The first copy was pushed with an unresolved recipient, the second copy arrived
/// from a not-yet-discovered agent (different label): once discovery resolves the
/// handle, the two entries are merged.
#[test]
fn feed_merges_copies_once_a_late_handle_resolves() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root"));
    let child = AgentBuilder::codex_child("t-late", "t-root", "t-root", "/root/late");
    let child_id = child.id();
    ws.one(EventLog::new(&root).at(10).encrypted_message(
        MessageDirection::Outgoing,
        handle::path("/root"),
        vec![handle::path("/root/late")],
        AgentMessageKind::NewTask,
    ));
    // Child events before its header: a placeholder without a path.
    ws.one(EventLog::new(&child_id).at(12).message(
        MessageDirection::Incoming,
        handle::id(&root),
        vec![handle::id(&child_id)],
        AgentMessageKind::NewTask,
        "late task",
    ));
    assert_eq!(messages(&ws).len(), 2, "not resolvable yet");
    ws.discover(child);
    let msgs = messages(&ws);
    assert_eq!(msgs.len(), 1, "{msgs:#?}");
    assert_eq!(msgs[0].text.as_deref(), Some("late task"));
    assert_eq!(msgs[0].to, vec![FeedParty::Agent(child_id.clone())]);
    // A replay of either copy adds nothing.
    ws.one(EventLog::new(&child_id).at(12).message(
        MessageDirection::Incoming,
        handle::id(&root),
        vec![handle::id(&child_id)],
        AgentMessageKind::NewTask,
        "late task",
    ));
    assert_eq!(messages(&ws).len(), 1);
}

/// A Claude subagent's completion is reported twice by the parent log: the
/// `SubagentHandback` result and, seconds later, the task-notification. One "done"
/// entry, unless the agent was addressed (resumed) in between.
#[test]
fn feed_merges_repeated_lifecycle_reports() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let sub = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    let mut log = EventLog::new(&lead);
    ws.one(
        log.at(5)
            .lifecycle(handle::native("a01"), LifecycleTransition::Completed),
    );
    ws.one(
        log.at(9)
            .lifecycle(handle::native("a01"), LifecycleTransition::Completed),
    );
    assert_eq!(lifecycles(&ws).len(), 1, "{:#?}", lifecycles(&ws));

    // Resumed with SendMessage, completes again: a new entry.
    ws.one(log.at(20).message(
        MessageDirection::Outgoing,
        handle::id(&lead),
        vec![handle::native("a01")],
        AgentMessageKind::Message,
        "part 2",
    ));
    ws.one(
        log.at(30)
            .lifecycle(handle::native("a01"), LifecycleTransition::Completed),
    );
    let l = lifecycles(&ws);
    assert_eq!(l.len(), 2, "{l:#?}");
    assert!(l.iter().all(|e| e.from == FeedParty::Agent(sub.clone())));
}

#[test]
fn feed_matches_unresolved_parties_by_label() {
    let mut ws = Ws::new();
    let a = ws.discover(AgentBuilder::claude_main("s-a"));
    let b = ws.discover(AgentBuilder::claude_main("s-b"));
    let peer = || AgentHandle::Unknown("peer-x".into());
    ws.one(EventLog::new(&a).at(1).encrypted_message(
        MessageDirection::Outgoing,
        peer(),
        vec![AgentHandle::User],
        AgentMessageKind::Peer,
    ));
    ws.one(EventLog::new(&b).at(2).message(
        MessageDirection::Incoming,
        peer(),
        vec![AgentHandle::User],
        AgentMessageKind::Peer,
        "hello",
    ));
    assert_eq!(ws.view.feed.len(), 1);
    let e = ws.view.feed.last().unwrap();
    assert_eq!(e.text.as_deref(), Some("hello"));
    assert_eq!(ws.view.party_label(&e.from), "peer-x");
    assert_eq!(e.to, vec![FeedParty::User]);
}

#[test]
fn feed_is_ordered_by_timestamp_across_files() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let a = ws.discover(AgentBuilder::claude_subagent("s-lead", "a01"));
    let b = ws.discover(AgentBuilder::claude_subagent("s-lead", "a02"));
    let mut log = EventLog::new(&lead);
    ws.one(
        log.at(30)
            .lifecycle(handle::native("a02"), LifecycleTransition::Completed),
    );
    ws.one(
        log.at(10)
            .lifecycle(handle::native("a01"), LifecycleTransition::Failed),
    );
    let order: Vec<_> = ws.view.feed.iter().map(|e| e.from.clone()).collect();
    assert_eq!(order, vec![FeedParty::Agent(a), FeedParty::Agent(b)]);
}

// ------------------------------------------------------------------ timeline / tools / context

#[test]
fn running_tool_timeline_and_reset() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let mut log = EventLog::new(&lead);
    let call = log.at(1).bash("mise run test\n--verbose");
    ws.one(call.clone());
    let v = &ws.view.agents[&lead];
    let tool = v.current_tool.as_ref().unwrap();
    assert_eq!(tool.name, "Bash");
    assert_eq!(tool.summary, "mise run test");
    assert_eq!(tool.since, ts(1));
    ws.one(log.at(2).tool_result(call.id, "ok", false));
    assert!(ws.view.agents[&lead].current_tool.is_none());
    let failing = log.at(3).bash("false");
    ws.one(failing.clone());
    ws.one(log.at(4).tool_result(failing.id, "exit 1", true));
    ws.one(log.at(5).usage(1000, Some("claude-opus-5-5")));

    let items: Vec<_> = ws.view.agents[&lead]
        .recent
        .iter()
        .map(|e| e.item.clone())
        .collect();
    assert_eq!(
        items.len(),
        3,
        "success result and usage are not timeline rows"
    );
    assert!(matches!(&items[2], TimelineItem::ToolError { preview } if preview == "exit 1"));
    assert_eq!(
        ws.view.agents[&lead].model.as_deref(),
        Some("claude-opus-5-5")
    );

    ws.apply(WorkspaceEvent::Events {
        agent: lead.clone(),
        events: vec![EventLog::new(&lead).at(0).user("again")],
        reset: true,
    });
    let v = &ws.view.agents[&lead];
    assert_eq!(v.recent.len(), 1);
    assert_eq!(v.context, ContextEvidence::default());
    assert!(v.model.is_none());
}

/// Catalog with only a built-in table entry for `claude-opus-5-5` (200k, to make the
/// `[1m]` marker observable) and nothing else.
struct TestCatalog;

impl ModelCatalog for TestCatalog {
    fn user_override(&self, _: Provider, _: Option<&str>) -> Option<u64> {
        None
    }
    fn provider_cache(&self, _: Provider, _: &str) -> Option<u64> {
        None
    }
    fn table(&self, _: Provider, model: &str) -> Option<u64> {
        model.starts_with("claude-opus-5-5").then_some(200_000)
    }
}

#[test]
fn window_is_resolved_through_the_catalog_on_evidence_change() {
    let resolver = CatalogResolver(&TestCatalog);
    let mut view = WorkspaceView::new();
    let root = AgentBuilder::codex_root("t-root").build();
    let id = root.id.clone();
    view.apply(WorkspaceEvent::AgentDiscovered(root), &resolver, ts(0));
    assert!(view.agents[&id].window.is_none());
    let mut log = EventLog::new(&id);
    view.apply(
        WorkspaceEvent::Events {
            agent: id.clone(),
            events: vec![
                log.model_change(None, "gpt-6-astra"),
                log.window_hint(258_400),
                log.usage(1234, None),
            ],
            reset: false,
        },
        &resolver,
        ts(0),
    );
    let v = &view.agents[&id];
    let w = v.window.as_ref().unwrap();
    assert_eq!(w.tokens, 258_400);
    assert_eq!(w.source, ContextSource::Log);
    assert_eq!(w.model.as_deref(), Some("gpt-6-astra"));
    assert_eq!(v.context.last_context_tokens, 1234);
    assert_eq!(v.model.as_deref(), Some("gpt-6-astra"));

    // Usage beyond the window: the resolver's observed floor applies on the next change.
    view.apply(
        WorkspaceEvent::Events {
            agent: id.clone(),
            events: vec![log.usage(300_000, None)],
            reset: false,
        },
        &resolver,
        ts(0),
    );
    let w = view.agents[&id].window.as_ref().unwrap();
    assert_eq!(w.source, ContextSource::Observed);
    assert_eq!(w.overruled, Some(ContextSource::Log));
}

#[test]
fn team_config_1m_model_is_an_external_marker_for_the_resolver() {
    let resolver = CatalogResolver(&TestCatalog);
    let mut view = WorkspaceView::new();
    let mate = AgentBuilder::claude_teammate("s-mate", "audit-A", "team-1").build();
    let id = mate.id.clone();
    view.apply(WorkspaceEvent::AgentDiscovered(mate), &resolver, ts(0));
    let mut log = EventLog::new(&id);
    view.apply(
        WorkspaceEvent::Events {
            agent: id.clone(),
            events: vec![log.usage(1000, Some("claude-opus-5-5"))],
            reset: false,
        },
        &resolver,
        ts(0),
    );
    let w = view.agents[&id].window.clone().unwrap();
    assert_eq!((w.tokens, w.source), (200_000, ContextSource::ModelTable));

    view.apply(
        WorkspaceEvent::SideState(SideStateUpdate::ClaudeTeam {
            team: "team-1".into(),
            lead_session_id: "s-lead".into(),
            members: vec![TeamMember {
                name: "audit-A".into(),
                agent_type: None,
                model: Some("claude-opus-5-5[1m]".into()),
                is_active: Some(true),
            }],
        }),
        &resolver,
        ts(0),
    );
    let v = &view.agents[&id];
    assert_eq!(v.context.external_marker, Some(1_000_000));
    let w = v.window.clone().unwrap();
    assert_eq!(
        (w.tokens, w.source),
        (1_000_000, ContextSource::ModelMarker)
    );
}

#[test]
fn diagnostics_and_errors_are_kept() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead"));
    let mut d = agtrace_types::ParseDiagnostics::default();
    d.record_invalid_json(3, 30, "bad");
    ws.apply(WorkspaceEvent::Diagnostics {
        agent: lead.clone(),
        diagnostics: d,
    });
    ws.apply(WorkspaceEvent::Error("permission denied".into()));
    assert_eq!(ws.view.total_diagnostic_errors(), 1);
    assert_eq!(
        ws.view.errors.last().map(String::as_str),
        Some("permission denied")
    );
}

// ------------------------------------------------------------------ resume / runtime ids

/// A resumed / bg-respawned lead: the team config names the lead's *runtime*
/// session id, which has no transcript of its own. The lead transcript reports it
/// as a `RuntimeSessionId` alias, and the teammate links to the transcript —
/// whichever of config and alias arrives first.
///
/// The runtime id may also coincide with the transcript id of some teammate of
/// another team (seen in real data); a teammate never leads, so the alias wins.
#[test]
fn team_lead_runtime_session_id_resolves_to_the_lead_transcript() {
    for (alias_first, collide) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut ws = Ws::new();
        let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
        if collide {
            ws.discover(AgentBuilder::claude_teammate("s-runtime", "other", "team-old").started(1));
        }
        let mate =
            ws.discover(AgentBuilder::claude_teammate("s-mate", "worker", "session-rt").started(5));
        let alias = EventLog::new(&lead)
            .at(1)
            .attribute(AgentAttributeKey::RuntimeSessionId, "s-runtime");
        let config = SideStateUpdate::ClaudeTeam {
            team: "session-rt".into(),
            lead_session_id: "s-runtime".into(),
            members: vec![],
        };
        if alias_first {
            ws.one(alias);
            ws.side(config);
        } else {
            ws.side(config);
            assert_eq!(ws.tree_parent(&mate), None, "runtime id not resolvable yet");
            ws.one(alias);
        }
        assert_eq!(
            ws.parent(&mate),
            Some(lead.clone()),
            "alias_first={alias_first}"
        );
        assert_eq!(ws.tree_parent(&mate), Some(lead.clone()));
        assert!(!ws.view.roots.contains(&mate));
        // The alias is not an ordinary (latest-wins) attribute.
        assert!(
            !ws.view.agents[&lead]
                .attributes
                .contains_key(&AgentAttributeKey::RuntimeSessionId)
        );

        // "team-lead" handles of the teammate resolve to the transcript too.
        let mut log = EventLog::new(&mate);
        ws.one(log.at(6).message(
            MessageDirection::Outgoing,
            handle::id(&mate),
            vec![handle::member(None, "team-lead")],
            AgentMessageKind::Message,
            "done",
        ));
        assert_eq!(
            ws.view.feed.last().unwrap().to,
            vec![FeedParty::Agent(lead)]
        );
    }
}

/// A teammate's `agent-name` record holds the display name inherited from its
/// lead. The teammate name wins for the label, and the inherited name neither
/// makes the teammate answer to the lead's name nor hides its own name.
#[test]
fn teammate_label_ignores_the_inherited_agent_name() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "worker", "team-1").started(2));
    let mut log = EventLog::new(&mate);
    ws.one(
        log.at(3)
            .attribute(AgentAttributeKey::AgentName, "Lead display name"),
    );
    assert_eq!(ws.view.agents[&mate].label(), "worker");

    // A handle naming the lead's display name does not resolve to the teammate.
    let mut lead_log = EventLog::new(&lead);
    ws.one(lead_log.at(4).message(
        MessageDirection::Outgoing,
        handle::id(&lead),
        vec![handle::member(Some("team-1"), "Lead display name")],
        AgentMessageKind::Message,
        "hi",
    ));
    assert!(matches!(
        ws.view.feed.last().unwrap().to[0],
        FeedParty::Unresolved { .. }
    ));

    // Without a header name, the spawn name labels it, not the inherited name.
    let mut r: AgentRef = AgentBuilder::claude_teammate("s-mate2", "x", "team-1")
        .started(5)
        .build();
    r.name = None;
    let mate2 = ws.discover(r);
    let mut log2 = EventLog::new(&mate2);
    ws.one(
        log2.at(6)
            .attribute(AgentAttributeKey::AgentName, "Lead display name"),
    );
    assert_ne!(ws.view.agents[&mate2].label(), "Lead display name");
    ws.one(
        lead_log
            .at(7)
            .spawn(handle::id(&mate2), AgentKind::Teammate, Some("worker-2")),
    );
    assert_eq!(ws.view.agents[&mate2].label(), "worker-2");
}

/// A transcript that was continued in another one (`continued-in`) is the same
/// logical session: it is shown under its continuation and is finished.
#[test]
fn continued_transcript_is_shown_under_its_continuation() {
    let mut ws = Ws::new();
    let old = ws.discover(AgentBuilder::claude_main("s-old").started(0));
    let new = ws.discover(AgentBuilder::claude_main("s-new").started(0));
    let mut log = EventLog::new(&old);
    ws.one(log.at(1).user("hi"));
    assert_eq!(ws.status(&old), AgentStatus::Running);
    assert_eq!(ws.view.roots.len(), 2);

    ws.one(log.at(2).attribute(AgentAttributeKey::ContinuedIn, "s-new"));
    assert_eq!(ws.tree_parent(&old), Some(new.clone()));
    assert_eq!(ws.view.roots, vec![new.clone()]);
    assert_eq!(ws.view.agents[&new].children, vec![old.clone()]);
    assert_eq!(ws.status(&old), AgentStatus::Done);
    // Not a spawn link: identity stays a root session.
    assert_eq!(ws.parent(&old), None);
}
