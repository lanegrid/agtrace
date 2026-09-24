//! Decoder snapshots of the synthetic Claude Code fixture tree (events + diagnostics
//! per agent file), plus semantic assertions on the multi-agent events.

use agtrace_providers::{
    ClaudeProvider, DecodeOptions, DiscoveryScope, FileHeader, Provider, decode_file,
};
use agtrace_types::*;
use serde_json::{Value, json};
use std::path::PathBuf;

fn projects_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../agtrace-testing/fixtures/v2026_09/claude/home/projects")
}

fn headers() -> Vec<FileHeader> {
    let mut headers = ClaudeProvider
        .discover(&DiscoveryScope {
            roots: Some(vec![projects_root()]),
            project_root: Some(PathBuf::from("/work/demo-project")),
        })
        .expect("discover");
    headers.sort_by(|a, b| a.agent.id.cmp(&b.agent.id));
    headers
}

fn decode(header: &FileHeader) -> (Vec<AgentEvent>, ParseDiagnostics) {
    let (_, events, diag) = decode_file(
        &ClaudeProvider,
        &header.agent.file,
        DecodeOptions::default(),
    )
    .expect("decode");
    (events, diag)
}

fn header_json(h: &FileHeader) -> Value {
    let a = &h.agent;
    json!({
        "id": a.id,
        "kind": a.kind,
        "root": a.root,
        "parent": a.parent,
        "native_session_id": a.native_session_id,
        "native_agent_id": a.native_agent_id,
        "name": a.name,
        "agent_type": a.agent_type,
        "team": a.team,
        "spawn_call_id": a.spawn_call_id,
        "depth": a.depth,
        "cwd": a.cwd,
        "started_at": a.started_at,
        "title": h.title,
    })
}

fn snapshot_name(h: &FileHeader) -> String {
    match h.agent.kind {
        AgentKind::Main => "claude_fixture_lead".to_string(),
        AgentKind::Teammate => "claude_fixture_teammate".to_string(),
        AgentKind::Subagent => "claude_fixture_subagent".to_string(),
        AgentKind::Fork => "claude_fixture_fork".to_string(),
        other => format!("claude_fixture_{other:?}"),
    }
}

#[test]
fn claude_fixture_snapshots() {
    let headers = headers();
    assert_eq!(headers.len(), 4, "lead, teammate, subagent, fork");
    for h in &headers {
        let (events, diagnostics) = decode(h);
        insta::assert_json_snapshot!(
            snapshot_name(h),
            json!({ "header": header_json(h), "diagnostics": diagnostics, "events": events })
        );
    }
}

fn find(headers: &[FileHeader], kind: AgentKind) -> &FileHeader {
    headers
        .iter()
        .find(|h| h.agent.kind == kind)
        .expect("agent kind")
}

fn payloads(events: &[AgentEvent]) -> Vec<&EventPayload> {
    events.iter().map(|e| &e.payload).collect()
}

#[test]
fn lead_multi_agent_events() {
    let headers = headers();
    let lead = find(&headers, AgentKind::Main);
    let (events, diag) = decode(lead);
    let p = payloads(&events);

    // Lenient lines are counted, never fatal.
    assert_eq!(diag.invalid_json, 1);
    assert_eq!(diag.unknown_kinds.get("future-record-kind"), Some(&1));
    assert_eq!(diag.schema_mismatch.get("assistant"), Some(&1));
    assert!(diag.ignored_kinds.contains_key("attachment/skill_listing"));
    assert_eq!(diag.ignored_kinds.get("mode"), Some(&3));

    // Spawns: teammate, async subagent, fork.
    let spawns: Vec<&AgentSpawnPayload> = p
        .iter()
        .filter_map(|p| match p {
            EventPayload::AgentSpawn(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(spawns.len(), 3);
    assert_eq!(
        spawns[0].child,
        AgentHandle::TeamMember {
            team: Some("session-00000001".into()),
            name: "audit-A".into()
        }
    );
    assert_eq!(spawns[0].kind, AgentKind::Teammate);
    assert_eq!(spawns[0].requested_model.as_deref(), Some("opus"));
    assert_eq!(
        spawns[1].child,
        AgentHandle::NativeAgentId("a0000000000000001".into())
    );
    assert_eq!(spawns[1].kind, AgentKind::Subagent);
    assert_eq!(
        spawns[1].resolved_model.as_deref(),
        Some("claude-opus-5-5[1m]")
    );
    assert_eq!(
        spawns[1].spawn_call_id.as_deref(),
        Some("toolu_synthetic_spawn_async")
    );
    assert!(spawns[1].tool_call_id.is_some());
    assert_eq!(spawns[2].kind, AgentKind::Fork);

    // Usage: one per message.id; msg_02 upserted (partial -> final).
    let usage: Vec<(&AgentEvent, &TokenUsagePayload)> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some((e, u)),
            _ => None,
        })
        .collect();
    let keys: Vec<_> = usage
        .iter()
        .map(|(_, u)| u.dedupe_key.clone().unwrap())
        .collect();
    assert_eq!(
        keys,
        vec![
            "msg_synthetic_01",
            "msg_synthetic_02",
            "msg_synthetic_02",
            "msg_synthetic_03",
            "msg_synthetic_04",
            "msg_synthetic_05",
            "msg_synthetic_06",
            "msg_synthetic_07",
        ]
    );
    assert_eq!(usage[1].0.id, usage[2].0.id);
    assert_eq!(usage[1].1.completeness, UsageCompleteness::PartialOutput);
    assert_eq!(usage[2].1.completeness, UsageCompleteness::Final);
    assert_eq!(usage[5].1.completeness, UsageCompleteness::PartialOutput);

    // Redacted thinking: one marker per message.
    let redacted = p
        .iter()
        .filter(|p| matches!(p, EventPayload::Reasoning(r) if r.text == "[thinking redacted]"))
        .count();
    assert_eq!(redacted, 2);

    // Teammate message batch: message + idle notification.
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentMessage(m) if m.kind == AgentMessageKind::Message
            && m.direction == MessageDirection::Incoming
            && m.summary.as_deref() == Some("found 3 bugs"))));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentLifecycle(l) if l.transition == LifecycleTransition::Idle)));

    // Fork task-notification delivered twice (queued_command + user record) -> once.
    let fork_done: Vec<_> = p
        .iter()
        .filter(|p| matches!(p,
            EventPayload::AgentLifecycle(l) if l.target == AgentHandle::NativeAgentId("a0000000000000002".into())))
        .collect();
    assert_eq!(fork_done.len(), 1);
    // Bash task-notification (queue-operation content is not decoded as a notification).
    assert_eq!(
        p.iter()
            .filter(|p| matches!(p, EventPayload::Notification(n) if n.kind.as_deref() == Some("task_notification")))
            .count(),
        1
    );
    // Hand-back -> message + Completed with usage.
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentLifecycle(l) if l.transition == LifecycleTransition::Completed
            && l.target == AgentHandle::NativeAgentId("a0000000000000001".into())
            && l.usage.as_ref().and_then(|u| u.total_tokens) == Some(4100))));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentMessage(m) if m.kind == AgentMessageKind::Handback
            && m.body.as_deref() == Some("There are 3 files in src."))));
    // SendMessage -> outgoing message; TaskStop on a teammate -> Killed.
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentMessage(m) if m.direction == MessageDirection::Outgoing
            && m.to == vec![AgentHandle::TeamMember { team: Some("session-00000001".into()), name: "audit-A".into() }])));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentLifecycle(l) if l.transition == LifecycleTransition::Killed)));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::AgentLifecycle(l) if l.transition == LifecycleTransition::AllBackgroundKilled)));

    // Turn ends, compaction, interrupt.
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::TurnEnd(t) if t.pending_background_agents == Some(2))));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::TurnEnd(t) if t.outcome == TurnOutcome::Interrupted)));
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::Compaction(c) if c.trigger == CompactionTrigger::Auto
            && c.pre_tokens == Some(973_000) && c.post_tokens == Some(41_000))));

    // Model changes: attachment (opus-5), /model (opus-5-5, 1M), no duplicate from the next message.
    let models: Vec<_> = p
        .iter()
        .filter_map(|p| match p {
            EventPayload::ModelChange(m) => Some((m.to.as_str(), m.source)),
            _ => None,
        })
        .collect();
    assert_eq!(
        models,
        vec![
            ("claude-opus-5", ModelChangeSource::Attachment),
            ("claude-opus-5-5", ModelChangeSource::LocalCommand),
        ]
    );
    // Extended-context evidence: spawn resolvedModel [1m], /model "(1M context)", cost-state [1m]
    // (the latter two for different ids).
    let markers: Vec<_> = p
        .iter()
        .filter_map(|p| match p {
            EventPayload::ContextWindowHint(ContextWindowHintPayload::ExtendedMarker {
                model,
                ..
            }) => Some(model.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(markers, vec!["claude-opus-5-5[1m]", "claude-opus-5-5"]);

    // Attributes only on change: permission mode, title, title change, agent name.
    let attrs: Vec<_> = p
        .iter()
        .filter_map(|p| match p {
            EventPayload::AgentAttribute(a) => Some((a.key, a.value.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        attrs,
        vec![
            (AgentAttributeKey::PermissionMode, "auto"),
            (AgentAttributeKey::Title, "Demo project audit"),
            (AgentAttributeKey::Title, "Demo project audit and fixes"),
            (AgentAttributeKey::AgentName, "demo-lead"),
        ]
    );

    // Array tool_result flattened; compact summary ignored.
    assert!(p.iter().any(|p| matches!(p,
        EventPayload::ToolResult(r) if r.output == "# Demo\n[image]\n[tool_reference Read]")));
    assert!(!p.iter().any(|p| matches!(p,
        EventPayload::User(u) if u.text.contains("being continued"))));

    // Records without a timestamp inherit the previous one; events are in file order.
    assert!(events.windows(2).all(|w| w[0].origin < w[1].origin));
    assert!(events.windows(2).all(|w| w[1].timestamp >= w[0].timestamp));
}

#[test]
fn teammate_events() {
    let headers = headers();
    let mate = find(&headers, AgentKind::Teammate);
    assert_eq!(mate.agent.name.as_deref(), Some("audit-A"));
    assert_eq!(mate.agent.team.as_deref(), Some("session-00000001"));
    assert_eq!(mate.agent.agent_type.as_deref(), Some("general-purpose"));
    let (events, diag) = decode(mate);
    assert!(!diag.has_errors(), "{diag:?}");
    let msgs: Vec<&AgentMessagePayload> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::AgentMessage(m) => Some(m),
            _ => None,
        })
        .collect();
    let lead = AgentHandle::TeamMember {
        team: Some("session-00000001".into()),
        name: "team-lead".into(),
    };
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].kind, AgentMessageKind::NewTask);
    assert_eq!(msgs[0].from, lead);
    assert_eq!(msgs[0].body.as_deref(), Some("Review the parser module."));
    assert_eq!(msgs[1].kind, AgentMessageKind::Message);
    assert_eq!(msgs[2].direction, MessageDirection::Outgoing);
    assert_eq!(msgs[2].to, vec![lead]);
}

#[test]
fn subagent_and_fork_headers() {
    let headers = headers();
    let sub = find(&headers, AgentKind::Subagent);
    assert_eq!(
        sub.agent.spawn_call_id.as_deref(),
        Some("toolu_synthetic_spawn_async")
    );
    assert_eq!(sub.agent.agent_type.as_deref(), Some("general-purpose"));
    assert_eq!(sub.agent.name.as_deref(), Some("Count source files"));
    let fork = find(&headers, AgentKind::Fork);
    assert_eq!(fork.agent.name.as_deref(), Some("docs-fork"));
    assert_eq!(
        fork.agent.parent,
        Some(AgentId::claude_session(
            "00000000-0000-4000-8000-000000000001"
        ))
    );

    // The fork's first record answers the parent's Agent call: no ToolResult (unknown call),
    // boilerplate ignored.
    let (events, diag) = decode(fork);
    assert!(!diag.has_errors(), "{diag:?}");
    assert!(!events.iter().any(|e| matches!(
        e.payload,
        EventPayload::ToolResult(_) | EventPayload::User(_)
    )));

    // Stream-start subagent usage is PartialOutput; SubagentHandback is an Agent tool call.
    let (events, _) = decode(sub);
    assert!(events.iter().all(|e| match &e.payload {
        EventPayload::TokenUsage(u) => u.completeness == UsageCompleteness::PartialOutput,
        _ => true,
    }));
    assert!(events.iter().any(|e| matches!(&e.payload,
        EventPayload::ToolCall(ToolCallPayload::Agent { arguments, .. }) if arguments.op == AgentOp::Handback)));
}
