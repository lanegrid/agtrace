//! Synthetic multi-agent workspace for the watch TUI tests and the preview example.
//!
//! Everything is invented (ids, names, text). The scenario covers:
//! - a Claude lead with two teammates (team `audit`), an async subagent and a fork;
//! - a Codex root with a nested child tree (`/root/judge`, `/root/judge/x`, `/root/scout`);
//! - plaintext and encrypted inter-agent messages, FINAL_ANSWER, idle notification,
//!   task notification;
//! - compaction, model change, interrupted turn, queued prompt absorbed mid-turn,
//!   a failed Codex sub-action and running tools;
//! - reasoning effort (own log and spawn request), a team-shared task list (created
//!   by the lead, updated by a teammate), a Codex goal and plan text.
#![allow(dead_code)]

use agtrace_sdk::types::{
    AgentAttributeKey, AgentEvent, AgentKind, AgentMessageKind, AgentProvider, AgentRef,
    AgentSpawnPayload, ContextSource, ContextWindow, EventPayload, ExecuteArgs,
    LifecycleTransition, MessageDirection, PlanItem, PlanItemStatus, PlanPayload,
    QueueOperationPayload, SubActionStatus, ToolCallPayload, ToolSubActionPayload, TurnOutcome,
};
use agtrace_sdk::workspace::{ContextEvidence, WorkspaceEvent, WorkspaceView};
use agtrace_testing::synth::{AgentBuilder, EventLog, handle, ts};
use chrono::{DateTime, Utc};

/// Fixture clock: 12:05:00 on the synthetic day.
pub fn now() -> DateTime<Utc> {
    ts(300)
}

/// Deterministic stand-in for the catalog resolver: explicit log value, else `[1m]`
/// marker, else a 200k table value for known Claude models / 272k for Codex.
pub fn resolve(agent: &AgentRef, ev: &ContextEvidence) -> Option<ContextWindow> {
    let (tokens, source) = if let Some(t) = ev.explicit {
        (t, ContextSource::Log)
    } else if let Some(t) = ev.marker.or(ev.external_marker) {
        (t, ContextSource::ModelMarker)
    } else if ev.model.is_some() {
        match agent.provider {
            AgentProvider::ClaudeCode => (200_000, ContextSource::ModelTable),
            AgentProvider::Codex => (272_000, ContextSource::ModelTable),
        }
    } else {
        return None;
    };
    Some(ContextWindow {
        tokens,
        source,
        model: ev.model.clone(),
        overruled: None,
    })
}

fn exec(log: &mut EventLog, name: &str, command: &str) -> AgentEvent {
    log.tool_call(ToolCallPayload::Execute {
        name: name.to_string(),
        arguments: ExecuteArgs {
            command: Some(command.to_string()),
            description: None,
            timeout: None,
            extra: serde_json::json!({}),
        },
        provider_call_id: None,
    })
}

fn task_created(log: &mut EventLog, id: &str, subject: &str, active: &str) -> AgentEvent {
    log.push(EventPayload::Plan(PlanPayload::TaskCreated {
        item: PlanItem {
            id: Some(id.to_string()),
            subject: subject.to_string(),
            active_form: Some(active.to_string()),
            status: PlanItemStatus::Pending,
        },
        description: None,
        team: Some("audit".to_string()),
    }))
}

fn task_updated(log: &mut EventLog, id: &str, status: PlanItemStatus) -> AgentEvent {
    log.push(EventPayload::Plan(PlanPayload::TaskUpdated {
        id: id.to_string(),
        status: Some(status),
        subject: None,
        active_form: None,
        team: Some("audit".to_string()),
    }))
}

fn spawn(
    log: &mut EventLog,
    child: agtrace_sdk::types::AgentHandle,
    kind: AgentKind,
    name: &str,
    agent_type: Option<&str>,
    model: Option<&str>,
) -> AgentEvent {
    log.spawn_with(AgentSpawnPayload {
        child,
        kind,
        name: Some(name.to_string()),
        agent_type: agent_type.map(str::to_string),
        requested_model: None,
        resolved_model: model.map(str::to_string),
        requested_effort: None,
        description: None,
        spawn_call_id: Some(format!("call_{name}")),
        tool_call_id: None,
    })
}

/// Watcher events of the scenario, in arrival order.
pub fn events() -> Vec<WorkspaceEvent> {
    let team = Some("audit");
    let lead = AgentBuilder::claude_main("s-lead").started(0).build();
    let audit_a = AgentBuilder::claude_teammate("s-audit-a", "audit-A", "audit")
        .started(12)
        .build();
    let audit_b = AgentBuilder::claude_teammate("s-audit-b", "audit-B", "audit")
        .started(13)
        .build();
    let sub = AgentBuilder::claude_subagent("s-lead", "a7ac2e91")
        .agent_type("Explore")
        .started(20)
        .build();
    let fork = AgentBuilder::claude_fork("s-lead", "f0cc1a22")
        .started(25)
        .build();
    let root = AgentBuilder::codex_root("t-root").started(5).build();
    let judge = AgentBuilder::codex_child("t-judge", "t-root", "t-root", "/root/judge")
        .started(40)
        .build();
    let judge_x = AgentBuilder::codex_child("t-judge-x", "t-root", "t-judge", "/root/judge/x")
        .started(90)
        .build();
    let scout = AgentBuilder::codex_child("t-scout", "t-root", "t-root", "/root/scout")
        .started(45)
        .build();

    let mut out = Vec::new();
    for r in [
        &lead, &audit_a, &audit_b, &sub, &fork, &root, &judge, &judge_x, &scout,
    ] {
        out.push(WorkspaceEvent::AgentDiscovered(r.clone()));
    }
    let mut push = |events: Vec<AgentEvent>| {
        let agent = events[0].agent.clone();
        out.push(WorkspaceEvent::Events {
            agent,
            events,
            reset: false,
        });
    };

    // ---------------------------------------------------------------- Claude lead
    let mut l = EventLog::new(&lead.id);
    let mut ev = vec![
        l.at(0).user("Audit the parser with a team of reviewers"),
        l.at(1).model_change(None, "claude-opus-5"),
        l.at(1).attribute(AgentAttributeKey::Effort, "high"),
        l.at(2).usage(96_000, Some("claude-opus-5")),
        l.at(3).assistant("Spawning two reviewers and an explorer."),
        task_created(l.at(4), "1", "Review the parser", "Reviewing the parser"),
        task_created(l.at(4), "2", "Scan for panics", "Scanning for panics"),
        task_created(l.at(4), "3", "Write the summary", "Writing the summary"),
        spawn(
            l.at(10),
            handle::member(team, "audit-A"),
            AgentKind::Teammate,
            "audit-A",
            Some("general-purpose"),
            Some("claude-opus-5"),
        ),
        spawn(
            l.at(11),
            handle::member(team, "audit-B"),
            AgentKind::Teammate,
            "audit-B",
            Some("general-purpose"),
            None,
        ),
        l.at(12).message(
            MessageDirection::Outgoing,
            handle::member(team, "team-lead"),
            vec![handle::member(team, "audit-A")],
            AgentMessageKind::NewTask,
            "review parser",
        ),
        spawn(
            l.at(20),
            handle::native("a7ac2e91"),
            AgentKind::Subagent,
            "explore call sites",
            Some("Explore"),
            Some("claude-sonnet-5"),
        ),
        spawn(
            l.at(25),
            handle::native("f0cc1a22"),
            AgentKind::Fork,
            "fork: bench",
            None,
            None,
        ),
        l.at(50).turn_end_with(TurnOutcome::Interrupted),
        l.at(55)
            .push(EventPayload::QueueOperation(QueueOperationPayload {
                operation: "remove".to_string(),
                content: Some("also check the tokenizer".to_string()),
                task_id: None,
                reason: Some("absorbed_mid_turn".to_string()),
            })),
        l.at(56)
            .push(EventPayload::QueueOperation(QueueOperationPayload {
                operation: "enqueue".to_string(),
                content: Some("noise".to_string()),
                task_id: None,
                reason: None,
            })),
        l.at(60).message(
            MessageDirection::Incoming,
            handle::member(team, "audit-A"),
            vec![handle::member(team, "team-lead")],
            AgentMessageKind::Message,
            "done, found 3 bugs",
        ),
        l.at(70)
            .model_change(Some("claude-opus-5"), "claude-opus-5-5[1m]"),
        l.at(80).compaction(Some(974_000), Some(112_000)),
        l.at(81).usage(420_000, Some("claude-opus-5-5[1m]")),
        l.at(100)
            .lifecycle(handle::member(team, "audit-B"), LifecycleTransition::Idle),
        l.at(150)
            .lifecycle(handle::native("a7ac2e91"), LifecycleTransition::Completed),
        l.at(150).message(
            MessageDirection::Incoming,
            handle::native("a7ac2e91"),
            vec![handle::member(team, "team-lead")],
            AgentMessageKind::TaskNotification,
            "found 2 call sites",
        ),
        l.at(200)
            .notification("api_error", "overloaded (not shown)"),
    ];
    ev.push(exec(l.at(290), "Bash", "mise run test"));
    push(ev);

    // ---------------------------------------------------------------- teammates
    let mut a = EventLog::new(&audit_a.id);
    let call = a.at(30).bash("cargo test -p parser");
    push(vec![
        a.at(12).message(
            MessageDirection::Incoming,
            handle::member(team, "team-lead"),
            vec![handle::member(team, "audit-A")],
            AgentMessageKind::NewTask,
            "review parser",
        ),
        a.at(13).usage(24_000, Some("claude-opus-5")),
        task_updated(a.at(14), "1", PlanItemStatus::InProgress),
        call.clone(),
        a.at(35).tool_result(call.id, "3 failed", true),
        a.at(60).message(
            MessageDirection::Outgoing,
            handle::member(team, "audit-A"),
            vec![handle::member(team, "team-lead")],
            AgentMessageKind::Message,
            "done, found 3 bugs",
        ),
        a.at(61).turn_end(),
    ]);

    let mut b = EventLog::new(&audit_b.id);
    let call = b.at(20).bash("rg unwrap src/");
    push(vec![
        b.at(13).user("scan for panics"),
        b.at(14).usage(18_000, Some("claude-opus-5")),
        task_updated(b.at(15), "2", PlanItemStatus::InProgress),
        call.clone(),
        b.at(22).tool_result(call.id, "12 matches", false),
        task_updated(b.at(95), "2", PlanItemStatus::Completed),
        b.at(95).assistant("No panics on the hot path."),
        b.at(99).turn_end(),
    ]);

    // ---------------------------------------------------------------- subagent + fork
    let mut s = EventLog::new(&sub.id);
    let call = s.at(21).tool_call(ToolCallPayload::Execute {
        name: "Grep".to_string(),
        arguments: ExecuteArgs {
            command: Some("parse_line".to_string()),
            description: None,
            timeout: None,
            extra: serde_json::json!({}),
        },
        provider_call_id: None,
    });
    push(vec![
        s.at(20).user("Find call sites of parse_line"),
        s.at(21).usage(30_000, Some("claude-sonnet-5")),
        call.clone(),
        s.at(24).tool_result(call.id, "2 files", false),
        s.at(149)
            .assistant("Two call sites: decoder.rs and lab.rs."),
    ]);

    let mut f = EventLog::new(&fork.id);
    push(vec![
        f.at(25).user("benchmark the decoder"),
        f.at(26).usage(150_000, Some("claude-opus-5")),
        exec(f.at(260), "Bash", "cargo bench -p agtrace-providers"),
    ]);

    // ---------------------------------------------------------------- Codex tree
    let mut r = EventLog::new(&root.id);
    push(vec![
        r.at(5).user("triage the flaky tests"),
        r.at(5).model_change(None, "gpt-5.6"),
        r.at(5).window_hint(258_400),
        r.at(5).attribute(AgentAttributeKey::Effort, "medium"),
        r.at(6).usage(60_000, Some("gpt-5.6")),
        r.at(7).push(EventPayload::Plan(PlanPayload::Goal {
            objective: "Make the watch tests deterministic".to_string(),
            status: Some("active".to_string()),
        })),
        r.at(30).push(EventPayload::Plan(PlanPayload::Text {
            text: "# Triage\n\n1. Reproduce each flaky test in a loop.\n2. Fix the timing assumption.\n3. Verify with 50 runs."
                .to_string(),
        })),
        spawn(
            r.at(40),
            handle::id(&judge.id),
            AgentKind::CodexThread,
            "judge",
            None,
            None,
        ),
        r.at(40).encrypted_message(
            MessageDirection::Outgoing,
            handle::path("/root"),
            vec![handle::path("/root/judge")],
            AgentMessageKind::NewTask,
        ),
        r.at(45).spawn_with(AgentSpawnPayload {
            child: handle::id(&scout.id),
            kind: AgentKind::CodexThread,
            name: Some("scout".to_string()),
            agent_type: None,
            requested_model: None,
            resolved_model: None,
            requested_effort: Some("low".to_string()),
            description: None,
            spawn_call_id: Some("call_scout".to_string()),
            tool_call_id: None,
        }),
        r.at(45).encrypted_message(
            MessageDirection::Outgoing,
            handle::path("/root"),
            vec![handle::path("/root/scout")],
            AgentMessageKind::NewTask,
        ),
        r.at(120).message(
            MessageDirection::Incoming,
            handle::path("/root/scout"),
            vec![handle::path("/root")],
            AgentMessageKind::FinalAnswer,
            "3 flaky tests, all in watch_command.rs",
        ),
        r.at(120)
            .lifecycle(handle::path("/root/scout"), LifecycleTransition::Completed),
        r.at(130).compaction(Some(240_000), None),
        r.at(131).usage(31_000, Some("gpt-5.6")),
        r.at(140).encrypted_message(
            MessageDirection::Outgoing,
            handle::path("/root"),
            vec![handle::path("/root/judge")],
            AgentMessageKind::Message,
        ),
        r.at(141).turn_end(),
    ]);

    let mut j = EventLog::new(&judge.id);
    let exec_call = exec(j.at(60), "exec", "cargo test -p agtrace-cli");
    let sub_action = |log: &mut EventLog, cmd: &str, status, exit| {
        log.push(EventPayload::ToolSubAction(ToolSubActionPayload {
            parent_tool_call_id: Some(exec_call.id),
            call: ToolCallPayload::Execute {
                name: "exec_command".to_string(),
                arguments: ExecuteArgs {
                    command: Some(cmd.to_string()),
                    description: None,
                    timeout: None,
                    extra: serde_json::json!({}),
                },
                provider_call_id: None,
            },
            status,
            exit_code: exit,
            output_preview: None,
            duration_ms: None,
        }))
    };
    let mut jev = vec![
        j.at(40).encrypted_message(
            MessageDirection::Incoming,
            handle::path("/root"),
            vec![handle::path("/root/judge")],
            AgentMessageKind::NewTask,
        ),
        j.at(40).window_hint(258_400),
        j.at(41).usage(200_000, Some("gpt-5.6")),
        exec_call.clone(),
    ];
    jev.push(sub_action(
        j.at(70),
        "cargo test -p agtrace-cli",
        SubActionStatus::Failed,
        Some(101),
    ));
    jev.push(sub_action(
        j.at(75),
        "git diff --stat",
        SubActionStatus::Completed,
        Some(0),
    ));
    jev.push(spawn(
        j.at(90),
        handle::id(&judge_x.id),
        AgentKind::CodexThread,
        "x",
        None,
        None,
    ));
    jev.push(j.at(140).encrypted_message(
        MessageDirection::Incoming,
        handle::path("/root"),
        vec![handle::path("/root/judge")],
        AgentMessageKind::Message,
    ));
    push(jev);

    let mut x = EventLog::new(&judge_x.id);
    push(vec![
        x.at(90).window_hint(258_400),
        x.at(91).usage(12_000, Some("gpt-5.6")),
        x.at(280).assistant("Reproducing the flake in a loop."),
    ]);

    let mut sc = EventLog::new(&scout.id);
    push(vec![
        sc.at(45).encrypted_message(
            MessageDirection::Incoming,
            handle::path("/root"),
            vec![handle::path("/root/scout")],
            AgentMessageKind::NewTask,
        ),
        sc.at(45).window_hint(258_400),
        sc.at(46).usage(40_000, Some("gpt-5.6")),
        sc.at(119).message(
            MessageDirection::Outgoing,
            handle::path("/root/scout"),
            vec![handle::path("/root")],
            AgentMessageKind::FinalAnswer,
            "3 flaky tests, all in watch_command.rs",
        ),
        sc.at(120).turn_end(),
    ]);

    out
}

/// The scenario folded into a view at [`now`].
pub fn workspace() -> WorkspaceView {
    let mut view = WorkspaceView::new();
    for e in events() {
        view.apply(e, &resolve, now());
    }
    view
}

/// Watcher events of more sessions around the scenario (sessions screen, compact
/// overview):
/// - `c-team` "Ship the release": a live bg Claude session (idle process) whose
///   lead finished 3 subagents and killed 8, with one still running; plus an old
///   `/clear` stub `c-team-clear` that inherited its name (folded into it);
/// - `f00dcafe-job`: a live bg process that has no transcript yet;
/// - `c-done`: a Claude session that went quiet 23 minutes ago (recent);
/// - `x-old`: a Codex thread last written 2.5 hours ago (older).
pub fn more_session_events() -> Vec<WorkspaceEvent> {
    use agtrace_sdk::types::SlashCommandPayload;
    use agtrace_sdk::workspace::{ProcessStatus, SideStateUpdate};

    let mut out = Vec::new();
    let push = |out: &mut Vec<WorkspaceEvent>, events: Vec<AgentEvent>| {
        let agent = events[0].agent.clone();
        out.push(WorkspaceEvent::Events {
            agent,
            events,
            reset: false,
        });
    };

    let team = AgentBuilder::claude_main("c-team").started(30).build();
    out.push(WorkspaceEvent::AgentDiscovered(team.clone()));
    let mut l = EventLog::new(&team.id);
    let mut ev = vec![
        l.at(30)
            .attribute(AgentAttributeKey::AgentName, "Ship the release"),
        l.at(30).attribute(AgentAttributeKey::SessionKind, "bg"),
        l.at(31).user("cut the v2 release"),
        l.at(32).usage(50_000, Some("claude-opus-5")),
    ];
    let subs: Vec<(String, Option<LifecycleTransition>)> = (0..8)
        .map(|i| (format!("k{i}"), Some(LifecycleTransition::Killed)))
        .chain((0..3).map(|i| (format!("d{i}"), Some(LifecycleTransition::Completed))))
        .chain(std::iter::once(("r0".to_string(), None)))
        .collect();
    for (i, (aid, _)) in subs.iter().enumerate() {
        ev.push(spawn(
            l.at(40 + i as i64),
            handle::native(aid),
            AgentKind::Subagent,
            &format!("step {aid}"),
            Some("general-purpose"),
            None,
        ));
    }
    for (aid, end) in &subs {
        if let Some(t) = end {
            ev.push(l.at(200).lifecycle(handle::native(aid), *t));
        }
    }
    ev.push(l.at(210).assistant("Waiting for the last step."));
    ev.push(l.at(211).turn_end());
    push(&mut out, ev);
    for (i, (aid, _)) in subs.iter().enumerate() {
        let r = AgentBuilder::claude_subagent("c-team", aid)
            .started(40 + i as i64)
            .build();
        out.push(WorkspaceEvent::AgentDiscovered(r.clone()));
        let mut s = EventLog::new(&r.id);
        let mut ev = vec![s.at(40 + i as i64).user(&format!("do step {aid}"))];
        if aid == "r0" {
            ev.push(exec(s.at(270), "Bash", "cargo publish --dry-run"));
        }
        push(&mut out, ev);
    }
    out.push(WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess {
        session_id: "c-team".to_string(),
        pid: 501,
        alive: true,
        status: Some(ProcessStatus::Idle),
        name: Some("c-team".to_string()),
        bg: true,
        updated_at: ts(211),
    }));

    let stub = AgentBuilder::claude_main("c-team-clear")
        .started(-8000)
        .build();
    out.push(WorkspaceEvent::AgentDiscovered(stub.clone()));
    let mut s = EventLog::new(&stub.id);
    push(
        &mut out,
        vec![
            s.at(-8000)
                .attribute(AgentAttributeKey::AgentName, "Ship the release"),
            s.at(-8000)
                .push(EventPayload::SlashCommand(SlashCommandPayload {
                    name: "clear".to_string(),
                    args: None,
                })),
        ],
    );

    out.push(WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess {
        session_id: "f00dcafe-job".to_string(),
        pid: 502,
        alive: true,
        status: Some(ProcessStatus::Idle),
        name: Some("f00dcafe".to_string()),
        bg: true,
        updated_at: ts(120),
    }));

    let done = AgentBuilder::claude_main("c-done").started(-1200).build();
    out.push(WorkspaceEvent::AgentDiscovered(done.clone()));
    let mut d = EventLog::new(&done.id);
    push(
        &mut out,
        vec![
            d.at(-1200).user("rename the config keys"),
            d.at(-1150).assistant("Renamed."),
            d.at(-1100).turn_end(),
        ],
    );

    let old = AgentBuilder::codex_root("x-old").started(-9000).build();
    out.push(WorkspaceEvent::AgentDiscovered(old.clone()));
    let mut o = EventLog::new(&old.id);
    push(
        &mut out,
        vec![
            o.at(-9000).user("bump the lockfile"),
            o.at(-8990).turn_end(),
        ],
    );
    out
}

/// The scenario plus [`more_session_events`], folded at [`now`].
pub fn many_sessions() -> WorkspaceView {
    let mut view = WorkspaceView::new();
    for e in events().into_iter().chain(more_session_events()) {
        view.apply(e, &resolve, now());
    }
    view
}
