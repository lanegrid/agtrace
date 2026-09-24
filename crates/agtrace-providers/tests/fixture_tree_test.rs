//! The synthetic v2026_09 fixture tree decodes cleanly through the provider contract.

use agtrace_providers::{
    ClaudeProvider, CodexProvider, DecodeOptions, DiscoveryScope, Provider, decode_file,
};
use agtrace_types::{AgentKind, EventPayload};
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../agtrace-testing/fixtures/v2026_09")
}

fn discover(provider: &dyn Provider, root: PathBuf) -> Vec<agtrace_providers::FileHeader> {
    let mut headers = provider
        .discover(&DiscoveryScope {
            roots: Some(vec![root]),
            project_root: Some(PathBuf::from("/work/demo-project")),
        })
        .expect("discover");
    headers.sort_by(|a, b| a.agent.id.cmp(&b.agent.id));
    headers
}

fn assert_clean(provider: &dyn Provider, header: &agtrace_providers::FileHeader) -> usize {
    let (_, events, diag) =
        decode_file(provider, &header.agent.file, DecodeOptions::default()).expect("decode");
    assert!(
        !diag.has_errors(),
        "{}: {diag:?}",
        header.agent.file.display()
    );
    assert!(events.iter().all(|e| e.agent == header.agent.id));
    // Events are in file order.
    assert!(events.windows(2).all(|w| w[0].origin < w[1].origin));
    events.len()
}

#[test]
fn claude_fixture_tree() {
    let headers = discover(&ClaudeProvider, fixture_root().join("claude/home/projects"));
    let ids: Vec<_> = headers.iter().map(|h| h.agent.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "claude:00000000-0000-4000-8000-000000000001",
            "claude:00000000-0000-4000-8000-000000000001/a0000000000000001",
            "claude:00000000-0000-4000-8000-000000000001/a0000000000000002",
            "claude:00000000-0000-4000-8000-000000000002",
        ]
    );
    let kinds: Vec<_> = headers.iter().map(|h| h.agent.kind).collect();
    assert_eq!(
        kinds,
        vec![
            AgentKind::Main,
            AgentKind::Subagent,
            AgentKind::Fork,
            AgentKind::Teammate
        ]
    );
    assert_eq!(headers[1].agent.parent.as_ref(), Some(&headers[0].agent.id));
    assert_eq!(headers[2].agent.parent.as_ref(), Some(&headers[0].agent.id));
    // The lead fixture deliberately contains bad lines (see claude_fixture_snapshots).
    for h in &headers[1..] {
        assert!(assert_clean(&ClaudeProvider, h) > 0);
    }
}

#[test]
fn codex_fixture_tree() {
    let headers = discover(&CodexProvider, fixture_root().join("codex/home/sessions"));
    assert_eq!(headers.len(), 3);
    let (root, child, fork) = (&headers[0], &headers[1], &headers[2]);
    assert_eq!(fork.agent.kind, AgentKind::Fork);
    assert_eq!(fork.agent.parent.as_ref(), Some(&root.agent.id));
    assert_eq!(root.agent.kind, AgentKind::Main);
    assert_eq!(child.agent.kind, AgentKind::CodexThread);
    assert_eq!(child.agent.parent.as_ref(), Some(&root.agent.id));
    assert_eq!(child.agent.root, root.agent.id);
    assert_eq!(child.agent.path.as_deref(), Some("/root/judge"));
    for h in &headers {
        assert!(assert_clean(&CodexProvider, h) > 0);
    }

    // Array tool output is flattened (was a hard parse failure before).
    let (_, events, _) =
        decode_file(&CodexProvider, &root.agent.file, DecodeOptions::default()).unwrap();
    assert!(events.iter().any(|e| matches!(
        &e.payload,
        EventPayload::ToolResult(r) if r.output.ends_with("Output:\n\nlib.rs\nparser.rs\n")
    )));
}
