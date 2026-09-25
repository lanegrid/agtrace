use agtrace_providers::{ClaudeProvider, CodexProvider, Provider};
use std::path::PathBuf;

/// The project hash is derived from the file header's cwd (not from per-event metadata).
fn assert_header_cwd_hash(provider: &dyn Provider, path: &str) {
    let path = PathBuf::from(path);
    let header = provider
        .read_header(&path)
        .expect("header read")
        .expect("sample is an agent file");
    let cwd = header.project_cwd.expect("header carries the project cwd");
    let project_hash = agtrace_core::project_hash_from_root(&cwd.to_string_lossy());
    assert_ne!(
        project_hash,
        agtrace_types::ProjectHash::from("unknown"),
        "Project hash derived from cwd should not be 'unknown'"
    );
}

#[test]
fn test_claude_derives_project_hash_from_header() {
    assert_header_cwd_hash(&ClaudeProvider, "tests/samples/claude_session.jsonl");
}

#[test]
fn test_codex_derives_project_hash_from_header() {
    assert_header_cwd_hash(&CodexProvider, "tests/samples/codex_session.jsonl");
}
