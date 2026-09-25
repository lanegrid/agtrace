//! Focused Claude Code decoder tests (bug classes of the 2.1.24x+ format).

use agtrace_providers::{ClaudeProvider, DecodeOptions, decode_file};
use agtrace_types::{AgentEvent, EventPayload, ParseDiagnostics, UsageCompleteness};
use std::io::Write;
use uuid::Uuid;

const SID: &str = "00000000-0000-4000-8000-000000000001";

fn decode_lines(lines: &[String]) -> (Vec<AgentEvent>, ParseDiagnostics) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("-work-demo-project/{SID}.jsonl"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&path).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
    drop(f);
    let (_, events, diag) =
        decode_file(&ClaudeProvider, &path, DecodeOptions::default()).expect("decodes");
    (events, diag)
}

/// Envelope of a transcript record (uuid, session, timestamp) around `body`.
fn rec(uuid_n: u32, ts_sec: u32, body: &str) -> String {
    format!(
        r#"{{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/work/demo-project","sessionId":"{SID}","version":"2.1.281","uuid":"00000000-0000-4000-8000-{uuid_n:012}","timestamp":"2026-09-20T10:00:{ts_sec:02}.000Z",{body}}}"#
    )
}

fn usage_json(output: u64, final_mode: bool) -> String {
    if final_mode {
        format!(
            r#"{{"input_tokens":10,"cache_creation_input_tokens":200,"cache_read_input_tokens":5000,"output_tokens":{output},"output_tokens_details":{{"thinking_tokens":0}},"iterations":[{{"type":"message","input_tokens":10,"output_tokens":{output},"cache_read_input_tokens":5000,"cache_creation_input_tokens":200}}]}}"#
        )
    } else {
        format!(
            r#"{{"input_tokens":10,"cache_creation_input_tokens":200,"cache_read_input_tokens":5000,"output_tokens":{output}}}"#
        )
    }
}

fn assistant(uuid_n: u32, msg_id: &str, block: &str, stop: Option<&str>, usage: &str) -> String {
    let stop = stop.map(|s| format!("\"{s}\"")).unwrap_or("null".into());
    rec(
        uuid_n,
        uuid_n,
        &format!(
            r#""type":"assistant","requestId":"req_synthetic","message":{{"id":"{msg_id}","type":"message","role":"assistant","model":"claude-opus-5-5","content":[{block}],"stop_reason":{stop},"stop_sequence":null,"usage":{usage}}}"#
        ),
    )
}

fn usages(events: &[AgentEvent]) -> Vec<(&AgentEvent, &agtrace_types::TokenUsagePayload)> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some((e, u)),
            _ => None,
        })
        .collect()
}

#[test]
fn split_records_emit_one_usage_per_message_id() {
    let u = usage_json(120, true);
    let lines = vec![
        assistant(
            1,
            "msg_synthetic_1",
            r#"{"type":"thinking","thinking":"","signature":"sig"}"#,
            Some("tool_use"),
            &u,
        ),
        assistant(
            2,
            "msg_synthetic_1",
            r#"{"type":"text","text":"Working on it."}"#,
            Some("tool_use"),
            &u,
        ),
        assistant(
            3,
            "msg_synthetic_1",
            r#"{"type":"tool_use","id":"toolu_synthetic_1","name":"Bash","input":{"command":"ls"}}"#,
            Some("tool_use"),
            &u,
        ),
    ];
    let (events, _) = decode_lines(&lines);
    let usages = usages(&events);
    assert_eq!(usages.len(), 1, "usage must be counted once per message.id");
    let (event, usage) = usages[0];
    let session_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, SID.as_bytes());
    assert_eq!(
        event.id,
        Uuid::new_v5(&session_uuid, b"msg_synthetic_1:usage"),
        "usage id is derived from the dedupe key"
    );
    assert_eq!(usage.dedupe_key.as_deref(), Some("msg_synthetic_1"));
    assert_eq!(usage.completeness, UsageCompleteness::Final);
    assert_eq!(usage.input.total(), 5210);
    assert_eq!(usage.output.total(), 120);
}

#[test]
fn changed_usage_for_same_message_is_upserted() {
    let lines = vec![
        // stream-start write mode: stop_reason null, no iterations, output is a snapshot
        assistant(
            1,
            "msg_synthetic_2",
            r#"{"type":"thinking","thinking":"","signature":"sig"}"#,
            None,
            &usage_json(3, false),
        ),
        assistant(
            2,
            "msg_synthetic_2",
            r#"{"type":"text","text":"Done."}"#,
            Some("end_turn"),
            &usage_json(90, true),
        ),
    ];
    let (events, _) = decode_lines(&lines);
    let usages = usages(&events);
    assert_eq!(usages.len(), 2, "a changed usage is re-emitted");
    assert_eq!(
        usages[0].0.id, usages[1].0.id,
        "re-emitted with the same id (upsert)"
    );
    assert_eq!(usages[0].1.completeness, UsageCompleteness::PartialOutput);
    assert_eq!(usages[1].1.completeness, UsageCompleteness::Final);
    assert_eq!(usages[1].1.output.total(), 90);
}

#[test]
fn stream_start_usage_is_partial_output() {
    let lines = vec![assistant(
        1,
        "msg_synthetic_3",
        r#"{"type":"text","text":"hi"}"#,
        None,
        &usage_json(2, false),
    )];
    let (events, _) = decode_lines(&lines);
    let usages = usages(&events);
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].1.completeness, UsageCompleteness::PartialOutput);
    assert_eq!(usages[0].1.input.total(), 5210, "input and cache are exact");
}

#[test]
fn array_tool_result_content_is_flattened() {
    let lines = vec![
        assistant(
            1,
            "msg_synthetic_4",
            r#"{"type":"tool_use","id":"toolu_synthetic_4","name":"Read","input":{"file_path":"/work/demo-project/README.md"}}"#,
            Some("tool_use"),
            &usage_json(5, true),
        ),
        rec(
            2,
            2,
            r#""type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_synthetic_4","content":[{"type":"text","text":"line one"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"AAAA"}},{"type":"tool_reference","tool_name":"Read"},{"type":"text","text":"line two"}]}]}"#,
        ),
    ];
    let (events, _) = decode_lines(&lines);
    let output = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::ToolResult(r) => Some(r.output.clone()),
            _ => None,
        })
        .expect("tool result");
    assert_eq!(output, "line one\n[image]\n[tool_reference Read]\nline two");
}

#[test]
fn cost_state_extended_model_keys_are_markers() {
    let lines = vec![
        r#"{"type":"cost-state","totalCostUSD":1.0,"modelUsage":{"claude-opus-5":{"inputTokens":1},"claude-opus-5-5[1m]":{"inputTokens":2}},"sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
        r#"{"type":"cost-state","totalCostUSD":2.0,"modelUsage":{"claude-opus-5-5[1m]":{"inputTokens":3}},"sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
    ];
    let (events, diag) = decode_lines(&lines);
    assert!(!diag.has_errors(), "{diag:?}");
    let markers: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::ContextWindowHint(
                agtrace_types::ContextWindowHintPayload::ExtendedMarker { model, .. },
            ) => Some(model.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(markers, vec!["claude-opus-5-5[1m]"], "once per key");
}

#[test]
fn suffixed_attachment_model_does_not_flap_and_markers_dedupe_by_base_id() {
    let lines = vec![
        rec(
            1,
            1,
            r#""type":"attachment","attachment":{"type":"model","identity":{"modelId":"claude-opus-5-5[1m]","marketingName":"Opus 5.5 (1M context)"}}"#,
        ),
        assistant(2, "msg_synthetic_5", r#"{"type":"text","text":"hi"}"#, Some("end_turn"), &usage_json(2, true)),
        r#"{"type":"cost-state","modelUsage":{"claude-opus-5-5[1m]":{}},"sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
        rec(3, 3, r#""type":"user","message":{"role":"user","content":"<local-command-stdout>Set model to `Opus 5.5 (1M context) (default)`</local-command-stdout>"}"#),
    ];
    let (events, _) = decode_lines(&lines);
    let changes: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::ModelChange(m) => Some(m.to.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        changes,
        vec!["claude-opus-5-5"],
        "suffix variants are the same model"
    );
    let markers = events
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::ContextWindowHint(_)))
        .count();
    assert_eq!(markers, 1);
}

fn payloads_of(events: &[AgentEvent]) -> Vec<&EventPayload> {
    events.iter().map(|e| &e.payload).collect()
}

#[test]
fn records_without_timestamp_inherit_the_last_seen_one() {
    let lines = vec![
        r#"{"type":"permission-mode","permissionMode":"auto","sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
        rec(1, 5, r#""type":"user","message":{"role":"user","content":"hello"}"#),
        r#"{"type":"agent-name","agentName":"demo","sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
        // unparsable timestamp: inherits as well
        rec(2, 6, r#""type":"system","subtype":"turn_duration","durationMs":5"#).replace("2026-09-20T10:00:06.000Z", "not-a-time"),
    ];
    let (events, _) = decode_lines(&lines);
    let ts: Vec<_> = events.iter().map(|e| e.timestamp.to_rfc3339()).collect();
    // First state record: fallback = header start (first timestamped record).
    assert_eq!(
        ts,
        vec![
            "2026-09-20T10:00:05+00:00",
            "2026-09-20T10:00:05+00:00",
            "2026-09-20T10:00:05+00:00",
            "2026-09-20T10:00:05+00:00",
        ]
    );
}

#[test]
fn system_and_queue_records() {
    let lines = vec![
        rec(1, 1, r#""type":"system","subtype":"api_error","retryAttempt":2,"maxRetries":10,"level":"error""#),
        rec(2, 2, r#""type":"system","subtype":"informational","content":"heads up","level":"warning""#),
        rec(3, 3, r#""type":"system","subtype":"local_command","content":"<local-command-stdout></local-command-stdout>","commandRun":{"command":"context","args":""}"#),
        rec(4, 4, r#""type":"system","subtype":"local_command","content":"<command-name>/resume</command-name>\n<command-message>resume</command-message>\n<command-args></command-args>""#),
        rec(5, 5, r#""type":"system","subtype":"brand_new_subtype""#),
        rec(6, 6, r#""type":"attachment","attachment":{"type":"queued_command","prompt":"do the thing","commandMode":"prompt","origin":{"kind":"human"}}"#),
        r#"{"type":"queue-operation","operation":"remove","reason":"absorbed_mid_turn","content":"do the thing","timestamp":"2026-09-20T10:00:07.000Z","sessionId":"00000000-0000-4000-8000-000000000001"}"#.to_string(),
        rec(8, 8, r#""type":"user","isMeta":true,"message":{"role":"user","content":"<system-reminder>ignore me</system-reminder>"}"#),
        rec(9, 9, r#""type":"user","message":{"role":"user","content":"<local-command-caveat>Caveat</local-command-caveat>"}"#),
    ];
    let (events, diag) = decode_lines(&lines);
    let p = payloads_of(&events);
    assert!(
        matches!(p[0], EventPayload::Notification(n) if n.text == "API error (retry 2/10)" && n.kind.as_deref() == Some("api_error"))
    );
    assert!(
        matches!(p[1], EventPayload::Notification(n) if n.text == "heads up" && n.level.as_deref() == Some("warning"))
    );
    assert!(
        matches!(p[2], EventPayload::SlashCommand(c) if c.name == "/context" && c.args.is_none())
    );
    assert!(matches!(p[3], EventPayload::SlashCommand(c) if c.name == "/resume"));
    assert!(
        matches!(p[4], EventPayload::Notification(n) if n.text == "Queued prompt: do the thing")
    );
    assert!(
        matches!(p[5], EventPayload::QueueOperation(q) if q.reason.as_deref() == Some("absorbed_mid_turn"))
    );
    assert_eq!(p.len(), 6, "meta / caveat user records are ignored");
    assert_eq!(diag.unknown_kinds.get("system/brand_new_subtype"), Some(&1));
}

/// `SendMessage{to: <subagent id>}` resumes a background subagent: the recipient is
/// that agent (`NativeAgentId`), not a team member named like the id.
#[test]
fn send_message_to_a_subagent_id_targets_the_subagent() {
    let lines = vec![
        assistant(
            1,
            "msg_synthetic_1",
            r#"{"type":"tool_use","id":"toolu_synthetic_send","name":"SendMessage","input":{"to":"a0123456789abcdef","message":"Continue with part 2."}}"#,
            Some("tool_use"),
            &usage_json(10, true),
        ),
        rec(
            2,
            2,
            r#""type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_synthetic_send","content":"Message sent","is_error":false}]},"toolUseResult":{"success":true,"message":"Message sent"}"#,
        ),
    ];
    let (events, _) = decode_lines(&lines);
    let to: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::AgentMessage(m) => Some(m.to.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        to,
        vec![vec![agtrace_types::AgentHandle::NativeAgentId(
            "a0123456789abcdef".into()
        )]]
    );
}

/// A resumed / respawned process writes into the same transcript under a new
/// runtime `session_id`; each distinct one (other than the transcript id) is
/// surfaced once as a `RuntimeSessionId` alias so team configs naming it resolve.
#[test]
fn runtime_session_ids_are_surfaced_as_aliases() {
    const RUNTIME_A: &str = "00000000-0000-4000-8000-0000000000aa";
    const RUNTIME_B: &str = "00000000-0000-4000-8000-0000000000bb";
    let user = |n: u32, runtime: &str| {
        rec(
            n,
            n,
            &format!(
                r#""session_id":"{runtime}","type":"user","message":{{"role":"user","content":"hi"}}"#
            ),
        )
    };
    let lines = vec![
        user(1, SID),
        user(2, RUNTIME_A),
        user(3, RUNTIME_A),
        user(4, RUNTIME_B),
    ];
    let (events, _) = decode_lines(&lines);
    let aliases: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::AgentAttribute(a)
                if a.key == agtrace_types::AgentAttributeKey::RuntimeSessionId =>
            {
                Some(a.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(aliases, vec![RUNTIME_A, RUNTIME_B]);
    let ids: std::collections::HashSet<Uuid> = events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), events.len(), "event ids stay unique");
}

fn plans(events: &[AgentEvent]) -> Vec<&agtrace_types::PlanPayload> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Plan(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// The record envelope's `effort` (or `perTurnEffort`) becomes an `Effort`
/// attribute, emitted when it changes (also back to an earlier value).
#[test]
fn effort_is_an_attribute_emitted_on_change() {
    let with_effort = |n: u32, extra: &str| {
        let line = assistant(
            n,
            &format!("msg_synthetic_{n}"),
            r#"{"type":"text","text":"ok"}"#,
            Some("end_turn"),
            &usage_json(1, true),
        );
        line.replacen(
            r#""type":"assistant","#,
            &format!(r#""type":"assistant",{extra}"#),
            1,
        )
    };
    let lines = vec![
        with_effort(1, r#""effort":"high","#),
        with_effort(2, r#""effort":"high","#),
        with_effort(3, r#""effort":"medium","perTurnEffort":"medium","#),
        with_effort(4, ""),
        with_effort(5, r#""effort":"high","#),
    ];
    let (events, diag) = decode_lines(&lines);
    assert_eq!(diag.error_count(), 0);
    let efforts: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::AgentAttribute(a)
                if a.key == agtrace_types::AgentAttributeKey::Effort =>
            {
                Some(a.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(efforts, vec!["high", "medium", "high"]);
    let ids: std::collections::HashSet<Uuid> = events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), events.len(), "event ids stay unique");
}

/// `TaskCreate` (id from its result) and `TaskUpdate` become typed plan events;
/// a failed update and a legacy `TodoWrite` list are handled too.
#[test]
fn task_tools_become_plan_events() {
    use agtrace_types::{PlanItem, PlanItemStatus, PlanPayload};
    let call = |n: u32, id: &str, name: &str, input: &str| {
        assistant(
            n,
            &format!("msg_synthetic_{n}"),
            &format!(r#"{{"type":"tool_use","id":"{id}","name":"{name}","input":{input}}}"#),
            Some("tool_use"),
            &usage_json(1, true),
        )
    };
    let result = |n: u32, id: &str, tur: &str| {
        rec(
            n,
            n,
            &format!(
                r#""type":"user","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"{id}","content":"ok"}}]}},"toolUseResult":{tur}"#
            ),
        )
    };
    let lines = vec![
        call(
            1,
            "toolu_c1",
            "TaskCreate",
            r#"{"subject":"Run the tests","description":"All of them.","activeForm":"Running the tests"}"#,
        ),
        result(
            2,
            "toolu_c1",
            r#"{"task":{"id":"1","subject":"Run the tests"}}"#,
        ),
        call(
            3,
            "toolu_u1",
            "TaskUpdate",
            r#"{"taskId":"1","status":"in_progress"}"#,
        ),
        result(
            4,
            "toolu_u1",
            r#"{"success":true,"taskId":"1","updatedFields":["status"],"statusChange":{"from":"pending","to":"in_progress"}}"#,
        ),
        call(
            5,
            "toolu_u2",
            "TaskUpdate",
            r#"{"taskId":"9","status":"completed"}"#,
        ),
        result(
            6,
            "toolu_u2",
            r#"{"success":false,"error":"Task not found"}"#,
        ),
        call(
            7,
            "toolu_t1",
            "TodoWrite",
            r#"{"todos":[{"content":"Read","activeForm":"Reading","status":"completed"},{"content":"Write","status":"in_progress"}]}"#,
        ),
    ];
    let (events, diag) = decode_lines(&lines);
    assert_eq!(diag.error_count(), 0);
    let got: Vec<PlanPayload> = plans(&events).into_iter().cloned().collect();
    assert_eq!(
        got,
        vec![
            PlanPayload::TaskCreated {
                item: PlanItem {
                    id: Some("1".into()),
                    subject: "Run the tests".into(),
                    active_form: Some("Running the tests".into()),
                    status: PlanItemStatus::Pending,
                },
                description: Some("All of them.".into()),
                team: None,
            },
            PlanPayload::TaskUpdated {
                id: "1".into(),
                status: Some(PlanItemStatus::InProgress),
                subject: None,
                active_form: None,
                team: None,
            },
            PlanPayload::Items {
                items: vec![
                    PlanItem {
                        id: None,
                        subject: "Read".into(),
                        active_form: Some("Reading".into()),
                        status: PlanItemStatus::Completed,
                    },
                    PlanItem {
                        id: None,
                        subject: "Write".into(),
                        active_form: None,
                        status: PlanItemStatus::InProgress,
                    },
                ],
            },
        ]
    );
    let ids: std::collections::HashSet<Uuid> = events.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), events.len(), "event ids stay unique");
}
