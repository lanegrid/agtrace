// Integration test for live subagent discovery in SessionStreamer.
//
// Claude Code writes subagent (sidechain) transcripts to
// `{log_root}/{session_id}/subagents/agent-*.jsonl`. These files are created
// *after* a Task tool invocation, i.e. typically after `agtrace watch` has
// already attached to the session. The streamer must pick them up dynamically
// instead of only tracking the files that existed at attach time.
use std::sync::Arc;
use std::time::{Duration, Instant};

use agtrace_runtime::{SessionStreamer, StreamEvent, WorkspaceEvent};
use agtrace_types::StreamId;

const SESSION_ID: &str = "11111111-2222-3333-4444-555555555555";

fn main_file_content() -> String {
    format!(
        concat!(
            r#"{{"parentUuid":null,"isSidechain":false,"type":"user","message":{{"role":"user","content":"hello"}},"uuid":"u1","timestamp":"2026-07-13T04:00:00.000Z","sessionId":"{sid}","cwd":"/tmp/proj","version":"2.1.207"}}"#,
            "\n"
        ),
        sid = SESSION_ID
    )
}

fn subagent_file_content() -> String {
    format!(
        concat!(
            r#"{{"parentUuid":null,"isSidechain":true,"agentId":"abc123def456","type":"user","message":{{"role":"user","content":"explore the codebase"}},"uuid":"s1","timestamp":"2026-07-13T04:01:00.000Z","sessionId":"{sid}","cwd":"/tmp/proj","version":"2.1.207"}}"#,
            "\n"
        ),
        sid = SESSION_ID
    )
}

#[test]
fn subagent_file_created_after_attach_appears_in_stream() {
    let dir = tempfile::tempdir().unwrap();
    let log_root = dir.path().to_path_buf();

    let main_file = log_root.join(format!("{SESSION_ID}.jsonl"));
    std::fs::write(&main_file, main_file_content()).unwrap();

    let adapter = agtrace_providers::create_adapter("claude_code").unwrap();
    let streamer = SessionStreamer::attach_from_filesystem(
        SESSION_ID.to_string(),
        log_root.clone(),
        Arc::new(adapter),
    )
    .unwrap();

    // Create the subagent transcript only after the streamer has attached,
    // mirroring how Claude Code spawns Task subagents mid-session.
    let subagents_dir = log_root.join(SESSION_ID).join("subagents");
    std::fs::create_dir_all(&subagents_dir).unwrap();
    std::fs::write(
        subagents_dir.join("agent-abc123def456.jsonl"),
        subagent_file_content(),
    )
    .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_sidechain = false;
    while Instant::now() < deadline {
        match streamer.receiver().recv_timeout(Duration::from_millis(200)) {
            Ok(WorkspaceEvent::Stream(StreamEvent::Events { sessions, .. })) => {
                if sessions
                    .iter()
                    .any(|s| matches!(s.stream_id, StreamId::Sidechain { .. }))
                {
                    saw_sidechain = true;
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    assert!(
        saw_sidechain,
        "subagent (sidechain) events did not appear in the stream after the \
         subagent file was created post-attach"
    );
}
