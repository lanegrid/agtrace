//! Lenient per-line decoding: bad lines are counted in diagnostics and never fail a file.

use agtrace_providers::{ClaudeProvider, CodexProvider, DecodeOptions, Provider, decode_file};
use agtrace_types::EventPayload;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_file(dir: &Path, rel: &str, lines: &[&str]) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&path).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
    path
}

const CLAUDE_USER: &str = r#"{"type":"user","uuid":"00000000-0000-4000-8000-0000000000a1","parentUuid":null,"sessionId":"00000000-0000-4000-8000-000000000001","timestamp":"2026-09-20T10:00:00Z","cwd":"/work/demo-project","message":{"role":"user","content":"hello"}}"#;
const CLAUDE_ASSISTANT: &str = r#"{"type":"assistant","uuid":"00000000-0000-4000-8000-0000000000a2","parentUuid":null,"sessionId":"00000000-0000-4000-8000-000000000001","timestamp":"2026-09-20T10:00:01Z","message":{"type":"message","id":"msg_synthetic_1","role":"assistant","model":"claude-opus-5-5","content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}}}"#;
// Known kind, wrong shape: `message` must be an object.
const CLAUDE_MISMATCH: &str = r#"{"type":"assistant","uuid":"x","sessionId":"00000000-0000-4000-8000-000000000001","timestamp":"2026-09-20T10:00:02Z","message":42}"#;

#[test]
fn claude_bad_lines_never_fail_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "proj/00000000-0000-4000-8000-000000000001.jsonl",
        &[
            CLAUDE_USER,
            "{this is not json",
            r#"{"type":"totally-new-kind","sessionId":"00000000-0000-4000-8000-000000000001"}"#,
            CLAUDE_MISMATCH,
            CLAUDE_ASSISTANT,
        ],
    );

    let (header, events, diag) =
        decode_file(&ClaudeProvider, &path, DecodeOptions::default()).expect("file decodes");

    assert_eq!(
        header.agent.id.as_str(),
        "claude:00000000-0000-4000-8000-000000000001"
    );
    assert_eq!(diag.lines, 5);
    assert_eq!(diag.invalid_json, 1);
    assert_eq!(diag.unknown_kinds.get("totally-new-kind"), Some(&1));
    assert_eq!(diag.schema_mismatch.get("assistant"), Some(&1));
    assert_eq!(diag.samples.len(), 2);

    // Good lines still produce events, positioned by file line.
    assert!(matches!(events[0].payload, EventPayload::User(_)));
    assert_eq!(events[0].origin.line, 0);
    let usage = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some((e.origin.line, u.clone())),
            _ => None,
        })
        .expect("usage event");
    assert_eq!(usage.0, 4);
    assert_eq!(usage.1.input.uncached, 10);
    assert_eq!(usage.1.input.cache_read, 100);
    assert_eq!(usage.1.input.cache_write, 20);
    assert_eq!(usage.1.model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(usage.1.dedupe_key.as_deref(), Some("msg_synthetic_1"));
    assert!(events.iter().all(|e| e.agent == header.agent.id));
}

const CODEX_META: &str = r#"{"timestamp":"2026-09-20T10:00:00Z","type":"session_meta","payload":{"id":"01900000-0000-7000-8000-000000000002","session_id":"01900000-0000-7000-8000-000000000001","timestamp":"2026-09-20T10:00:00Z","cwd":"/work/demo-project","originator":"codex_cli_rs","cli_version":"0.153.0","source":{"subagent":{"thread_spawn":{"parent_thread_id":"01900000-0000-7000-8000-000000000001","depth":1,"agent_path":"/root/judge","agent_nickname":"Judge","agent_role":null}}}}}"#;
const CODEX_TURN: &str = r#"{"timestamp":"2026-09-20T10:00:01Z","type":"turn_context","payload":{"cwd":"/work/demo-project","model":"gpt-5.6-sol"}}"#;
const CODEX_CALL: &str = r#"{"timestamp":"2026-09-20T10:00:02Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"ls\"]}","call_id":"call_synthetic_1"}}"#;
const CODEX_ARRAY_OUTPUT: &str = r#"{"timestamp":"2026-09-20T10:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call_synthetic_1","output":[{"type":"input_text","text":"file-a"},{"type":"input_text","text":"file-b"}]}}"#;
const CODEX_TOKENS_NULL_WINDOW: &str = r#"{"timestamp":"2026-09-20T10:00:04Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":120,"cached_input_tokens":100,"output_tokens":7,"reasoning_output_tokens":2,"total_tokens":127},"last_token_usage":{"input_tokens":120,"cached_input_tokens":100,"output_tokens":7,"reasoning_output_tokens":2,"total_tokens":127},"model_context_window":null}}}"#;
const CODEX_TOKENS_WINDOW: &str = r#"{"timestamp":"2026-09-20T10:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":130,"cached_input_tokens":100,"output_tokens":8,"total_tokens":138},"model_context_window":258400}}}"#;
// Known kind, wrong shape: function_call without call_id.
const CODEX_MISMATCH: &str = r#"{"timestamp":"2026-09-20T10:00:06Z","type":"response_item","payload":{"type":"function_call","name":"shell"}}"#;

#[test]
fn codex_thread_spawn_file_decodes_with_serde_fixes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "sessions/2026/09/20/rollout-2026-09-20T10-00-00-01900000-0000-7000-8000-000000000002.jsonl",
        &[
            CODEX_META,
            CODEX_TURN,
            CODEX_CALL,
            CODEX_ARRAY_OUTPUT,
            CODEX_TOKENS_NULL_WINDOW,
            CODEX_TOKENS_WINDOW,
            "not json at all",
            r#"{"timestamp":"2026-09-20T10:00:07Z","type":"brand_new_record","payload":{}}"#,
            CODEX_MISMATCH,
        ],
    );
    assert!(CodexProvider.probe(&path));

    let (header, events, diag) =
        decode_file(&CodexProvider, &path, DecodeOptions::default()).expect("file decodes");

    assert_eq!(
        header.agent.id.as_str(),
        "codex:01900000-0000-7000-8000-000000000002"
    );
    assert_eq!(
        header.agent.parent.as_ref().map(|p| p.as_str()),
        Some("codex:01900000-0000-7000-8000-000000000001")
    );
    assert_eq!(diag.invalid_json, 1);
    assert_eq!(diag.unknown_kinds.get("brand_new_record"), Some(&1));
    assert_eq!(
        diag.schema_mismatch.get("response_item/function_call"),
        Some(&1)
    );

    let output = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::ToolResult(r) => Some(r.output.clone()),
            _ => None,
        })
        .expect("array output decoded");
    assert_eq!(output, "file-a\nfile-b");

    let usages: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::TokenUsage(u) => Some(u.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(usages.len(), 2);
    assert_eq!(usages[0].input.uncached, 20);
    assert_eq!(usages[0].input.cache_read, 100);
    assert_eq!(usages[0].model.as_deref(), Some("gpt-5.6-sol"));

    let hints: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::ContextWindowHint(h) => Some(h.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        hints,
        vec![agtrace_types::ContextWindowHintPayload::Explicit {
            tokens: 258_400,
            model: Some("gpt-5.6-sol".to_string())
        }]
    );
}

#[test]
fn file_without_newline_at_eof_is_decoded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proj/x.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, CLAUDE_USER).unwrap();
    let (_, events, diag) =
        decode_file(&ClaudeProvider, &path, DecodeOptions::default()).expect("file decodes");
    assert_eq!(events.len(), 1);
    assert!(!diag.has_errors());
}
