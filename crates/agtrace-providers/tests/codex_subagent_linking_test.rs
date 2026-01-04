use agtrace_providers::codex::{extract_codex_header, extract_spawn_events};
use agtrace_providers::traits::LogDiscovery;
use std::path::Path;

#[test]
fn test_extract_spawn_events_from_parent() {
    let path = Path::new("tests/samples/rollout-parent-019b88e0.jsonl");

    let spawn_events = extract_spawn_events(path).expect("Should extract spawn events");

    assert_eq!(spawn_events.len(), 1, "Should have exactly 1 spawn event");
    assert_eq!(spawn_events[0].subagent_type, "review");
    assert_eq!(spawn_events[0].timestamp, "2026-01-04T12:05:09.476Z");
    assert_eq!(spawn_events[0].spawn_context.turn_index, 1);
}

#[test]
fn test_extract_subagent_header() {
    let path = Path::new("tests/samples/rollout-subagent-019b88e5.jsonl");

    let header = extract_codex_header(path).expect("Should extract header");

    assert_eq!(
        header.session_id,
        Some("019b88e5-a2e4-7b90-8953-38fce393c653".to_string())
    );
    assert_eq!(header.subagent_type, Some("review".to_string()));
    assert_eq!(
        header.timestamp,
        Some("2026-01-04T12:05:09.500Z".to_string())
    );
}

#[test]
fn test_scan_sessions_links_parent_child() {
    let discovery = agtrace_providers::codex::CodexDiscovery;

    let sessions = discovery
        .scan_sessions(Path::new("tests/samples"))
        .expect("Should scan sessions");

    // Debug: print all found sessions
    eprintln!("Found {} sessions:", sessions.len());
    for s in &sessions {
        eprintln!("  - {} (file: {})", s.session_id, s.main_file.display());
    }

    // Find parent and subagent sessions
    let parent = sessions
        .iter()
        .find(|s| s.session_id == "019b88e0-0b0f-7bb0-a9ba-5cc2d8dffde9");
    let subagent = sessions
        .iter()
        .find(|s| s.session_id == "019b88e5-a2e4-7b90-8953-38fce393c653");

    assert!(parent.is_some(), "Should find parent session");
    assert!(subagent.is_some(), "Should find subagent session");

    let subagent = subagent.unwrap();

    // Verify parent-child linking
    assert_eq!(
        subagent.parent_session_id,
        Some("019b88e0-0b0f-7bb0-a9ba-5cc2d8dffde9".to_string()),
        "Subagent should be linked to parent"
    );

    assert!(
        subagent.spawned_by.is_some(),
        "Subagent should have spawn context"
    );
    let spawn_ctx = subagent.spawned_by.as_ref().unwrap();
    assert_eq!(spawn_ctx.turn_index, 1);
}
