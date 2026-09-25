//! Sessions of a workspace: liveness, order, folding of transcripts that belong to
//! the same logical session, and display names.

use agtrace_engine::workspace::{
    NoWindow, ProcessStatus, Session, SessionFold, SessionNameSource, SessionState,
    SideStateUpdate, WorkspaceEvent, WorkspaceView,
};
use agtrace_testing::synth::{AgentBuilder, EventLog, ts};
use agtrace_types::{
    AgentAttributeKey, AgentEvent, AgentId, AgentRef, EventPayload, SlashCommandPayload,
};
use chrono::{DateTime, Utc};

struct Ws {
    view: WorkspaceView,
    now: DateTime<Utc>,
}

impl Ws {
    fn new(now: i64) -> Self {
        Self {
            view: WorkspaceView::new(),
            now: ts(now),
        }
    }

    fn apply(&mut self, ev: WorkspaceEvent) {
        self.view.apply(ev, &NoWindow, self.now);
    }

    fn discover(&mut self, r: impl Into<AgentRef>) -> AgentId {
        let r = r.into();
        let id = r.id.clone();
        self.apply(WorkspaceEvent::AgentDiscovered(r));
        id
    }

    fn feed(&mut self, events: Vec<AgentEvent>) {
        let agent = events[0].agent.clone();
        self.apply(WorkspaceEvent::Events {
            agent,
            events,
            reset: false,
        });
    }

    fn process(&mut self, session: &str, pid: u32, status: ProcessStatus, name: &str, bg: bool) {
        self.apply(WorkspaceEvent::SideState(SideStateUpdate::ClaudeProcess {
            session_id: session.to_string(),
            pid,
            alive: true,
            status: Some(status),
            name: Some(name.to_string()),
            bg,
            updated_at: self.now,
        }));
    }

    fn sessions(&self) -> Vec<Session> {
        self.view.sessions(self.now)
    }

    fn session(&self, root: &AgentId) -> Session {
        self.sessions()
            .into_iter()
            .find(|s| s.root == *root)
            .unwrap_or_else(|| panic!("no session {root:?}"))
    }
}

fn slash(log: &mut EventLog, name: &str) -> AgentEvent {
    log.push(EventPayload::SlashCommand(SlashCommandPayload {
        name: name.to_string(),
        args: None,
    }))
}

/// Order: busy, idle, recent, older; newest activity first within a group.
/// Claude liveness comes from the registry (a live idle process is live however
/// quiet its log is); Codex from a running agent or a write within 30 minutes.
#[test]
fn sessions_are_ordered_by_liveness_then_activity() {
    let mut ws = Ws::new(4 * 3600);
    let at = |h: f64| (h * 3600.0) as i64;

    // Claude, live idle process, last write 3h ago.
    let idle = ws.discover(AgentBuilder::claude_main("c-idle").started(0));
    ws.feed(vec![EventLog::new(&idle).at(at(1.0)).user("old question")]);
    ws.process("c-idle", 11, ProcessStatus::Idle, "c-idle", false);
    // Claude, busy process.
    let busy = ws.discover(AgentBuilder::claude_main("c-busy").started(0));
    ws.feed(vec![EventLog::new(&busy).at(at(3.9)).user("do it")]);
    ws.process("c-busy", 12, ProcessStatus::Busy, "c-busy", false);
    // Codex root written 10 minutes ago, its turn ended: live idle.
    let cx = ws.discover(AgentBuilder::codex_root("x-recent").started(0));
    let mut log = EventLog::new(&cx);
    ws.feed(vec![
        log.at(at(3.8)).user("triage"),
        log.at(at(3.83)).turn_end(),
    ]);
    // Claude without a registry entry, ended 40 minutes ago: recent.
    let recent = ws.discover(AgentBuilder::claude_main("c-recent").started(0));
    let mut log = EventLog::new(&recent);
    ws.feed(vec![log.at(at(3.3)).user("q"), log.at(at(3.33)).turn_end()]);
    // Codex root quiet for 3 hours: older.
    let old = ws.discover(AgentBuilder::codex_root("x-old").started(0));
    let mut log = EventLog::new(&old);
    ws.feed(vec![log.at(at(0.5)).user("q"), log.at(at(0.6)).turn_end()]);
    ws.view.tick(ws.now);

    let got: Vec<(String, SessionState)> = ws
        .sessions()
        .into_iter()
        .map(|s| (s.root.native_session_id().to_string(), s.state))
        .collect();
    assert_eq!(
        got,
        vec![
            ("c-busy".to_string(), SessionState::Busy),
            ("x-recent".to_string(), SessionState::Idle),
            ("c-idle".to_string(), SessionState::Idle),
            ("c-recent".to_string(), SessionState::Recent),
            ("x-old".to_string(), SessionState::Older),
        ]
    );
    assert!(ws.session(&busy).is_live() && !ws.session(&recent).is_live());
}

/// A session is busy while any of its agents runs, even when the root is idle.
#[test]
fn a_running_child_makes_the_session_busy() {
    let mut ws = Ws::new(120);
    let root = ws.discover(AgentBuilder::codex_root("t-root").started(0));
    let child = ws
        .discover(AgentBuilder::codex_child("t-kid", "t-root", "t-root", "/root/kid").started(10));
    let mut log = EventLog::new(&root);
    ws.feed(vec![log.at(0).user("go"), log.at(5).turn_end()]);
    ws.feed(vec![EventLog::new(&child).at(100).bash("cargo test")]);
    let s = ws.session(&root);
    assert_eq!(s.state, SessionState::Busy);
    assert_eq!((s.agents, s.running, s.idle), (2, 1, 1));
}

/// Regression (yohaku-studio): a `/clear` stub that inherited the lead's
/// `agent-name` / `ai-title` was listed as a second root with the same name. It is
/// folded under the session it belongs to, until it gets a conversation of its own.
#[test]
fn stub_transcript_with_the_session_name_folds_into_it() {
    let mut ws = Ws::new(600);
    let lead = ws.discover(AgentBuilder::claude_main("s-lead").started(0));
    let mut log = EventLog::new(&lead);
    ws.feed(vec![
        log.at(0)
            .attribute(AgentAttributeKey::AgentName, "Find the bottleneck"),
        log.at(1).user("why is it slow?"),
        log.at(2).assistant("Looking."),
    ]);
    let stub = ws.discover(AgentBuilder::claude_main("s-stub").started(100));
    let mut slog = EventLog::new(&stub);
    ws.feed(vec![
        slog.at(100)
            .attribute(AgentAttributeKey::AgentName, "Find the bottleneck"),
        slog.at(100).attribute(AgentAttributeKey::SessionKind, "bg"),
        slash(slog.at(101), "clear"),
    ]);
    assert!(ws.view.agents[&stub].is_stub());
    assert_eq!(ws.view.roots, vec![lead.clone()]);
    assert_eq!(ws.view.agents[&stub].tree_parent.as_ref(), Some(&lead));
    assert_eq!(ws.view.agents[&stub].session_fold, Some(SessionFold::Stub));
    assert_eq!(ws.view.agents[&stub].agent.parent, None, "not a spawn link");
    let sessions = ws.sessions();
    assert_eq!(sessions.len(), 1);
    assert!(
        sessions[0].bg,
        "the folded transcript was written by a bg session"
    );

    // It starts a conversation of its own: a separate session again.
    ws.feed(vec![slog.at(200).user("a new question")]);
    assert!(!ws.view.agents[&stub].is_stub());
    assert_eq!(ws.view.roots.len(), 2);
    assert_eq!(ws.view.agents[&stub].session_fold, None);

    // A stub without a matching name stays a session of its own.
    let lone = ws.discover(AgentBuilder::claude_main("s-lone").started(300));
    ws.feed(vec![slash(EventLog::new(&lone).at(300), "resume")]);
    assert!(ws.view.roots.contains(&lone));
}

/// A transcript whose id another transcript wrote records under (the process
/// resumed that transcript) folds into it, like a `continued-in` transcript.
#[test]
fn runtime_alias_and_continued_transcripts_fold_into_the_session() {
    let mut ws = Ws::new(600);
    let resumed = ws.discover(AgentBuilder::claude_main("s-main").started(0));
    let first = ws.discover(AgentBuilder::claude_main("s-first").started(50));
    let earlier = ws.discover(AgentBuilder::claude_main("s-earlier").started(0));
    ws.feed(vec![EventLog::new(&first).at(50).user("hello")]);
    ws.feed(vec![
        EventLog::new(&earlier)
            .at(1)
            .attribute(AgentAttributeKey::ContinuedIn, "s-main"),
    ]);
    ws.feed(vec![
        EventLog::new(&resumed)
            .at(60)
            .attribute(AgentAttributeKey::RuntimeSessionId, "s-first"),
    ]);
    assert_eq!(ws.view.roots, vec![resumed.clone()]);
    assert_eq!(
        ws.view.agents[&first].session_fold,
        Some(SessionFold::RuntimeAlias)
    );
    assert_eq!(
        ws.view.agents[&earlier].session_fold,
        Some(SessionFold::Continued)
    );
    // The process registered under the runtime id is the session's process.
    ws.process("s-first", 7, ProcessStatus::Busy, "Main work", false);
    let s = ws.session(&resumed);
    assert_eq!(s.state, SessionState::Busy);
    assert_eq!(s.agents, 3);
    assert_eq!(s.name, "Main work");
}

/// Name: a meaningful registry name, else the session's own name / title, else
/// the first prompt, else the first slash command, else the short id.
#[test]
fn session_names_prefer_meaningful_sources() {
    let mut ws = Ws::new(600);
    let named = ws.discover(AgentBuilder::claude_main("aaaa1111-named").started(0));
    let mut log = EventLog::new(&named);
    ws.feed(vec![
        log.at(1)
            .attribute(AgentAttributeKey::Title, "Fix the login flow"),
        log.at(2).user("please fix login"),
    ]);
    // A registry name that repeats the id is not a name.
    ws.process("aaaa1111-named", 1, ProcessStatus::Idle, "aaaa1111", true);
    let s = ws.session(&named);
    assert_eq!(
        (s.name.as_str(), s.name_source),
        ("Fix the login flow", SessionNameSource::Title)
    );
    assert!(s.bg);
    ws.process(
        "aaaa1111-named",
        1,
        ProcessStatus::Idle,
        "login-fixer",
        true,
    );
    assert_eq!(ws.session(&named).name, "login-fixer");

    let prompt = ws.discover(AgentBuilder::codex_root("bbbb2222-codex").started(0));
    ws.feed(vec![EventLog::new(&prompt).at(3).user(
        "triage the flaky tests in the watcher, then report back",
    )]);
    let s = ws.session(&prompt);
    assert_eq!(s.name_source, SessionNameSource::Prompt);
    assert!(s.name.starts_with("triage the flaky tests"));

    let cmd = ws.discover(AgentBuilder::claude_main("cccc3333-cmd").started(0));
    ws.feed(vec![slash(EventLog::new(&cmd).at(4), "resume")]);
    let s = ws.session(&cmd);
    assert_eq!(
        (s.name.as_str(), s.name_source),
        ("/resume · cccc3333", SessionNameSource::Command)
    );

    let bare = ws.discover(AgentBuilder::claude_main("dddd4444-bare").started(0));
    let s = ws.session(&bare);
    assert_eq!(
        (s.name.as_str(), s.name_source),
        ("dddd4444", SessionNameSource::Id)
    );
}

/// A live process that has not written a transcript yet (a bg job waiting for
/// work) is a session; once its transcript appears it is that transcript's.
#[test]
fn live_process_without_transcript_is_a_session() {
    let mut ws = Ws::new(600);
    ws.process("eeee5555-job", 9, ProcessStatus::Idle, "eeee5555", true);
    let s = ws.session(&AgentId::claude_session("eeee5555-job"));
    assert!(!s.has_transcript);
    assert_eq!(s.state, SessionState::Idle);
    assert_eq!(s.name, "eeee5555");
    assert!(s.bg);

    let id = ws.discover(AgentBuilder::claude_main("eeee5555-job").started(600));
    ws.feed(vec![EventLog::new(&id).at(600).user("start")]);
    let all = ws.sessions();
    assert_eq!(all.len(), 1);
    assert!(all[0].has_transcript);
}
