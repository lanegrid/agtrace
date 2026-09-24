//! `Client::watch_workspace` end to end: the runtime watcher runs against a writable
//! copy of the synthetic fixture workspace, pointed to by `AGTRACE_CLAUDE_HOME` /
//! `AGTRACE_CODEX_HOME`, and the SDK folds its events into a `WorkspaceView`.
//!
//! This file is its own test binary (own process) because it sets the environment.

use agtrace_sdk::Client;
use agtrace_sdk::types::{AgentId, AgentKind};
use agtrace_sdk::watch::{TimelineItem, WatchScope, WorkspaceView};
use agtrace_testing::live_fixture::{
    CODEX_CHILD, CODEX_FORK, CODEX_ROOT, FORK_ID, LEAD_SESSION, LiveFixture, PROJECT_ROOT,
    SUBAGENT_ID, TEAMMATE_SESSION,
};
use std::io::Write;
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(15);

const LEAD_USER_LINE: &str = r#"{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/work/demo-project","sessionId":"00000000-0000-4000-8000-000000000001","type":"user","uuid":"00000000-0000-4000-8000-00000000fe02","timestamp":"2026-09-20T10:02:00.000Z","message":{"role":"user","content":"Late follow-up from the test."}}"#;

fn has_user_text(view: &WorkspaceView, id: &AgentId, needle: &str) -> bool {
    view.agents.get(id).is_some_and(|a| {
        a.recent
            .iter()
            .any(|e| matches!(&e.item, TimelineItem::User { text } if text.contains(needle)))
    })
}

#[tokio::test]
async fn live_workspace_follows_the_fixture_workspace() -> anyhow::Result<()> {
    let fx = LiveFixture::new(chrono::Local::now().date_naive())?;
    fx.set_registry_pid(std::process::id())?;
    // Created during the test: a teammate (discovered late) and a Codex child.
    let teammate = fx.stash(&fx.teammate_file())?;
    let codex_child_path = fx.codex_file(CODEX_CHILD);
    let codex_child = fx.stash(&codex_child_path)?;

    // SAFETY: this test binary has a single test; nothing else reads the environment
    // concurrently.
    unsafe {
        std::env::set_var(agtrace_sdk::utils::CLAUDE_HOME_ENV, fx.claude_home());
        std::env::set_var(agtrace_sdk::utils::CODEX_HOME_ENV, fx.codex_home());
    }
    let data = tempfile::TempDir::new()?;
    let client = Client::connect(data.path()).await?;

    let mut live = client.watch_workspace(WatchScope::project(PROJECT_ROOT))?;
    let lead = AgentId::claude_session(LEAD_SESSION);
    let subagent = AgentId::claude_subagent(LEAD_SESSION, SUBAGENT_ID);
    let fork = AgentId::claude_subagent(LEAD_SESSION, FORK_ID);
    let codex_root = AgentId::codex_thread(CODEX_ROOT);
    let codex_fork = AgentId::codex_thread(CODEX_FORK);

    assert!(
        live.wait_until(WAIT, |v| {
            [&lead, &subagent, &fork, &codex_root, &codex_fork]
                .iter()
                .all(|id| v.agents.get(*id).is_some_and(|a| a.discovered))
        }),
        "initial agents discovered"
    );
    live.with_view(|v| {
        assert_eq!(v.agents.len(), 5);
        // Tree: subagent and fork under the lead, the Codex fork under its root.
        let lead_view = &v.agents[&lead];
        assert!(lead_view.children.contains(&subagent));
        assert!(lead_view.children.contains(&fork));
        assert!(v.agents[&codex_root].children.contains(&codex_fork));
        assert_eq!(v.agents[&fork].agent.kind, AgentKind::Fork);
        // Context windows are resolved through the catalog.
        assert!(v.agents[&codex_root].window.is_some());
        assert!(lead_view.window.is_some());
    });
    let generation = live.generation();
    assert!(generation > 0);

    // A teammate appears later and is linked to its lead through the team config.
    let teammate_id = AgentId::claude_session(TEAMMATE_SESSION);
    fx.restore(&teammate, &fx.teammate_file())?;
    assert!(
        live.wait_until(WAIT, |v| v
            .agents
            .get(&teammate_id)
            .is_some_and(|a| a.discovered && a.tree_parent.as_ref() == Some(&lead))),
        "late teammate discovered under the lead"
    );

    // A Codex child thread is spawned later.
    let child_id = AgentId::codex_thread(CODEX_CHILD);
    fx.restore(&codex_child, &codex_child_path)?;
    assert!(
        live.wait_until(WAIT, |v| v.agents.get(&child_id).is_some_and(|a| a
            .discovered
            && a.agent.parent.as_ref() == Some(&codex_root))),
        "late Codex child discovered under its root"
    );

    // Lines appended to a tracked file reach the agent's timeline.
    std::fs::OpenOptions::new()
        .append(true)
        .open(fx.lead_file())?
        .write_all(format!("{LEAD_USER_LINE}\n").as_bytes())?;
    assert!(
        live.wait_until(WAIT, |v| has_user_text(v, &lead, "Late follow-up")),
        "appended line folded"
    );

    // The async change notification fires for further changes.
    let before = live.generation();
    live.rescan();
    std::fs::OpenOptions::new()
        .append(true)
        .open(fx.lead_file())?
        .write_all(format!("{}\n", LEAD_USER_LINE.replace("fe02", "fe03")).as_bytes())?;
    let deadline = std::time::Instant::now() + WAIT;
    while live.generation() == before && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let changed = live.changed().await;
    assert!(changed.is_some_and(|g| g > before));
    Ok(())
}
