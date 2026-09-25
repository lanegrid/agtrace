//! Per-agent detail state of the workspace fold (overview lanes, agent detail
//! screen): activity, status history, context series, instructions, result, last
//! assistant text, totals and end reason.

use agtrace_engine::workspace::{
    AgentStatus, InstructionKind, NoWindow, WorkspaceEvent, WorkspaceView,
};
use agtrace_testing::synth::{AgentBuilder, EventLog, handle, ts};
use agtrace_types::{
    AgentAttributeKey, AgentEvent, AgentId, AgentKind, AgentLifecyclePayload, AgentMessageKind,
    AgentOp, AgentRef, AgentSpawnPayload, AgentToolArgs, EventPayload, LifecycleTransition,
    MessageDirection, PlanItem, PlanItemStatus, PlanPayload, QueueOperationPayload,
    ReasoningPayload, TokenInput, TokenOutput, TokenUsagePayload, ToolCallPayload, TurnOutcome,
};

struct Ws {
    view: WorkspaceView,
}

impl Ws {
    fn new() -> Self {
        Self {
            view: WorkspaceView::new(),
        }
    }

    fn apply(&mut self, ev: WorkspaceEvent) {
        self.view.apply(ev, &NoWindow, ts(0));
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
}

fn secs_of(t: chrono::DateTime<chrono::Utc>) -> i64 {
    (t - ts(0)).num_seconds()
}

#[test]
fn activity_buckets_count_own_log_activity_per_minute() {
    let mut ws = Ws::new();
    let id = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let mut l = EventLog::new(&id);
    let call = l.at(30).bash("cargo test");
    ws.feed(vec![
        l.at(0).user("go"),
        l.at(1).usage(1_000, Some("claude-opus-5")), // usage is not activity
        call.clone(),
        l.at(35).tool_result(call.id, "ok", false),
        l.at(70).turn_end(), // metadata, not activity
        l.at(130).compaction(Some(900), Some(100)),
    ]);
    let d = &ws.view.agents[&id].detail;
    let got: Vec<(i64, u32, u32)> = d
        .activity
        .iter()
        .map(|b| (secs_of(b.start), b.events, b.compactions))
        .collect();
    assert_eq!(got, vec![(0, 3, 0), (120, 0, 1)]);
    assert_eq!(d.compactions, 1);
}

#[test]
fn status_history_records_own_and_parent_side_transitions_in_event_time() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let sub = ws.discover(AgentBuilder::claude_subagent("s1", "a1").started(10));
    let mut s = EventLog::new(&sub);
    ws.feed(vec![
        s.at(10).user("find call sites"),
        s.at(20).assistant("working"),
        s.at(40).turn_end(),
        s.at(50).assistant("more"),
    ]);
    let mut l = EventLog::new(&lead);
    ws.feed(vec![
        l.at(0).user("go"),
        l.at(60)
            .lifecycle(handle::native("a1"), LifecycleTransition::Completed),
    ]);
    let h = &ws.view.agents[&sub].detail.status_history;
    let points: Vec<(i64, AgentStatus)> = h.iter().map(|p| (secs_of(p.at), p.status)).collect();
    assert_eq!(
        points,
        vec![
            (10, AgentStatus::Running),
            (40, AgentStatus::Idle),
            (50, AgentStatus::Running),
            (60, AgentStatus::Done),
        ]
    );
    assert_eq!(h.at(ts(45)), Some(AgentStatus::Idle));
    assert_eq!(h.entered(AgentStatus::Done), Some(ts(60)));
}

#[test]
fn context_series_has_usage_samples_and_compaction_markers() {
    let mut ws = Ws::new();
    let id = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let mut l = EventLog::new(&id);
    ws.feed(vec![
        l.at(1).usage(100_000, Some("claude-opus-5")),
        l.at(2).usage(100_000, Some("claude-opus-5")),
        l.at(3).compaction(Some(900_000), Some(50_000)),
        l.at(4).usage(60_000, Some("claude-opus-5")),
    ]);
    let series: Vec<(i64, Option<u64>, bool)> = ws.view.agents[&id]
        .detail
        .context_series
        .iter()
        .map(|p| (secs_of(p.at), p.tokens, p.compaction))
        .collect();
    assert_eq!(
        series,
        vec![
            (1, Some(100_000), false),
            (2, Some(100_000), false),
            (3, Some(50_000), true),
            (4, Some(60_000), false),
        ]
    );
}

#[test]
fn instructions_are_prompts_and_messages_addressed_to_the_agent() {
    let team = Some("t");
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "mate", "t").started(5));
    let mut l = EventLog::new(&lead);
    ws.feed(vec![
        l.at(0).user("Audit the parser"),
        l.at(1).spawn(
            handle::member(team, "mate"),
            AgentKind::Teammate,
            Some("mate"),
        ),
        // A report from its own teammate is not an instruction of the lead ...
        l.at(30).message(
            MessageDirection::Incoming,
            handle::member(team, "mate"),
            vec![handle::member(team, "team-lead")],
            AgentMessageKind::Message,
            "done, 3 bugs",
        ),
        // ... nor is a result notification.
        l.at(31).message(
            MessageDirection::Incoming,
            handle::native("zz"),
            vec![handle::member(team, "team-lead")],
            AgentMessageKind::TaskNotification,
            "x",
        ),
        l.at(40)
            .push(EventPayload::QueueOperation(QueueOperationPayload {
                operation: "remove".to_string(),
                content: Some("also check the tokenizer".to_string()),
                task_id: None,
                reason: Some("absorbed_mid_turn".to_string()),
            })),
        l.at(41)
            .push(EventPayload::QueueOperation(QueueOperationPayload {
                operation: "enqueue".to_string(),
                content: Some("noise".to_string()),
                task_id: None,
                reason: None,
            })),
    ]);
    let long_task = "review the parser\n".repeat(400); // > 4 KiB, < 16 KiB
    let mut m = EventLog::new(&mate);
    ws.feed(vec![
        m.at(5).message(
            MessageDirection::Incoming,
            handle::member(team, "team-lead"),
            vec![handle::member(team, "mate")],
            AgentMessageKind::NewTask,
            &long_task,
        ),
        m.at(20).message(
            MessageDirection::Incoming,
            handle::member(team, "team-lead"),
            vec![handle::member(team, "mate")],
            AgentMessageKind::Message,
            "also check the cache key",
        ),
    ]);

    let lead_ins: Vec<(InstructionKind, Option<String>)> = ws.view.agents[&lead]
        .detail
        .all_instructions()
        .map(|i| (i.kind.clone(), i.text.clone()))
        .collect();
    assert_eq!(
        lead_ins,
        vec![
            (
                InstructionKind::Prompt,
                Some("Audit the parser".to_string())
            ),
            (
                InstructionKind::Queued,
                Some("also check the tokenizer".to_string())
            ),
        ]
    );

    let d = &ws.view.agents[&mate].detail;
    let first = d.initial_task.as_ref().expect("initial task");
    assert_eq!(
        first.kind,
        InstructionKind::Message(AgentMessageKind::NewTask)
    );
    assert_eq!(first.from.as_deref(), Some("s-lead"));
    assert_eq!(
        first.text.as_deref(),
        Some(long_task.trim()),
        "kept in full"
    );
    let later: Vec<&str> = d
        .instructions
        .iter()
        .filter_map(|i| i.text.as_deref())
        .collect();
    assert_eq!(later, vec!["also check the cache key"]);
}

#[test]
fn encrypted_codex_task_keeps_sender_and_flag() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root").started(0));
    let child =
        ws.discover(AgentBuilder::codex_child("t-c", "t-root", "t-root", "/root/c").started(5));
    let mut c = EventLog::new(&child);
    ws.feed(vec![c.at(5).encrypted_message(
        MessageDirection::Incoming,
        handle::path("/root"),
        vec![handle::path("/root/c")],
        AgentMessageKind::NewTask,
    )]);
    let _ = root;
    let i = ws.view.agents[&child]
        .detail
        .initial_task
        .clone()
        .expect("task");
    assert!(i.encrypted);
    assert_eq!(i.text, None);
    assert_eq!(i.from.as_deref(), Some("/root"));
}

#[test]
fn result_comes_from_own_final_answer_or_the_parents_notification() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("t-root").started(0));
    let child =
        ws.discover(AgentBuilder::codex_child("t-c", "t-root", "t-root", "/root/c").started(5));
    let mut c = EventLog::new(&child);
    ws.feed(vec![c.at(50).message(
        MessageDirection::Outgoing,
        handle::path("/root/c"),
        vec![handle::path("/root")],
        AgentMessageKind::FinalAnswer,
        "3 flaky tests",
    )]);
    let r = ws.view.agents[&child]
        .detail
        .result
        .clone()
        .expect("result");
    assert_eq!(r.kind, AgentMessageKind::FinalAnswer);
    assert_eq!(r.text.as_deref(), Some("3 flaky tests"));
    assert!(r.own);
    assert!(ws.view.agents[&root].detail.result.is_none());

    // Claude: the task-notification in the lead's log carries the subagent's
    // result, also when the subagent is discovered after the notification.
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let mut l = EventLog::new(&lead);
    ws.feed(vec![l.at(90).message(
        MessageDirection::Incoming,
        handle::native("a1"),
        vec![handle::member(None, "team-lead")],
        AgentMessageKind::TaskNotification,
        "found 2 call sites",
    )]);
    let sub = ws.discover(AgentBuilder::claude_subagent("s1", "a1").started(10));
    let r = ws.view.agents[&sub]
        .detail
        .result
        .clone()
        .expect("late result");
    assert_eq!(r.text.as_deref(), Some("found 2 call sites"));
    assert!(!r.own);

    // A reset of the subagent's own log keeps the result reported by the lead.
    let mut s = EventLog::new(&sub);
    ws.apply(WorkspaceEvent::Events {
        agent: sub.clone(),
        events: vec![s.at(10).user("find call sites")],
        reset: true,
    });
    assert!(ws.view.agents[&sub].detail.result.is_some());
}

#[test]
fn spawn_prompt_is_attached_from_the_spawn_tool_call() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let sub = ws.discover(AgentBuilder::claude_subagent("s1", "a1").started(10));
    let mut l = EventLog::new(&lead);
    let mut args = AgentToolArgs::new(AgentOp::Spawn);
    args.message_preview = Some("Find every call site of parse_line".to_string());
    let call = l.at(9).tool_call(ToolCallPayload::Agent {
        name: "Agent".to_string(),
        arguments: args,
        provider_call_id: Some("toolu_1".to_string()),
    });
    let spawn = l.at(10).spawn_with(AgentSpawnPayload {
        child: handle::native("a1"),
        kind: AgentKind::Subagent,
        name: None,
        agent_type: Some("Explore".to_string()),
        requested_model: None,
        resolved_model: None,
        requested_effort: None,
        description: Some("explore".to_string()),
        spawn_call_id: Some("toolu_1".to_string()),
        tool_call_id: Some(call.id),
    });
    ws.feed(vec![call, spawn]);
    let s = ws.view.agents[&sub].spawn.clone().expect("linked");
    assert_eq!(
        s.prompt.as_deref(),
        Some("Find every call site of parse_line")
    );
    assert!(!s.prompt_encrypted);
}

#[test]
fn last_text_totals_and_end_reason() {
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s1").started(0));
    let sub = ws.discover(AgentBuilder::claude_subagent("s1", "a1").started(10));
    let mut s = EventLog::new(&sub);
    let usage = |i: u64, o: u64, key: &str| {
        EventPayload::TokenUsage(
            TokenUsagePayload::new(TokenInput::new(i, 0, 0), TokenOutput::new(o, 0, 0))
                .with_dedupe_key(Some(key.to_string())),
        )
    };
    let call = s.at(12).bash("rg parse_line");
    ws.feed(vec![
        s.at(10).user("find call sites"),
        s.at(11).push(EventPayload::Reasoning(ReasoningPayload {
            text: "Let me grep first.".to_string(),
        })),
        s.at(11).push(usage(1_000, 1, "m1")),
        s.at(11).push(usage(1_000, 50, "m1")),
        call.clone(),
        s.at(13).tool_result(call.id, "2 files", false),
        s.at(14)
            .assistant("I'll patch the tokenizer so that empty input is rejected."),
        s.at(14).push(usage(3_000, 20, "m2")),
        s.at(15).turn_end_with(TurnOutcome::Failed {
            error: Some("rate limited".to_string()),
        }),
    ]);
    let d = &ws.view.agents[&sub].detail;
    assert_eq!(
        d.last_message.as_ref().map(|m| m.text.as_str()),
        Some("I'll patch the tokenizer so that empty input is rejected.")
    );
    assert_eq!(
        d.last_reasoning.as_ref().map(|m| m.text.as_str()),
        Some("Let me grep first.")
    );
    assert_eq!((d.totals.input, d.totals.output), (4_000, 70));
    assert_eq!((d.totals.turns, d.totals.tool_calls), (1, 1));
    assert_eq!(d.end_reason.as_deref(), Some("rate limited"));

    // A kill reason reported by the parent.
    let mut l = EventLog::new(&lead);
    ws.feed(vec![l.at(20).push(EventPayload::AgentLifecycle(
        AgentLifecyclePayload {
            target: handle::native("a1"),
            transition: LifecycleTransition::Killed,
            reason: Some("stopped by user".to_string()),
            usage: None,
        },
    ))]);
    let d = &ws.view.agents[&sub].detail;
    assert_eq!(d.end_reason.as_deref(), Some("stopped by user"));
    assert_eq!(
        d.status_history.last().map(|p| p.status),
        Some(AgentStatus::Killed)
    );
}

fn task_created(id: &str, subject: &str, active: &str, team: Option<&str>) -> EventPayload {
    EventPayload::Plan(PlanPayload::TaskCreated {
        item: PlanItem {
            id: Some(id.to_string()),
            subject: subject.to_string(),
            active_form: Some(active.to_string()),
            status: PlanItemStatus::Pending,
        },
        description: None,
        team: team.map(str::to_string),
    })
}

fn task_updated(id: &str, status: PlanItemStatus, team: Option<&str>) -> EventPayload {
    EventPayload::Plan(PlanPayload::TaskUpdated {
        id: id.to_string(),
        status: Some(status),
        subject: None,
        active_form: None,
        team: team.map(str::to_string),
    })
}

fn tasks_of(ws: &Ws, id: &AgentId) -> Vec<(Option<String>, PlanItemStatus, Option<String>)> {
    ws.view.agents[id]
        .detail
        .plan
        .tasks
        .iter()
        .map(|t| (t.subject.clone(), t.status.clone(), t.by.clone()))
        .collect()
}

/// Claude Agent Teams share one task list: a teammate's update shows in the lead's
/// list (attributed to the teammate) and in the teammate's own plan (with the
/// subject from the lead's list); a deleted task disappears.
#[test]
fn team_task_updates_reach_the_lead_and_the_updater() {
    let team = Some("t");
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "mate", "t").started(5));
    let mut l = EventLog::new(&lead);
    ws.feed(vec![
        l.at(0).user("Audit the parser"),
        l.at(1).push(task_created("1", "Parse", "Parsing", team)),
        l.at(2).push(task_created("2", "Test", "Testing", team)),
        l.at(3)
            .push(task_created("3", "Scratch", "Scratching", team)),
        l.at(4).spawn(
            handle::member(team, "mate"),
            AgentKind::Teammate,
            Some("mate"),
        ),
        l.at(6)
            .push(task_updated("3", PlanItemStatus::Deleted, team)),
    ]);
    let mut m = EventLog::new(&mate);
    ws.feed(vec![
        m.at(20)
            .push(task_updated("1", PlanItemStatus::InProgress, team)),
        m.at(21).assistant("parsing"),
    ]);

    let mate_label = Some("mate".to_string());
    assert_eq!(
        tasks_of(&ws, &lead),
        vec![
            (
                Some("Parse".into()),
                PlanItemStatus::InProgress,
                mate_label.clone()
            ),
            (Some("Test".into()), PlanItemStatus::Pending, None),
        ]
    );
    assert_eq!(
        tasks_of(&ws, &mate),
        vec![(Some("Parse".into()), PlanItemStatus::InProgress, None)]
    );
    let doing = ws.view.agents[&mate].detail.plan.in_progress().unwrap();
    assert_eq!(doing.doing(), Some("Parsing"));
    // The lead is not working on it itself.
    assert_eq!(
        ws.view.agents[&lead]
            .detail
            .plan
            .in_progress()
            .map(|t| t.by.clone()),
        Some(mate_label)
    );
}

/// The teammate's log is folded before the lead's: its update waits for the lead
/// to become resolvable and then applies on top of the (earlier) creation.
#[test]
fn team_task_update_folded_before_the_creation() {
    let team = Some("t");
    let mut ws = Ws::new();
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mate = ws.discover(AgentBuilder::claude_teammate("s-mate", "mate", "t").started(5));
    let mut m = EventLog::new(&mate);
    ws.feed(vec![m.at(20).push(task_updated(
        "1",
        PlanItemStatus::Completed,
        team,
    ))]);
    let mut l = EventLog::new(&lead);
    ws.feed(vec![
        l.at(1).push(task_created("1", "Parse", "Parsing", team)),
        l.at(4).spawn(
            handle::member(team, "mate"),
            AgentKind::Teammate,
            Some("mate"),
        ),
    ]);
    assert_eq!(
        tasks_of(&ws, &lead),
        vec![(
            Some("Parse".into()),
            PlanItemStatus::Completed,
            Some("mate".into())
        )]
    );
    // The teammate only knows the id (the subject is looked up when shown).
    assert_eq!(
        tasks_of(&ws, &mate),
        vec![(None, PlanItemStatus::Completed, None)]
    );
}

/// Without a team the task list is the agent's own; a `TodoWrite` list replaces
/// it; plan text and goal keep the latest value.
#[test]
fn own_task_list_plan_text_and_goal() {
    let mut ws = Ws::new();
    let id = ws.discover(AgentBuilder::codex_root("r1").started(0));
    let mut l = EventLog::new(&id);
    ws.feed(vec![
        l.at(1).push(task_created("1", "Read", "Reading", None)),
        l.at(2)
            .push(task_updated("1", PlanItemStatus::Completed, None)),
        l.at(3).push(EventPayload::Plan(PlanPayload::Goal {
            objective: "Ship it".into(),
            status: Some("active".into()),
        })),
        l.at(4).push(EventPayload::Plan(PlanPayload::Text {
            text: "# Plan v1".into(),
        })),
        l.at(5).push(EventPayload::Plan(PlanPayload::Text {
            text: "# Plan v2".into(),
        })),
        l.at(6).push(EventPayload::Plan(PlanPayload::Goal {
            objective: "Ship it".into(),
            status: Some("paused".into()),
        })),
    ]);
    let p = &ws.view.agents[&id].detail.plan;
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].status, PlanItemStatus::Completed);
    assert!(p.in_progress().is_none());
    assert_eq!(p.text.as_ref().map(|t| t.text.as_str()), Some("# Plan v2"));
    let g = p.goal.as_ref().unwrap();
    assert_eq!(
        (g.objective.as_str(), g.status.as_deref()),
        ("Ship it", Some("paused"))
    );
    // Plan events are not activity (the lane / running signal ignore them).
    assert!(ws.view.agents[&id].detail.activity.is_empty());

    ws.feed(vec![l.at(7).push(EventPayload::Plan(PlanPayload::Items {
        items: vec![PlanItem {
            id: None,
            subject: "Write".into(),
            active_form: None,
            status: PlanItemStatus::InProgress,
        }],
    }))]);
    let p = &ws.view.agents[&id].detail.plan;
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.in_progress().and_then(|t| t.doing()), Some("Write"));
}

/// Effort: the own log's latest `Effort` attribute, else the spawn's request.
#[test]
fn effort_from_own_log_or_spawn_request() {
    let mut ws = Ws::new();
    let root = ws.discover(AgentBuilder::codex_root("r1").started(0));
    let child = ws.discover(AgentBuilder::codex_child("c1", "r1", "r1", "/root/judge").started(5));
    let mut r = EventLog::new(&root);
    ws.feed(vec![
        r.at(1).attribute(AgentAttributeKey::Effort, "low"),
        r.at(2).attribute(AgentAttributeKey::Effort, "high"),
        r.at(3).spawn_with(AgentSpawnPayload {
            child: handle::path("/root/judge"),
            kind: AgentKind::CodexThread,
            name: Some("judge".into()),
            agent_type: None,
            requested_model: None,
            resolved_model: None,
            requested_effort: Some("medium".into()),
            description: None,
            spawn_call_id: None,
            tool_call_id: None,
        }),
    ]);
    assert_eq!(ws.view.agents[&root].effort(), Some("high"));
    assert_eq!(ws.view.agents[&child].effort(), Some("medium"));
    let mut c = EventLog::new(&child);
    ws.feed(vec![c.at(6).attribute(AgentAttributeKey::Effort, "low")]);
    assert_eq!(ws.view.agents[&child].effort(), Some("low"));
}
