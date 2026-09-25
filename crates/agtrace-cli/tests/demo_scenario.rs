//! `agtrace demo` end to end (without a terminal): the synthetic scenario is replayed
//! into a temporary workspace at high speed while the real workspace watcher folds
//! it; the resulting tree, feed and statuses must match the story.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use agtrace::demo::{PROJECT, TempWorkspace, replay, scenario};
use agtrace_sdk::types::{AgentId, AgentKind, AgentMessageKind};
use agtrace_sdk::watch::{AgentStatus, LiveWorkspace, WatchScope, WatcherOptions, WorkspaceView};
use agtrace_sdk::workspace::FeedKind;
use chrono::{Local, Utc};

const WAIT: Duration = Duration::from_secs(20);

fn has_message(view: &WorkspaceView, kind: AgentMessageKind) -> bool {
    view.feed
        .iter()
        .any(|e| matches!(&e.kind, FeedKind::Message(k) if *k == kind))
}

#[test]
fn demo_scenario_builds_the_multi_agent_tree() -> anyhow::Result<()> {
    let dir = TempWorkspace::create()?;
    // 40x: the ~60 s scenario replays in ~1.5 s; timestamps stay consistent with it.
    let steps = scenario(
        Utc::now(),
        40.0,
        std::process::id(),
        Local::now().date_naive(),
    );
    let live = LiveWorkspace::watch_roots(
        WatchScope::Project {
            root: PathBuf::from(PROJECT),
            since: WatchScope::DEFAULT_SINCE,
        },
        dir.roots(),
        WatcherOptions::default(),
        Arc::new(agtrace_sdk::utils::builtin_model_catalog()),
    )?;
    replay(&dir.path, &steps, &AtomicBool::new(false))?;

    let lead = AgentId::claude_session("d0000000-0000-4000-8000-000000000001");
    let mate = AgentId::claude_session("d0000000-0000-4000-8000-000000000002");
    let sub = AgentId::claude_subagent("d0000000-0000-4000-8000-000000000001", "ad000000000000001");
    let root = AgentId::codex_thread("01900000-0000-7000-8000-0000000000d1");
    let judge = AgentId::codex_thread("01900000-0000-7000-8000-0000000000d2");

    let settled = live.wait_until(WAIT, |v| {
        let kind = |id: &AgentId| v.agent(id).map(|a| a.agent.kind);
        kind(&lead) == Some(AgentKind::Main)
            && kind(&mate) == Some(AgentKind::Teammate)
            && kind(&sub) == Some(AgentKind::Subagent)
            && kind(&root) == Some(AgentKind::Main)
            && kind(&judge) == Some(AgentKind::CodexThread)
            && v.agent(&mate).and_then(|a| a.tree_parent.as_ref()) == Some(&lead)
            && v.agent(&sub).and_then(|a| a.tree_parent.as_ref()) == Some(&lead)
            && v.agent(&judge).and_then(|a| a.tree_parent.as_ref()) == Some(&root)
            && has_message(v, AgentMessageKind::FinalAnswer)
            && has_message(v, AgentMessageKind::Handback)
            && has_message(v, AgentMessageKind::NewTask)
            && v.agent(&mate)
                .is_some_and(|a| a.status == AgentStatus::Killed)
    });
    let dump = live.with_view(|v| {
        v.tree()
            .into_iter()
            .map(|(id, depth)| {
                let a = &v.agents[id];
                format!(
                    "{}{} {:?} {:?} parent={:?}",
                    "  ".repeat(depth),
                    a.label(),
                    a.agent.kind,
                    a.status,
                    a.tree_parent
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + &format!(
                "\nfeed: {:?}\nerrors: {:?}\ndiag: {}",
                v.feed.iter().map(|e| &e.kind).collect::<Vec<_>>(),
                v.errors.iter().collect::<Vec<_>>(),
                v.total_diagnostic_errors()
            )
    });
    assert!(settled, "demo workspace did not settle:\n{dump}");

    live.with_view(|v| {
        assert_eq!(v.total_diagnostic_errors(), 0, "{dump}");
        assert!(v.errors.is_empty(), "{dump}");
        // Context windows are resolved: Claude 1M from the model attachment, Codex from the log.
        let window = |id: &AgentId| {
            v.agent(id)
                .and_then(|a| a.window.as_ref())
                .map(|w| w.tokens)
        };
        assert_eq!(window(&lead), Some(1_000_000), "{dump}");
        assert_eq!(window(&root), Some(258_400), "{dump}");
        // Codex bodies are encrypted in the feed; FINAL_ANSWER is plaintext.
        assert!(v.feed.iter().any(|e| {
            matches!(&e.kind, FeedKind::Message(AgentMessageKind::NewTask)) && e.encrypted
        }));
    });
    Ok(())
}
