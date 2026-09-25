//! Watcher state tests against a writable copy of the synthetic fixture workspace.
//! The state machine is driven directly (no thread, explicit ticks).

use super::state::WatcherState;
use super::*;
use agtrace_testing::live_fixture::{
    CODEX_CHILD, CODEX_FORK, CODEX_ROOT, FORK_ID, LEAD_SESSION, LiveFixture, PROJECT_ROOT,
    SUBAGENT_ID, TEAMMATE_SESSION,
};
use agtrace_types::{AgentKind, EventPayload};
use chrono::Local;
use std::collections::BTreeSet;
use std::io::Write;

fn fixture() -> LiveFixture {
    let fx = LiveFixture::new(Local::now().date_naive()).unwrap();
    fx.set_registry_pid(std::process::id()).unwrap();
    fx
}

fn roots(fx: &LiveFixture) -> WatchRoots {
    WatchRoots::from_homes(Some(fx.claude_home()), Some(fx.codex_home()))
}

fn project_state(fx: &LiveFixture) -> WatcherState {
    WatcherState::new(WatchScope::project(PROJECT_ROOT), roots(fx))
}

fn lead() -> AgentId {
    AgentId::claude_session(LEAD_SESSION)
}

fn all_ids() -> BTreeSet<AgentId> {
    [
        lead(),
        AgentId::claude_session(TEAMMATE_SESSION),
        AgentId::claude_subagent(LEAD_SESSION, SUBAGENT_ID),
        AgentId::claude_subagent(LEAD_SESSION, FORK_ID),
        AgentId::codex_thread(CODEX_ROOT),
        AgentId::codex_thread(CODEX_CHILD),
        AgentId::codex_thread(CODEX_FORK),
    ]
    .into_iter()
    .collect()
}

fn discovered(events: &[WorkspaceEvent]) -> BTreeSet<AgentId> {
    events
        .iter()
        .filter_map(|e| match e {
            WorkspaceEvent::AgentDiscovered(a) => Some(a.id.clone()),
            _ => None,
        })
        .collect()
}

fn events_of<'a>(events: &'a [WorkspaceEvent], id: &AgentId) -> Vec<&'a agtrace_types::AgentEvent> {
    events
        .iter()
        .filter_map(|e| match e {
            WorkspaceEvent::Events { agent, events, .. } if agent == id => Some(events),
            _ => None,
        })
        .flatten()
        .collect()
}

fn append(path: &std::path::Path, text: &str) {
    let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    f.write_all(text.as_bytes()).unwrap();
}

const LEAD_USER_LINE: &str = r#"{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/work/demo-project","sessionId":"00000000-0000-4000-8000-000000000001","type":"user","uuid":"00000000-0000-4000-8000-00000000fe01","timestamp":"2026-09-20T10:02:00.000Z","message":{"role":"user","content":"One more thing."}}"#;

#[test]
fn project_scope_discovers_the_fixture_workspace() {
    let fx = fixture();
    let mut state = project_state(&fx);
    let out = state.discovery_tick(SystemTime::now());

    assert_eq!(discovered(&out), all_ids());
    for id in all_ids() {
        // Discovered before its events, and every file produced events.
        let first_discovery = out
            .iter()
            .position(|e| matches!(e, WorkspaceEvent::AgentDiscovered(a) if a.id == id))
            .unwrap();
        let first_events = out
            .iter()
            .position(|e| matches!(e, WorkspaceEvent::Events { agent, .. } if *agent == id))
            .unwrap_or_else(|| panic!("no events for {id:?}"));
        assert!(first_discovery < first_events, "{id:?}");
        assert!(!events_of(&out, &id).is_empty());
    }
    // Initial tails are not resets.
    assert!(
        out.iter()
            .all(|e| !matches!(e, WorkspaceEvent::Events { reset: true, .. }))
    );

    // Side state: team config (lead tracked), registry (live pid), subagent metas.
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::SideState(SideStateUpdate::ClaudeTeam { team, lead_session_id, members })
            if team == "session-00000001" && lead_session_id == LEAD_SESSION && members.len() == 2
    )));
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess { session_id, alive: true, status: Some(ProcessStatus::Busy), .. })
            if session_id == LEAD_SESSION
    )));
    let metas = out
        .iter()
        .filter(|e| {
            matches!(
                e,
                WorkspaceEvent::SideState(SideStateUpdate::ClaudeSubagentMeta { .. })
            )
        })
        .count();
    assert_eq!(metas, 2);
    // The lead has deliberate bad lines: diagnostics are reported.
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::Diagnostics { agent, diagnostics } if *agent == lead() && diagnostics.invalid_json > 0
    )));

    // Kinds and linkage come from the headers.
    let kind_of = |id: &AgentId| {
        out.iter()
            .find_map(|e| match e {
                WorkspaceEvent::AgentDiscovered(a) if a.id == *id => Some(a.kind),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        kind_of(&AgentId::claude_session(TEAMMATE_SESSION)),
        AgentKind::Teammate
    );
    assert_eq!(
        kind_of(&AgentId::claude_subagent(LEAD_SESSION, FORK_ID)),
        AgentKind::Fork
    );
    assert_eq!(
        kind_of(&AgentId::codex_thread(CODEX_CHILD)),
        AgentKind::CodexThread
    );

    // Nothing changed: the next ticks are quiet.
    assert!(state.discovery_tick(SystemTime::now()).is_empty());
    assert!(state.poll_tick().is_empty());
}

#[test]
fn appended_and_partial_lines_are_emitted_once_complete() {
    let fx = fixture();
    let mut state = project_state(&fx);
    state.discovery_tick(SystemTime::now());
    let lines_before = std::fs::read_to_string(fx.lead_file())
        .unwrap()
        .lines()
        .count() as u64;

    let (head, tail) = LEAD_USER_LINE.split_at(40);
    append(&fx.lead_file(), head);
    assert!(state.poll_tick().is_empty(), "partial line is not decoded");
    append(&fx.lead_file(), &format!("{tail}\n"));
    let out = state.poll_tick();
    let new = events_of(&out, &lead());
    assert_eq!(new.len(), 1);
    assert!(matches!(new[0].payload, EventPayload::User(_)));
    assert_eq!(new[0].origin.line, lines_before);
    assert!(matches!(
        out.as_slice(),
        [WorkspaceEvent::Events { reset: false, .. }]
    ));
    assert!(state.poll_tick().is_empty());
}

#[test]
fn truncated_file_is_re_emitted_as_reset() {
    let fx = fixture();
    let mut state = project_state(&fx);
    state.discovery_tick(SystemTime::now());
    let file = fx.codex_file(CODEX_CHILD);
    let text = std::fs::read_to_string(&file).unwrap();
    let first_two: String = text.lines().take(2).map(|l| format!("{l}\n")).collect();
    std::fs::write(&file, first_two).unwrap();
    let out = state.poll_tick();
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::Events { agent, reset: true, .. } if *agent == AgentId::codex_thread(CODEX_CHILD)
    )));
}

#[test]
fn teammate_created_later_is_discovered_on_the_next_tick() {
    let fx = fixture();
    let stashed = fx.stash(&fx.teammate_file()).unwrap();
    let mut state = project_state(&fx);
    let first = state.discovery_tick(SystemTime::now());
    assert!(!discovered(&first).contains(&AgentId::claude_session(TEAMMATE_SESSION)));

    fx.restore(&stashed, &fx.teammate_file()).unwrap();
    let out = state.discovery_tick(SystemTime::now());
    assert_eq!(
        discovered(&out),
        [AgentId::claude_session(TEAMMATE_SESSION)]
            .into_iter()
            .collect()
    );
    assert!(!events_of(&out, &AgentId::claude_session(TEAMMATE_SESSION)).is_empty());
}

#[test]
fn codex_child_spawned_later_is_discovered_under_its_root() {
    let fx = fixture();
    let child = fx.codex_file(CODEX_CHILD);
    let stashed = fx.stash(&child).unwrap();
    let mut state = project_state(&fx);
    assert!(
        !discovered(&state.discovery_tick(SystemTime::now()))
            .contains(&AgentId::codex_thread(CODEX_CHILD))
    );

    // The rollout is born empty, then gets its session_meta line.
    std::fs::write(&child, "").unwrap();
    assert!(
        !discovered(&state.discovery_tick(SystemTime::now()))
            .contains(&AgentId::codex_thread(CODEX_CHILD))
    );
    fx.restore(&stashed, &child).unwrap();
    let out = state.discovery_tick(SystemTime::now());
    let agent = out
        .iter()
        .find_map(|e| match e {
            WorkspaceEvent::AgentDiscovered(a) => Some(a.clone()),
            _ => None,
        })
        .expect("child discovered");
    assert_eq!(agent.id, AgentId::codex_thread(CODEX_CHILD));
    assert_eq!(agent.parent, Some(AgentId::codex_thread(CODEX_ROOT)));
}

#[test]
fn window_excludes_stale_roots_unless_the_registry_says_live() {
    let fx = fixture();
    fx.set_registry_pid(u32::MAX - 1).unwrap(); // not a live pid
    fx.age_all(3 * 60 * 60).unwrap();
    let mut state = project_state(&fx);
    assert!(discovered(&state.discovery_tick(SystemTime::now())).is_empty());

    // A live registry entry for the lead brings in the lead and all its descendants
    // (subagents, and the teammate via the team config), however old their files are.
    fx.set_registry_pid(std::process::id()).unwrap();
    let out = state.discovery_tick(SystemTime::now());
    let expected: BTreeSet<AgentId> = [
        lead(),
        AgentId::claude_session(TEAMMATE_SESSION),
        AgentId::claude_subagent(LEAD_SESSION, SUBAGENT_ID),
        AgentId::claude_subagent(LEAD_SESSION, FORK_ID),
    ]
    .into_iter()
    .collect();
    assert_eq!(discovered(&out), expected);
}

#[test]
fn recent_root_brings_its_old_descendants() {
    let fx = fixture();
    fx.age_all(3 * 60 * 60).unwrap();
    fx.set_registry_pid(u32::MAX - 1).unwrap();
    // Only the Codex root was written recently.
    filetime::set_file_mtime(fx.codex_file(CODEX_ROOT), filetime::FileTime::now()).unwrap();
    let mut state = project_state(&fx);
    let out = state.discovery_tick(SystemTime::now());
    let expected: BTreeSet<AgentId> = [
        AgentId::codex_thread(CODEX_ROOT),
        AgentId::codex_thread(CODEX_CHILD),
        AgentId::codex_thread(CODEX_FORK),
    ]
    .into_iter()
    .collect();
    assert_eq!(discovered(&out), expected);
}

#[test]
fn other_projects_are_ignored() {
    let fx = fixture();
    let mut state = WatcherState::new(WatchScope::project("/work/other"), roots(&fx));
    let out = state.discovery_tick(SystemTime::now());
    assert!(discovered(&out).is_empty());
}

#[test]
fn root_scope_follows_one_tree() {
    let fx = fixture();
    let mut claude = WatcherState::new(WatchScope::Root(lead()), roots(&fx));
    let out = claude.discovery_tick(SystemTime::now());
    let expected: BTreeSet<AgentId> = [
        lead(),
        AgentId::claude_session(TEAMMATE_SESSION),
        AgentId::claude_subagent(LEAD_SESSION, SUBAGENT_ID),
        AgentId::claude_subagent(LEAD_SESSION, FORK_ID),
    ]
    .into_iter()
    .collect();
    assert_eq!(discovered(&out), expected);

    let mut codex = WatcherState::new(
        WatchScope::Root(AgentId::codex_thread(CODEX_ROOT)),
        roots(&fx),
    );
    let out = codex.discovery_tick(SystemTime::now());
    let expected: BTreeSet<AgentId> = [
        AgentId::codex_thread(CODEX_ROOT),
        AgentId::codex_thread(CODEX_CHILD),
        AgentId::codex_thread(CODEX_FORK),
    ]
    .into_iter()
    .collect();
    assert_eq!(discovered(&out), expected);
}

#[test]
fn root_scope_finds_an_old_codex_root_outside_recent_date_dirs() {
    let fx = LiveFixture::new(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()).unwrap();
    let mut state = WatcherState::new(
        WatchScope::Root(AgentId::codex_thread(CODEX_ROOT)),
        roots(&fx),
    );
    let out = state.discovery_tick(SystemTime::now());
    assert_eq!(discovered(&out).len(), 3);
}

fn days_ago(n: u64) -> chrono::NaiveDate {
    Local::now().date_naive() - chrono::Days::new(n)
}

/// `AgentDiscovered` count per agent (a tree must never list an agent twice).
fn discovery_counts(events: &[WorkspaceEvent]) -> std::collections::BTreeMap<AgentId, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for e in events {
        if let WorkspaceEvent::AgentDiscovered(a) = e {
            *counts.entry(a.id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn codex_ids() -> BTreeSet<AgentId> {
    [CODEX_ROOT, CODEX_CHILD, CODEX_FORK]
        .into_iter()
        .map(AgentId::codex_thread)
        .collect()
}

#[test]
fn root_scope_finds_descendants_in_later_date_dirs() {
    // A long-running tree: root created 10 days ago, children on later days (each
    // rollout lives in the dir of its creation date), none today or yesterday.
    let fx = LiveFixture::new(days_ago(10)).unwrap();
    fx.move_codex_to_day(CODEX_CHILD, days_ago(9)).unwrap();
    fx.move_codex_to_day(CODEX_FORK, days_ago(3)).unwrap();
    let mut state = WatcherState::new(
        WatchScope::Root(AgentId::codex_thread(CODEX_ROOT)),
        roots(&fx),
    );
    let now = SystemTime::now();
    let mut out = state.discovery_tick(now);
    assert_eq!(discovered(&out), codex_ids());
    out.extend(state.discovery_tick(now + Duration::from_secs(1)));
    out.extend(state.discovery_tick(now + Duration::from_secs(40)));
    assert!(
        discovery_counts(&out).values().all(|n| *n == 1),
        "{:?}",
        discovery_counts(&out)
    );
}

#[test]
fn project_scope_window_reaches_older_date_dirs() {
    // Rollouts created 5 days ago and still written to (fresh mtimes).
    let fx = LiveFixture::new(days_ago(5)).unwrap();
    let mut wide = WatcherState::new(
        WatchScope::Project {
            root: PROJECT_ROOT.into(),
            since: Duration::from_secs(7 * 24 * 3600),
        },
        roots(&fx),
    );
    let out = wide.discovery_tick(SystemTime::now());
    assert!(codex_ids().is_subset(&discovered(&out)));
}

#[test]
fn older_date_dirs_are_relisted_on_a_slow_cadence() {
    let fx = LiveFixture::new(days_ago(5)).unwrap();
    let child = fx.codex_file(CODEX_CHILD);
    let stashed = fx.stash(&child).unwrap();
    let mut state = WatcherState::new(
        WatchScope::Project {
            root: PROJECT_ROOT.into(),
            since: Duration::from_secs(7 * 24 * 3600),
        },
        roots(&fx),
    );
    let now = SystemTime::now();
    state.discovery_tick(now);
    fx.restore(&stashed, &child).unwrap();
    let child_id = AgentId::codex_thread(CODEX_CHILD);
    let out = state.discovery_tick(now + Duration::from_secs(1));
    assert!(!discovered(&out).contains(&child_id));
    let out = state.discovery_tick(now + Duration::from_secs(31));
    assert!(discovered(&out).contains(&child_id));
}

#[test]
fn codex_history_range_is_empty_for_recent_windows() {
    use super::state::codex_history_range;
    let today = days_ago(0);
    assert!(codex_history_range(today, today).is_empty());
    assert!(codex_history_range(days_ago(1), today).is_empty());
    // `first - 1` (offset margin) ..= the day before yesterday.
    assert_eq!(
        codex_history_range(days_ago(4), today),
        vec![days_ago(5), days_ago(4), days_ago(3), days_ago(2)]
    );
}

#[test]
fn registry_entry_removal_is_reported_dead_once() {
    let fx = fixture();
    let mut state = project_state(&fx);
    state.discovery_tick(SystemTime::now());
    std::fs::remove_file(
        fx.claude_home()
            .join(format!("sessions/{}.json", std::process::id())),
    )
    .unwrap();
    let out = state.discovery_tick(SystemTime::now());
    assert!(matches!(
        out.as_slice(),
        [WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess {
            alive: false,
            ..
        })]
    ));
    assert!(state.discovery_tick(SystemTime::now()).is_empty());
}

#[test]
fn subagent_meta_change_is_reported() {
    let fx = fixture();
    let mut state = project_state(&fx);
    state.discovery_tick(SystemTime::now());
    let meta = fx.subagent_file(SUBAGENT_ID).with_extension("meta.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    v["stoppedByUser"] = serde_json::json!(true);
    std::fs::write(&meta, v.to_string()).unwrap();
    filetime::set_file_mtime(&meta, filetime::FileTime::from_unix_time(1, 0)).unwrap();
    let out = state.discovery_tick(SystemTime::now());
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::SideState(SideStateUpdate::ClaudeSubagentMeta { agent, stopped_by_user: true, .. })
            if *agent == AgentId::claude_subagent(LEAD_SESSION, SUBAGENT_ID)
    )));
}

#[test]
fn watcher_thread_streams_events_and_stops_on_drop() {
    let fx = fixture();
    let watcher = WorkspaceWatcher::start(
        WatchScope::project(PROJECT_ROOT),
        roots(&fx),
        WatcherOptions {
            poll_interval: std::time::Duration::from_millis(20),
            discovery_interval: std::time::Duration::from_millis(50),
        },
    )
    .unwrap();
    let mut seen = BTreeSet::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while seen != all_ids() && std::time::Instant::now() < deadline {
        if let Ok(WorkspaceEvent::AgentDiscovered(a)) = watcher
            .receiver()
            .recv_timeout(std::time::Duration::from_millis(200))
        {
            seen.insert(a.id);
        }
    }
    assert_eq!(seen, all_ids());

    append(&fx.lead_file(), &format!("{LEAD_USER_LINE}\n"));
    let mut got_append = false;
    while !got_append && std::time::Instant::now() < deadline {
        if let Ok(WorkspaceEvent::Events { agent, events, .. }) = watcher
            .receiver()
            .recv_timeout(std::time::Duration::from_millis(200))
        {
            got_append = agent == lead()
                && events
                    .iter()
                    .any(|e| matches!(e.payload, EventPayload::User(_)));
        }
    }
    assert!(got_append);
    watcher.rescan();
    drop(watcher); // joins the thread
}

/// A live process of the project that has not written a transcript yet (a bg job
/// waiting for work) is reported, so the sessions list can show it; one of another
/// project is not.
#[test]
fn live_process_without_transcript_is_reported_in_project_scope() {
    let fx = fixture();
    let entry = |sid: &str, cwd: &str| {
        serde_json::json!({
            "pid": std::process::id(),
            "sessionId": sid,
            "cwd": cwd,
            "kind": "bg",
            "name": &sid[..8],
            "status": "idle",
            "updatedAt": 1789898460000i64
        })
        .to_string()
    };
    let dir = fx.claude_home().join("sessions");
    std::fs::write(
        dir.join("1000001.json"),
        entry(
            "0000abcd-0000-4000-8000-000000000077",
            "/work/demo-project/sub",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("1000002.json"),
        entry("0000dcba-0000-4000-8000-000000000088", "/work/elsewhere"),
    )
    .unwrap();
    let mut state = project_state(&fx);
    let out = state.discovery_tick(SystemTime::now());
    let reported: Vec<(&str, bool)> = out
        .iter()
        .filter_map(|e| match e {
            WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess {
                session_id,
                alive: true,
                bg,
                ..
            }) => Some((session_id.as_str(), *bg)),
            _ => None,
        })
        .collect();
    assert!(
        reported.contains(&("0000abcd-0000-4000-8000-000000000077", true)),
        "{reported:?}"
    );
    assert!(
        !reported
            .iter()
            .any(|(s, _)| *s == "0000dcba-0000-4000-8000-000000000088"),
        "{reported:?}"
    );

    // Gone: reported dead once.
    std::fs::remove_file(dir.join("1000001.json")).unwrap();
    let out = state.discovery_tick(SystemTime::now());
    assert!(out.iter().any(|e| matches!(
        e,
        WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess { session_id, alive: false, .. })
            if session_id == "0000abcd-0000-4000-8000-000000000077"
    )));
}
