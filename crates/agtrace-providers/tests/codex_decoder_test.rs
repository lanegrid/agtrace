//! Codex ≥ 0.153 (multi_agent v2) decoder: fixture snapshots and focused behaviour tests.

use agtrace_providers::{
    CodexProvider, DecodeOptions, DiscoveryScope, FileHeader, ParseDiagnostics, Provider,
    decode_file,
};
use agtrace_types::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const ROOT: &str = "01900000-0000-7000-8000-000000000001";
const JUDGE: &str = "01900000-0000-7000-8000-000000000002";
const FORK: &str = "01900000-0000-7000-8000-000000000003";

fn sessions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../agtrace-testing/fixtures/v2026_09/codex/home/sessions")
}

fn fixture(thread: &str) -> PathBuf {
    let dir = sessions_dir().join("2026/09/20");
    std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.to_string_lossy().ends_with(&format!("{thread}.jsonl")))
        .unwrap_or_else(|| panic!("fixture for {thread}"))
}

fn decode(path: &Path) -> (FileHeader, Vec<AgentEvent>, ParseDiagnostics) {
    decode_file(&CodexProvider, path, DecodeOptions::default()).expect("decodes")
}

fn payloads<'a, T>(
    events: &'a [AgentEvent],
    f: impl Fn(&'a EventPayload) -> Option<T> + 'a,
) -> Vec<T> {
    events.iter().filter_map(|e| f(&e.payload)).collect()
}

fn snapshot(name: &str, thread: &str) {
    let (_, events, diag) = decode(&fixture(thread));
    assert!(!diag.has_errors(), "{name}: {diag:?}");
    insta::assert_json_snapshot!(
        name,
        serde_json::json!({ "diagnostics": diag, "events": events })
    );
}

#[test]
fn snapshot_root_thread() {
    snapshot("codex_v2026_09_root", ROOT);
}

#[test]
fn snapshot_child_thread() {
    snapshot("codex_v2026_09_child", JUDGE);
}

#[test]
fn snapshot_fork_thread() {
    snapshot("codex_v2026_09_fork", FORK);
}

// ---------------------------------------------------------------------------
// Twins and usage
// ---------------------------------------------------------------------------

#[test]
fn twins_are_not_duplicated() {
    let (_, events, diag) = decode(&fixture(ROOT));
    let users = payloads(&events, |p| match p {
        EventPayload::User(u) => Some(u.text.clone()),
        _ => None,
    });
    // Injected context (environment / developer) is not a user prompt; the item_completed
    // UserMessage twin is ignored.
    assert_eq!(
        users,
        vec!["Review the parser and fix bugs.", "Stop, that's enough."]
    );
    let reasoning = payloads(&events, |p| match p {
        EventPayload::Reasoning(r) => Some(r.text.clone()),
        _ => None,
    });
    assert_eq!(reasoning, vec!["Plan the review."]);
    let messages = payloads(&events, |p| match p {
        EventPayload::Message(m) => Some((m.text.clone(), m.phase.clone())),
        _ => None,
    });
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].1.as_deref(), Some("commentary"));
    assert_eq!(messages[1].1.as_deref(), Some("final_answer"));
    for kind in [
        "event_msg/item_completed/Reasoning",
        "event_msg/item_completed/AgentMessage",
        "event_msg/item_completed/UserMessage",
        "event_msg/item_completed/ContextCompaction",
    ] {
        assert_eq!(diag.ignored_kinds.get(kind), Some(&1), "{kind}");
    }
}

#[test]
fn usage_comes_from_token_usage_record_only() {
    let (_, events, _) = decode(&fixture(ROOT));
    let usage: Vec<(Uuid, TokenUsagePayload)> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some((e.id, u.clone())),
            _ => None,
        })
        .collect();
    // 5 token_usage_record lines; token_count lines contribute no usage.
    assert_eq!(usage.len(), 5);
    let (id, first) = &usage[0];
    assert_eq!(first.dedupe_key.as_deref(), Some("resp_synthetic_1"));
    assert_eq!(first.input.uncached, 4000);
    assert_eq!(first.input.cache_read, 8000);
    assert_eq!(first.input.cache_write, 0);
    assert_eq!(first.output.generated, 200, "output minus reasoning");
    assert_eq!(first.output.reasoning, 100);
    assert_eq!(first.output.total(), 300);
    assert_eq!(first.model.as_deref(), Some("gpt-6-astra"));
    let session = Uuid::new_v5(&Uuid::NAMESPACE_OID, ROOT.as_bytes());
    assert_eq!(
        *id,
        Uuid::new_v5(&session, b"resp_synthetic_1:usage"),
        "deterministic id per response_id"
    );
}

fn write_rollout(dir: &Path, lines: &[String]) -> PathBuf {
    let path = dir.join(format!("rollout-2026-09-20T10-00-00-{ROOT}.jsonl"));
    let mut f = std::fs::File::create(&path).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
    path
}

fn meta_line(extra: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-09-20T10:00:00.000Z","type":"session_meta","ordinal":0,"payload":{{"id":"{ROOT}","session_id":"{ROOT}","cwd":"/work/demo-project","source":"cli"{extra}}}}}"#
    )
}

fn usage_line(ordinal: u64, response_id: &str, input: u64) -> String {
    format!(
        r#"{{"timestamp":"2026-09-20T10:00:01.000Z","type":"token_usage_record","ordinal":{ordinal},"payload":{{"response_id":"{response_id}","usage":{{"input_tokens":{input},"cached_input_tokens":0,"output_tokens":5,"reasoning_output_tokens":0,"total_tokens":{}}}}}}}"#,
        input + 5
    )
}

#[test]
fn usage_is_deduped_by_response_id_and_upserted_on_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_rollout(
        dir.path(),
        &[
            meta_line(""),
            usage_line(1, "resp_a", 100),
            usage_line(2, "resp_a", 100),
            usage_line(3, "resp_a", 150),
            usage_line(4, "resp_b", 10),
        ],
    );
    let (_, events, diag) = decode(&path);
    assert!(!diag.has_errors());
    let usage: Vec<(Uuid, u64)> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some((e.id, u.input.uncached)),
            _ => None,
        })
        .collect();
    assert_eq!(usage.len(), 3, "identical repeat is dropped");
    assert_eq!(usage[0].0, usage[1].0, "changed usage re-emits the same id");
    assert_eq!(usage[1].1, 150);
    assert_ne!(usage[1].0, usage[2].0);
}

// ---------------------------------------------------------------------------
// Fork prefix
// ---------------------------------------------------------------------------

#[test]
fn fork_prefix_is_skipped() {
    let (header, events, diag) = decode(&fixture(FORK));
    assert_eq!(header.agent.kind, AgentKind::Fork);
    // Lines 1..6 are the copied parent history (incl. the parent's session_meta).
    assert_eq!(diag.ignored_kinds.get("fork_prefix"), Some(&5));
    assert!(events.iter().all(|e| e.origin.line >= 6));
    assert!(
        !events.iter().any(|e| matches!(
            e.payload,
            EventPayload::User(_) | EventPayload::Compaction(_)
        )),
        "parent's prompt and compaction are not re-emitted"
    );
    let models = payloads(&events, |p| match p {
        EventPayload::ModelChange(m) => Some((m.to.clone(), m.source)),
        _ => None,
    });
    assert_eq!(
        models,
        vec![("gpt-6-astra".to_string(), ModelChangeSource::ThreadSettings)]
    );
}

#[test]
fn fork_prefix_without_ordinal_uses_line_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let strip = |s: String| {
        s.replace(r#","ordinal":0"#, "")
            .replace(r#","ordinal":1"#, "")
    };
    let path = write_rollout(
        dir.path(),
        &[
            strip(meta_line(
                r#","forked_from_id":"p","subagent_history_start_ordinal":2"#,
            )),
            strip(usage_line(1, "resp_parent", 100)),
            usage_line(2, "resp_own", 10).replace(r#","ordinal":2"#, ""),
        ],
    );
    let (_, events, diag) = decode(&path);
    assert_eq!(diag.ignored_kinds.get("fork_prefix"), Some(&1));
    let keys = payloads(&events, |p| match p {
        EventPayload::TokenUsage(u) => u.dedupe_key.clone(),
        _ => None,
    });
    assert_eq!(keys, vec!["resp_own"]);
}

// ---------------------------------------------------------------------------
// exec sub-actions
// ---------------------------------------------------------------------------

#[test]
fn exec_sub_actions_are_correlated() {
    let (_, events, _) = decode(&fixture(ROOT));
    let exec_calls: Vec<(Uuid, Option<String>)> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::ToolCall(ToolCallPayload::Execute {
                name, arguments, ..
            }) if name == "exec" => Some((e.id, arguments.command.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(exec_calls.len(), 2);
    assert_eq!(exec_calls[0].1.as_deref(), Some("ls src"));
    assert_eq!(exec_calls[1].1.as_deref(), Some("cargo test"));

    let subs = payloads(&events, |p| match p {
        EventPayload::ToolSubAction(s) => Some(s.clone()),
        _ => None,
    });
    // ls, 2 file changes, git status (written after the exec output), cargo test
    assert_eq!(subs.len(), 5);
    assert!(
        subs[..4]
            .iter()
            .all(|s| s.parent_tool_call_id == Some(exec_calls[0].0))
    );
    assert!(matches!(
        &subs[0].call,
        ToolCallPayload::Execute { arguments, .. } if arguments.command.as_deref() == Some("ls src")
    ));
    assert_eq!(subs[0].exit_code, Some(0));
    assert_eq!(subs[0].duration_ms, Some(120));
    assert!(
        matches!(&subs[1].call, ToolCallPayload::FileWrite { arguments, .. } if arguments.file_path.ends_with("new.rs"))
    );
    assert!(
        matches!(&subs[2].call, ToolCallPayload::FileEdit { arguments, .. } if arguments.file_path.ends_with("parser.rs"))
    );
    // "Script running with cell ID 3" keeps the second exec open until wait{cell_id:3} completes.
    assert!(matches!(
        &subs[3].call,
        ToolCallPayload::Execute { arguments, .. } if arguments.command.as_deref() == Some("git status --short")
    ));
    assert_eq!(subs[4].parent_tool_call_id, Some(exec_calls[1].0));
    assert_eq!(subs[4].status, SubActionStatus::Failed);
    assert_eq!(subs[4].exit_code, Some(101));
}

#[test]
fn child_sub_actions_cover_all_item_kinds() {
    let (_, events, _) = decode(&fixture(JUDGE));
    let subs = payloads(&events, |p| match p {
        EventPayload::ToolSubAction(s) => Some(s.clone()),
        _ => None,
    });
    let kinds: Vec<ToolKind> = subs.iter().map(|s| s.call.kind()).collect();
    assert_eq!(
        kinds,
        vec![
            ToolKind::Execute,
            ToolKind::Read,
            ToolKind::Search,
            ToolKind::Other
        ]
    );
    assert!(subs.iter().all(|s| s.parent_tool_call_id.is_some()));
    let mcp = &subs[3];
    assert_eq!(mcp.status, SubActionStatus::Failed);
    assert_eq!(mcp.call.name(), "mcp__node_repl__js");
    assert_eq!(mcp.duration_ms, Some(1000));
}

// ---------------------------------------------------------------------------
// Collaboration
// ---------------------------------------------------------------------------

#[test]
fn collaboration_calls_emit_agent_tool_calls_and_messages() {
    let (_, events, _) = decode(&fixture(ROOT));
    let agent_calls = payloads(&events, |p| match p {
        EventPayload::ToolCall(ToolCallPayload::Agent { arguments, .. }) => Some(arguments.op),
        _ => None,
    });
    assert_eq!(
        agent_calls,
        vec![
            AgentOp::Spawn,
            AgentOp::Spawn,
            AgentOp::Spawn,
            AgentOp::Send,
            AgentOp::Interrupt,
            AgentOp::Wait
        ]
    );

    let outgoing = payloads(&events, |p| match p {
        EventPayload::AgentMessage(m) if m.direction == MessageDirection::Outgoing => {
            Some(m.clone())
        }
        _ => None,
    });
    let routes: Vec<(String, AgentMessageKind, bool)> = outgoing
        .iter()
        .map(|m| {
            let to = match &m.to[..] {
                [AgentHandle::Path(p)] => p.clone(),
                other => panic!("unexpected {other:?}"),
            };
            (to, m.kind.clone(), m.encrypted)
        })
        .collect();
    assert_eq!(
        routes,
        vec![
            ("/root/judge".into(), AgentMessageKind::NewTask, true),
            ("/root/forker".into(), AgentMessageKind::NewTask, true),
            ("/root/extra".into(), AgentMessageKind::NewTask, true),
            ("/root/judge".into(), AgentMessageKind::Message, true),
            ("/root/forker".into(), AgentMessageKind::Interrupt, false),
        ]
    );
    assert!(
        outgoing
            .iter()
            .all(|m| m.body.is_none() && m.from == AgentHandle::Path("/root".into()))
    );

    // Failed spawn output is an error result.
    let errors = payloads(&events, |p| match p {
        EventPayload::ToolResult(r) if r.is_error => Some(r.output.clone()),
        _ => None,
    });
    assert_eq!(
        errors,
        vec!["collab spawn failed: agent thread limit reached"]
    );
}

#[test]
fn sub_agent_activity_yields_spawn_and_lifecycle() {
    let (_, events, _) = decode(&fixture(ROOT));
    let spawn_call_uuid = |call: &str| {
        events
            .iter()
            .find(|e| matches!(&e.payload, EventPayload::ToolCall(c) if c.provider_call_id() == Some(call)))
            .map(|e| e.id)
    };
    let spawns = payloads(&events, |p| match p {
        EventPayload::AgentSpawn(s) => Some(s.clone()),
        _ => None,
    });
    assert_eq!(spawns.len(), 2, "failed spawn has no SubAgentActivity");
    assert_eq!(
        spawns[0].child,
        AgentHandle::Id(AgentId::codex_thread(JUDGE))
    );
    assert_eq!(spawns[0].kind, AgentKind::CodexThread);
    assert_eq!(spawns[0].name.as_deref(), Some("judge"));
    assert_eq!(spawns[0].requested_model.as_deref(), Some("gpt-5.6-sol"));
    assert_eq!(spawns[0].spawn_call_id.as_deref(), Some("call_spawn_1"));
    assert_eq!(spawns[0].tool_call_id, spawn_call_uuid("call_spawn_1"));
    assert_eq!(spawns[1].kind, AgentKind::Fork, "fork_turns: all");
    assert_eq!(
        spawns[1].child,
        AgentHandle::Id(AgentId::codex_thread(FORK))
    );

    let lifecycle = payloads(&events, |p| match p {
        EventPayload::AgentLifecycle(l) => Some((l.target.clone(), l.transition)),
        _ => None,
    });
    let me = AgentHandle::Id(AgentId::codex_thread(ROOT));
    let judge = AgentHandle::Id(AgentId::codex_thread(JUDGE));
    let fork = AgentHandle::Id(AgentId::codex_thread(FORK));
    assert_eq!(
        lifecycle,
        vec![
            (me.clone(), LifecycleTransition::Running),
            (judge.clone(), LifecycleTransition::Running),
            (fork, LifecycleTransition::Interrupted),
            (judge, LifecycleTransition::Completed),
            (me, LifecycleTransition::Running),
        ]
    );
}

#[test]
fn received_messages_carry_trigger_turn_and_plaintext_final_answer() {
    let (_, root_events, _) = decode(&fixture(ROOT));
    let incoming = payloads(&root_events, |p| match p {
        EventPayload::AgentMessage(m) if m.direction == MessageDirection::Incoming => {
            Some(m.clone())
        }
        _ => None,
    });
    assert_eq!(incoming.len(), 1);
    let fa = &incoming[0];
    assert_eq!(fa.kind, AgentMessageKind::FinalAnswer);
    assert_eq!(fa.from, AgentHandle::Path("/root/judge".into()));
    assert_eq!(fa.to, vec![AgentHandle::Path("/root".into())]);
    assert_eq!(fa.body.as_deref(), Some("The parser looks correct."));
    assert!(!fa.encrypted);
    assert_eq!(fa.triggers_turn, Some(false));
    assert_eq!(fa.provider_message_id.as_deref(), Some("amsg_synthetic_3"));

    let (_, child_events, _) = decode(&fixture(JUDGE));
    let msgs = payloads(&child_events, |p| match p {
        EventPayload::AgentMessage(m) => Some(m.clone()),
        _ => None,
    });
    let summary: Vec<_> = msgs
        .iter()
        .map(|m| (m.direction, m.kind.clone(), m.encrypted, m.triggers_turn))
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                MessageDirection::Incoming,
                AgentMessageKind::NewTask,
                true,
                Some(true)
            ),
            (
                MessageDirection::Incoming,
                AgentMessageKind::Message,
                true,
                Some(false)
            ),
            // The child's task_complete delivers its FINAL_ANSWER to the parent.
            (
                MessageDirection::Outgoing,
                AgentMessageKind::FinalAnswer,
                false,
                None
            ),
        ]
    );
    assert_eq!(msgs[2].to, vec![AgentHandle::Path("/root".into())]);
    assert_eq!(msgs[2].body.as_deref(), Some("The parser looks correct."));
    assert!(msgs[0].body.is_none());
}

// ---------------------------------------------------------------------------
// Turns, model, context, compaction
// ---------------------------------------------------------------------------

#[test]
fn turns_models_context_and_compaction() {
    let (_, events, diag) = decode(&fixture(ROOT));
    let turn_ends = payloads(&events, |p| match p {
        EventPayload::TurnEnd(t) => Some((t.outcome.clone(), t.turn_id.clone(), t.duration_ms)),
        _ => None,
    });
    assert_eq!(
        turn_ends,
        vec![
            (TurnOutcome::Completed, Some("turn-r1".into()), Some(60000)),
            (TurnOutcome::Interrupted, Some("turn-r2".into()), Some(1500)),
        ]
    );

    let models = payloads(&events, |p| match p {
        EventPayload::ModelChange(m) => Some((m.from.clone(), m.to.clone(), m.source)),
        _ => None,
    });
    assert_eq!(
        models,
        vec![
            (None, "gpt-6-astra".into(), ModelChangeSource::TurnContext),
            (
                Some("gpt-6-astra".into()),
                "gpt-5.6-sol".into(),
                ModelChangeSource::ThreadSettings
            ),
        ]
    );

    // One explicit window hint (task_started); repeated values from token_count are dropped.
    let hints = payloads(&events, |p| match p {
        EventPayload::ContextWindowHint(h) => Some(h.clone()),
        _ => None,
    });
    assert_eq!(
        hints,
        vec![ContextWindowHintPayload::Explicit {
            tokens: 258_400,
            model: None
        }]
    );

    let compactions = payloads(&events, |p| match p {
        EventPayload::Compaction(c) => Some(c.clone()),
        _ => None,
    });
    assert_eq!(compactions.len(), 1);
    assert_eq!(compactions[0].trigger, CompactionTrigger::Auto);
    assert_eq!(compactions[0].pre_tokens, Some(240_000));
    assert_eq!(compactions[0].window_number, Some(1));

    assert_eq!(diag.ignored_kinds.get("world_state"), Some(&2));
    assert_eq!(
        diag.ignored_kinds.get("event_msg/thread_goal_updated"),
        None
    );
    assert!(diag.unknown_kinds.is_empty(), "{:?}", diag.unknown_kinds);

    // turn_context / thread_settings_applied carry the same effort: one attribute.
    let efforts = payloads(&events, |p| match p {
        EventPayload::AgentAttribute(a) if a.key == AgentAttributeKey::Effort => {
            Some(a.value.clone())
        }
        _ => None,
    });
    assert_eq!(efforts, vec!["medium".to_string()]);
    let goals = payloads(&events, |p| match p {
        EventPayload::Plan(g @ PlanPayload::Goal { .. }) => Some(g.clone()),
        _ => None,
    });
    assert_eq!(
        goals,
        vec![PlanPayload::Goal {
            objective: "synthetic".into(),
            status: Some("paused".into())
        }]
    );
    // The spawn call's requested effort reaches the AgentSpawn.
    let spawn_efforts = payloads(&events, |p| match p {
        EventPayload::AgentSpawn(s) => Some(s.requested_effort.clone()),
        _ => None,
    });
    assert_eq!(spawn_efforts.first(), Some(&Some("medium".to_string())));
}

/// Plan-mode `Plan` items and goal updates become typed plan events; effort
/// changes from turn_context and thread settings are attributes (on change).
#[test]
fn plan_goal_and_effort() {
    let dir = tempfile::tempdir().unwrap();
    let line = |ordinal: u64, sec: u32, body: &str| {
        format!(
            r#"{{"timestamp":"2026-09-20T10:00:{sec:02}.000Z","type":{body},"ordinal":{ordinal}}}"#
        )
    };
    let path = write_rollout(
        dir.path(),
        &[
            meta_line(""),
            line(
                1,
                1,
                r#""turn_context","payload":{"turn_id":"t1","model":"gpt-6-astra","effort":"low"}"#,
            ),
            line(
                2,
                2,
                r#""event_msg","payload":{"type":"thread_goal_updated","threadId":"x","goal":{"objective":"Ship the parser","status":"active"}}"#,
            ),
            line(
                3,
                3,
                r#""event_msg","payload":{"type":"item_completed","turn_id":"t1","item":{"type":"Plan","id":"t1-plan","text":"\n# Plan\n\n1. Read\n2. Fix\n"}}"#,
            ),
            line(
                4,
                4,
                r#""event_msg","payload":{"type":"thread_settings_applied","thread_id":"x","thread_settings":{"model":"gpt-6-astra","reasoning_effort":"high"}}"#,
            ),
            line(
                5,
                5,
                r#""turn_context","payload":{"turn_id":"t2","model":"gpt-6-astra","effort":"high"}"#,
            ),
            line(
                6,
                6,
                r#""event_msg","payload":{"type":"thread_goal_updated","threadId":"x","goal":{"objective":"   "}}"#,
            ),
        ],
    );
    let (_, events, diag) = decode(&path);
    assert!(!diag.has_errors(), "{diag:?}");
    let plans = payloads(&events, |p| match p {
        EventPayload::Plan(p) => Some(p.clone()),
        _ => None,
    });
    assert_eq!(
        plans,
        vec![
            PlanPayload::Goal {
                objective: "Ship the parser".into(),
                status: Some("active".into())
            },
            PlanPayload::Text {
                text: "# Plan\n\n1. Read\n2. Fix".into()
            },
        ]
    );
    let efforts = payloads(&events, |p| match p {
        EventPayload::AgentAttribute(a) if a.key == AgentAttributeKey::Effort => {
            Some(a.value.clone())
        }
        _ => None,
    });
    assert_eq!(efforts, vec!["low".to_string(), "high".to_string()]);
}

/// The FINAL_ANSWER is delivered before the turn ends: an event after the turn
/// end would read as new activity and turn the finished child back to Running.
#[test]
fn final_answer_precedes_the_turn_end() {
    let (_, events, _) = decode(&fixture(JUDGE));
    let fa = events
        .iter()
        .position(|e| {
            matches!(&e.payload, EventPayload::AgentMessage(m) if m.kind == AgentMessageKind::FinalAnswer)
        })
        .expect("final answer");
    assert!(
        matches!(
            events.get(fa + 1).map(|e| &e.payload),
            Some(EventPayload::TurnEnd(_))
        ),
        "{:?}",
        events.get(fa + 1).map(|e| &e.payload)
    );
}

#[test]
fn failed_task_complete_is_a_failed_turn() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_rollout(
        dir.path(),
        &[
            meta_line(""),
            r#"{"timestamp":"2026-09-20T10:00:05.000Z","type":"event_msg","ordinal":1,"payload":{"type":"task_complete","turn_id":"t1","duration_ms":10,"last_agent_message":null,"error":{"message":"server overloaded","codex_error_info":"server_overloaded"}}}"#.to_string(),
        ],
    );
    let (_, events, _) = decode(&path);
    let outcomes = payloads(&events, |p| match p {
        EventPayload::TurnEnd(t) => Some(t.outcome.clone()),
        _ => None,
    });
    assert_eq!(
        outcomes,
        vec![TurnOutcome::Failed {
            error: Some("server overloaded".into())
        }]
    );
}

// ---------------------------------------------------------------------------
// Leniency and discovery
// ---------------------------------------------------------------------------

#[test]
fn bad_lines_are_counted_and_never_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_rollout(
        dir.path(),
        &[
            meta_line(""),
            "{not json".to_string(),
            r#"{"timestamp":"2026-09-20T10:00:01.000Z","type":"brand_new_record","ordinal":2,"payload":{}}"#.to_string(),
            r#"{"timestamp":"2026-09-20T10:00:01.000Z","type":"event_msg","ordinal":3,"payload":{"type":"task_started","model_context_window":"big"}}"#.to_string(),
            r#"{"timestamp":"2026-09-20T10:00:01.000Z","type":"event_msg","ordinal":4,"payload":{"type":"item_completed","turn_id":"t","item":{"type":"BrandNewItem","id":"x"}}}"#.to_string(),
            usage_line(5, "resp_ok", 7),
        ],
    );
    let (_, events, diag) = decode(&path);
    assert_eq!(diag.lines, 6);
    assert_eq!(diag.invalid_json, 1);
    assert_eq!(diag.unknown_kinds.get("brand_new_record"), Some(&1));
    assert_eq!(
        diag.unknown_kinds
            .get("event_msg/item_completed/BrandNewItem"),
        Some(&1)
    );
    assert_eq!(diag.schema_mismatch.get("event_msg/task_started"), Some(&1));
    assert_eq!(diag.decoded, 4);
    assert_eq!(events.len(), 1, "good lines still decode");
    assert_eq!(events[0].origin.line, 5);
}

#[test]
fn discover_links_tree_and_reads_titles() {
    let mut headers = CodexProvider
        .discover(&DiscoveryScope {
            roots: Some(vec![sessions_dir()]),
            project_root: Some(PathBuf::from("/work/demo-project")),
        })
        .unwrap();
    headers.sort_by(|a, b| a.agent.id.cmp(&b.agent.id));
    assert_eq!(headers.len(), 3);
    let (root, judge, fork) = (&headers[0], &headers[1], &headers[2]);
    assert_eq!(root.agent.path.as_deref(), Some("/root"));
    assert_eq!(
        root.title.as_deref(),
        Some("Parser review and fixes"),
        "latest session_index line wins"
    );
    assert_eq!(judge.agent.kind, AgentKind::CodexThread);
    assert_eq!(judge.agent.parent.as_ref(), Some(&root.agent.id));
    assert_eq!(judge.title, None);
    assert_eq!(fork.agent.kind, AgentKind::Fork);
    assert_eq!(fork.agent.path.as_deref(), Some("/root/forker"));
    assert_eq!(fork.agent.root, root.agent.id);
}
