//! Multi-agent watch TUI: presenter snapshots, rendered buffers (80x24, 140x40)
//! and the keybinding reducer, on a synthetic workspace.
//!
//! The TUI is an always-visible navigator (scope → sessions → agent trees, with
//! folded groups) next to the content of the selected node and a message feed
//! scoped to it.

#[path = "support/watch_fixture.rs"]
mod fixture;

use std::time::{Duration, Instant};

use agtrace::presentation::presenters::watch::build_screen;
use agtrace::presentation::view_models::watch::{
    ContentVm, DetailSection, LaneWindow, NAV_OLDER, NAV_TOP, NavKind, NowVm, Pane, RowKind,
    Scroll, SectionMetrics, TOAST_TTL, Toast, UiState, Viewport, WatchScreenVm, fold_key,
};
use agtrace::presentation::views::watch::input::{
    Effect, action_for, action_in, apply, expire_toast, sync_selection,
};
use agtrace::presentation::views::watch::{measure, render, viewport};
use agtrace::watch::{SharedWorkspace, build};
use agtrace_sdk::workspace::WorkspaceView;
use chrono::FixedOffset;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

/// UI state as `watch` starts it: the top node selected, the navigator focused.
fn ui() -> UiState {
    UiState::new("project demo-project", FixedOffset::east_opt(0).unwrap())
}

/// The scenario spans 5 minutes: the 15m window shows its activity.
fn ui15() -> UiState {
    UiState {
        window: LaneWindow::M15,
        ..ui()
    }
}

fn screen(view: &WorkspaceView, ui: &UiState) -> WatchScreenVm {
    build_screen(view, ui, fixture::now())
}

/// Select a navigator node the way the reducer does (the content starts over).
fn select(ui: &mut UiState, key: &str) {
    ui.selected = Some(key.to_string());
    ui.root_detail = false;
    ui.detail_auto = true;
}

/// Build and draw one frame the way the TUI loop does (viewport, build, measure,
/// sync, render).
fn draw(view: &WorkspaceView, ui: &mut UiState, w: u16, h: u16) -> String {
    let area = Rect::new(0, 0, w, h);
    ui.viewport = viewport(area, ui);
    let vm = screen(view, ui);
    measure(area, &vm, &mut ui.viewport);
    sync_selection(ui, &vm);
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| render(f, &vm)).unwrap();
    t.backend().to_string()
}

/// `depth kind label [flags]` per navigator row.
fn nav(vm: &WatchScreenVm) -> Vec<String> {
    vm.nav
        .rows
        .iter()
        .map(|r| {
            let open = match (r.expandable, r.expanded) {
                (true, true) => "▾",
                (true, false) => "▸",
                _ => " ",
            };
            format!(
                "{}{open} {:?} {}{}",
                "  ".repeat(r.depth as usize),
                r.kind,
                r.label,
                if r.selected { "  ◀" } else { "" }
            )
        })
        .collect()
}

fn keys(vm: &WatchScreenVm) -> Vec<&str> {
    vm.nav.rows.iter().map(|r| r.key.as_str()).collect()
}

fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

/// One key against the frame the current state renders (as the TUI loop does).
fn press(view: &WorkspaceView, ui: &mut UiState, code: KeyCode) -> Effect {
    let vm = screen(view, ui);
    sync_selection(ui, &vm);
    let action = action_in(ui, key(code)).expect("bound key");
    apply(ui, action, &vm, Instant::now())
}

fn presses(view: &WorkspaceView, ui: &mut UiState, codes: &[KeyCode]) {
    for c in codes {
        press(view, ui, *c);
    }
}

fn toast(ui: &UiState) -> &str {
    ui.toast.as_ref().map(|t| t.text.as_str()).unwrap_or("")
}

fn selected(view: &WorkspaceView, ui: &UiState) -> String {
    screen(view, ui).selected_row().unwrap().key.clone()
}

use KeyCode::{Down, Enter, Esc, Left, Right, Up};

// ------------------------------------------------------------------ navigator

/// Default navigator: the scope, then the sessions (collapsed while there are
/// several); the top node is selected and shows the overview.
#[test]
fn presenter_default_screen() {
    let view = fixture::workspace();
    let vm = screen(&view, &ui());
    assert_eq!(
        nav(&vm),
        [
            "▾ Top demo-project  ◀",
            "  ▸ Session Audit the parser with a team of reviewers",
            "  ▸ Session triage the flaky tests",
        ]
    );
    assert!(matches!(vm.content, ContentVm::Overview));
    assert_eq!(vm.feed_scope, "all");
    assert_eq!(vm.status.crumbs, ["demo-project"]);
    insta::assert_json_snapshot!(vm);
}

/// Expanded trees: a single finished child stays in place; many finished children
/// (and the session's earlier transcripts) fold into one group node; older
/// sessions collapse into one node; `d` shows everything in place.
#[test]
fn navigator_folds_finished_children_and_older_sessions() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    for id in ["claude:s-lead", "codex:t-root", "claude:c-team"] {
        ui.open.insert(id.to_string(), true);
    }
    let vm = screen(&view, &ui);
    insta::assert_snapshot!("navigator_many_sessions", nav(&vm).join("\n"));
    let group = vm
        .nav
        .rows
        .iter()
        .find(|r| r.key == fold_key("claude:c-team"))
        .expect("fold group");
    assert_eq!(group.label, "⊘ 8 killed · ✓ 3 done · 1 earlier (d)");
    assert_eq!(group.parent.as_deref(), Some("claude:c-team"));
    assert!(
        vm.nav
            .rows
            .iter()
            .any(|r| r.label == "explore call sites" && r.kind == NavKind::Agent),
        "a single finished subagent stays in place"
    );
    assert!(!vm.nav.rows.iter().any(|r| r.label == "step k0"));
    assert_eq!(vm.status.folded, 12, "11 steps and the /clear stub");

    // Expanding the group lists its items below it.
    ui.open.insert(fold_key("claude:c-team"), true);
    let vm = screen(&view, &ui);
    let k0 = vm.nav.rows.iter().find(|r| r.label == "step k0").unwrap();
    assert_eq!(
        k0.parent.as_deref(),
        Some(fold_key("claude:c-team").as_str())
    );
    // The older sessions group.
    ui.open.insert(NAV_OLDER.to_string(), true);
    let vm = screen(&view, &ui);
    let old = vm.nav.rows.iter().find(|r| r.key == "codex:x-old").unwrap();
    assert_eq!(old.parent.as_deref(), Some(NAV_OLDER));

    // `d`: no groups, finished agents in place.
    ui.show_done = true;
    let vm = screen(&view, &ui);
    assert!(!vm.nav.rows.iter().any(|r| r.kind == NavKind::Fold));
    assert!(vm.nav.rows.iter().any(|r| r.label == "step k0"));
    assert_eq!(vm.status.folded, 0);
}

/// A transcript continued in another one is shown under its continuation and
/// marked as the earlier transcript (both usually carry the same title).
#[test]
fn continued_transcript_row_is_nested_and_marked() {
    use agtrace_sdk::types::AgentAttributeKey;
    use agtrace_sdk::workspace::WorkspaceEvent;
    use agtrace_testing::synth::{AgentBuilder, EventLog};

    let mut view = WorkspaceView::new();
    let mut apply = |e: WorkspaceEvent| view.apply(e, &fixture::resolve, fixture::now());
    let old = AgentBuilder::claude_main("s-old")
        .name("Same title")
        .started(0);
    let new = AgentBuilder::claude_main("s-new")
        .name("Same title")
        .started(0);
    let old_id = old.id();
    apply(WorkspaceEvent::AgentDiscovered(old.build()));
    apply(WorkspaceEvent::AgentDiscovered(new.build()));
    apply(WorkspaceEvent::Events {
        agent: old_id.clone(),
        events: vec![
            EventLog::new(&old_id)
                .at(1)
                .attribute(AgentAttributeKey::ContinuedIn, "s-new"),
        ],
        reset: false,
    });
    // One session: open by default.
    let vm = screen(&view, &ui());
    assert_eq!(
        nav(&vm),
        [
            "▾ Top demo-project  ◀",
            "  ▾ Session Same title",
            "      Agent earlier transcript · Same title",
        ]
    );
}

/// Sessions presenter: live first (busy, idle incl. a process without a
/// transcript), then recent and older; names from the best source; the `/clear`
/// stub folded into its session instead of listed.
#[test]
fn presenter_sessions_order_names_and_folding() {
    let view = fixture::many_sessions();
    let vm = screen(&view, &ui());
    let rows: Vec<(String, String, bool, bool)> = vm
        .sessions
        .rows
        .iter()
        .map(|r| (r.id.clone(), r.name.clone(), r.bg, r.has_transcript))
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "claude:s-lead".into(),
                "Audit the parser with a team of reviewers".into(),
                false,
                true
            ),
            (
                "codex:t-root".into(),
                "triage the flaky tests".into(),
                false,
                true
            ),
            (
                "claude:c-team".into(),
                "Ship the release".into(),
                true,
                true
            ),
            ("claude:f00dcafe-job".into(), "f00dcafe".into(), true, false),
            (
                "claude:c-done".into(),
                "rename the config keys".into(),
                false,
                true
            ),
            (
                "codex:x-old".into(),
                "bump the lockfile".into(),
                false,
                true
            ),
        ]
    );
    assert_eq!(
        (vm.sessions.live, vm.sessions.recent, vm.sessions.older),
        (4, 1, 1)
    );
    // The busy step of the team session is what the session does now.
    let team = &vm.sessions.rows[2];
    assert_eq!(team.agents, 14, "lead, 12 subagents, the folded stub");
    assert!(matches!(&team.now, NowVm::Tool { name, .. } if name == "Bash"));
    assert_eq!(vm.status.sessions, 6);
    assert_eq!(vm.status.live, 4);
    // The navigator lists the live and recent sessions, the older one in a group.
    assert_eq!(
        keys(&vm),
        [
            NAV_TOP,
            "claude:s-lead",
            "codex:t-root",
            "claude:c-team",
            "claude:f00dcafe-job",
            "claude:c-done",
            NAV_OLDER
        ]
    );
    insta::assert_json_snapshot!(vm.sessions);
}

// ------------------------------------------------------------------ reducer: ←/→/↑/↓

/// ↑/↓ move through the rows and the content follows at once; → expands, then
/// goes to the first child; → on a leaf agent focuses the content; ← (content)
/// returns to the navigator; ← collapses, then goes to the parent.
#[test]
fn arrows_walk_the_hierarchy() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    press(&view, &mut ui, Down);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));
    let vm = screen(&view, &ui);
    assert!(matches!(&vm.content, ContentVm::Session { id, .. } if id == "claude:s-lead"));
    assert_eq!(vm.feed_scope, "this session");

    // → expands the collapsed session (toast), → again goes to its first child.
    press(&view, &mut ui, Right);
    assert_eq!(ui.open.get("claude:s-lead"), Some(&true));
    assert_eq!(
        toast(&ui),
        "▾ expanded Audit the parser with a team of reviewers"
    );
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));
    press(&view, &mut ui, Right);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-audit-a"));
    let vm = screen(&view, &ui);
    assert!(matches!(&vm.content, ContentVm::Agent { id } if id == "claude:s-audit-a"));
    assert_eq!(vm.detail.as_ref().unwrap().title, "audit-A");
    assert_eq!(
        vm.status.crumbs,
        [
            "demo-project",
            "Audit the parser with a team of reviewers",
            "audit-A"
        ]
    );

    // A leaf: → reads it (content focus); ← goes back to the navigator.
    press(&view, &mut ui, Right);
    assert_eq!(ui.focus, Pane::Content);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-audit-a"));
    press(&view, &mut ui, Left);
    assert_eq!(ui.focus, Pane::Navigator);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-audit-a"));

    // ← on a leaf: to the parent; ← on an expanded node: collapse; ← again: parent.
    press(&view, &mut ui, Left);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));
    press(&view, &mut ui, Left);
    assert_eq!(ui.open.get("claude:s-lead"), Some(&false));
    assert_eq!(
        toast(&ui),
        "▸ collapsed Audit the parser with a team of reviewers"
    );
    assert_eq!(screen(&view, &ui).nav.rows.len(), 3);
    press(&view, &mut ui, Left);
    assert_eq!(ui.selected.as_deref(), Some(NAV_TOP));
    // The top node has no parent and stays open.
    press(&view, &mut ui, Left);
    assert_eq!(ui.selected.as_deref(), Some(NAV_TOP));
    // → on the top node goes to the first session.
    press(&view, &mut ui, Right);
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));

    // ↑/↓ saturate at the ends.
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.selected.as_deref(), Some("codex:t-root"));
    press(&view, &mut ui, Down);
    assert_eq!(ui.selected.as_deref(), Some("codex:t-root"));
    press(&view, &mut ui, KeyCode::Char('g'));
    press(&view, &mut ui, Up);
    assert_eq!(ui.selected.as_deref(), Some(NAV_TOP));
}

/// Enter focuses the content (read / scroll); Esc goes back to the navigator,
/// then to the top node, then resets the view toggles; Tab / S-Tab cycle the
/// navigator, the content and the messages.
#[test]
fn enter_esc_and_tab_move_the_focus() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    ui.open.insert("codex:t-root".to_string(), true);
    select(&mut ui, "codex:t-judge");
    press(&view, &mut ui, Enter);
    assert_eq!(ui.focus, Pane::Content);
    press(&view, &mut ui, Esc);
    assert_eq!(ui.focus, Pane::Navigator);
    assert_eq!(ui.selected.as_deref(), Some("codex:t-judge"));
    press(&view, &mut ui, Esc);
    assert_eq!(
        ui.selected.as_deref(),
        Some(NAV_TOP),
        "Esc: to the top node"
    );
    ui.show_done = true;
    press(&view, &mut ui, Esc);
    assert!(!ui.show_done && ui.open.is_empty());
    assert_eq!(toast(&ui), "view reset");

    for (code, want) in [
        (KeyCode::Tab, Pane::Content),
        (KeyCode::Tab, Pane::Feed),
        (KeyCode::Tab, Pane::Navigator),
        (KeyCode::BackTab, Pane::Feed),
    ] {
        press(&view, &mut ui, code);
        assert_eq!(ui.focus, want, "{code:?}");
    }
    // ← from the messages goes back to the navigator too.
    press(&view, &mut ui, Left);
    assert_eq!(ui.focus, Pane::Navigator);
}

/// Regression: keys pressed between two frames are applied against the same
/// (last drawn) screen; `j j` must still move two rows, and → → must expand and
/// then descend once the children are drawn.
#[test]
fn moves_between_frames_accumulate() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    let vm = screen(&view, &ui);
    sync_selection(&mut ui, &vm);
    let now = Instant::now();
    let hit = |ui: &mut UiState, vm: &WatchScreenVm, code| {
        apply(ui, action_for(key(code)).unwrap(), vm, now)
    };
    hit(&mut ui, &vm, Down);
    hit(&mut ui, &vm, Down);
    assert_eq!(ui.selected.as_deref(), Some("codex:t-root"));
    // → → within one frame: the second one waits for the children to be drawn.
    hit(&mut ui, &vm, Right);
    hit(&mut ui, &vm, Right);
    assert_eq!(ui.selected.as_deref(), Some("codex:t-root"));
    let vm = screen(&view, &ui);
    hit(&mut ui, &vm, Right);
    assert_eq!(ui.selected.as_deref(), Some("codex:t-judge"));
    // space space folds and unfolds within one frame.
    hit(&mut ui, &vm, KeyCode::Char(' '));
    hit(&mut ui, &vm, KeyCode::Char(' '));
    assert_eq!(ui.open.get("codex:t-judge"), Some(&true));
}

/// A selected agent that becomes hidden (its session collapsed, `d` toggled, its
/// group folded) moves to its nearest shown ancestor or group.
#[test]
fn hidden_selection_moves_to_what_is_shown() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    ui.open.insert("claude:c-team".to_string(), true);
    ui.show_done = true;
    select(&mut ui, "claude:c-team/k3");
    assert_eq!(selected(&view, &ui), "claude:c-team/k3");
    ui.show_done = false;
    assert_eq!(selected(&view, &ui), fold_key("claude:c-team"));
    ui.open.insert("claude:c-team".to_string(), false);
    assert_eq!(selected(&view, &ui), "claude:c-team");
    select(&mut ui, "codex:x-old");
    assert_eq!(selected(&view, &ui), NAV_OLDER);
    select(&mut ui, "claude:gone");
    assert_eq!(selected(&view, &ui), NAV_TOP);
}

// ------------------------------------------------------------------ content by node

/// The content pane follows the selected node: top overview, session overview,
/// agent detail, a folded group's items, the older sessions.
#[test]
fn content_follows_the_selected_node() {
    let view = fixture::many_sessions();
    let mut ui = ui15();
    ui.open.insert("claude:c-team".to_string(), true);
    let vm = screen(&view, &ui);
    let ov = vm.overview.as_ref().unwrap();
    assert_eq!(ov.older_hidden, 1);
    let headers: Vec<&str> = ov
        .rows
        .iter()
        .filter_map(|r| r.root.as_ref().map(|h| h.label.as_str()))
        .collect();
    assert_eq!(
        headers,
        [
            "Audit the parser with a team of reviewers",
            "triage the flaky tests",
            "Ship the release",
            "rename the config keys"
        ]
    );

    select(&mut ui, "claude:c-team");
    let vm = screen(&view, &ui);
    assert!(
        matches!(&vm.content, ContentVm::Session { id, has_transcript: true } if id == "claude:c-team")
    );
    let ov = vm.overview.as_ref().unwrap();
    let ids: Vec<&str> = ov.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["claude:c-team", "claude:c-team/r0"]);
    let h = ov.rows[0].root.as_ref().unwrap();
    assert_eq!(
        (h.folded.killed, h.folded.done, h.folded.transcripts),
        (8, 3, 1)
    );

    select(&mut ui, &fold_key("claude:c-team"));
    let vm = screen(&view, &ui);
    assert!(
        matches!(&vm.content, ContentVm::Fold { parent, folded } if parent == "claude:c-team" && folded.total() == 12)
    );
    assert_eq!(vm.overview.as_ref().unwrap().rows.len(), 12);
    assert_eq!(vm.feed_scope, "folded agents");
    assert!(!vm.feed.is_empty());
    assert!(
        vm.feed
            .iter()
            .all(|f| f.from.starts_with("step ") || f.to.iter().any(|t| t.starts_with("step "))),
        "{:?}",
        vm.feed
    );
    assert_eq!(
        vm.status.crumbs,
        [
            "demo-project",
            "Ship the release",
            "⊘ 8 killed · ✓ 3 done · 1 earlier (d)"
        ]
    );

    select(&mut ui, NAV_OLDER);
    let vm = screen(&view, &ui);
    assert!(matches!(vm.content, ContentVm::Older { count: 1 }));

    select(&mut ui, "claude:c-team/r0");
    let vm = screen(&view, &ui);
    assert!(matches!(&vm.content, ContentVm::Agent { id } if id == "claude:c-team/r0"));
    assert_eq!(vm.detail.as_ref().unwrap().title, "step r0");
}

/// The feed is scoped to the selection: everything (top), the session's
/// messages, or those sent / received / emitted by the agent.
#[test]
fn feed_is_scoped_to_the_selection() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    let all = screen(&view, &ui).feed.len();
    assert_eq!(all, view.feed.len());

    select(&mut ui, "codex:t-root");
    let vm = screen(&view, &ui);
    assert!(!vm.feed.is_empty() && vm.feed.len() < all);
    assert!(
        vm.feed.iter().all(|f| f.from.starts_with("/root")),
        "{:?}",
        vm.feed
    );

    ui.open.insert("claude:s-lead".to_string(), true);
    select(&mut ui, "claude:s-audit-a");
    let vm = screen(&view, &ui);
    assert_eq!(vm.feed_scope, "audit-A");
    assert!(!vm.feed.is_empty());
    assert!(
        vm.feed
            .iter()
            .all(|f| f.from == "audit-A" || f.to.iter().any(|t| t == "audit-A")),
        "{:?}",
        vm.feed
    );
}

#[test]
fn encrypted_bodies_are_never_shown() {
    let view = fixture::workspace();
    for id in [NAV_TOP, "codex:t-judge", "codex:t-root"] {
        let mut ui = self::ui();
        ui.open.insert("codex:t-root".to_string(), true);
        select(&mut ui, id);
        let vm = screen(&view, &ui);
        let enc: Vec<_> = vm.feed.iter().filter(|r| r.encrypted).collect();
        assert!(!enc.is_empty(), "{id}");
        assert!(enc.iter().all(|r| r.text.is_none()));
        for row in vm.detail.iter().flat_map(|d| &d.timeline) {
            if matches!(row.kind, RowKind::MessageIn | RowKind::MessageOut)
                && row.label.starts_with("/root")
                && !row.text.contains("FINAL_ANSWER")
            {
                assert!(row.text.ends_with("[encrypted]"), "{row:?}");
            }
        }
    }
    // FINAL_ANSWER is plaintext.
    let vm = screen(&view, &ui());
    assert!(
        vm.feed
            .iter()
            .any(|r| r.tag == "FINAL_ANSWER"
                && r.text.as_deref().is_some_and(|t| t.contains("flaky")))
    );
}

#[test]
fn timeline_shows_only_selected_system_events() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    select(&mut ui, "claude:s-lead");
    press(&view, &mut ui, KeyCode::Char('t'));
    let vm = screen(&view, &ui);
    let d = vm.detail.expect("root detail");
    let text: Vec<String> = d
        .timeline
        .iter()
        .map(|r| format!("{} {}", r.label, r.text))
        .collect();
    // Notifications (api_error) and plain queue noise are not shown.
    assert!(!text.iter().any(|t| t.contains("overloaded")));
    assert!(!text.iter().any(|t| t.contains("noise")));
    // Absorbed queued prompt, compaction, model change and interrupt are.
    for kind in [
        RowKind::Queued,
        RowKind::Compaction,
        RowKind::ModelChange,
        RowKind::Interrupt,
    ] {
        assert!(d.timeline.iter().any(|r| r.kind == kind), "{kind:?}");
    }
}

// ------------------------------------------------------------------ section keys

/// Section the detail opened on, once the presenter resolved it.
fn opened_section(view: &WorkspaceView, ui: &mut UiState) -> DetailSection {
    let vm = screen(view, ui);
    sync_selection(ui, &vm);
    assert!(!ui.detail_auto, "the resolved section is stored back");
    let d = vm.detail.expect("detail shown");
    assert_eq!(d.section, ui.detail_section);
    d.section
}

/// i / n / r / t show that section of the selected agent's detail and focus the
/// content: an agent itself, a session's root agent, the first session from the
/// top node, a folded group's first item; in the content they switch sections.
#[test]
fn section_keys_open_the_detail_of_the_selection() {
    let view = fixture::many_sessions();
    for (code, want) in [
        ('i', DetailSection::Instructions),
        ('n', DetailSection::Now),
        ('r', DetailSection::Result),
        ('t', DetailSection::Timeline),
    ] {
        let mut ui = self::ui();
        ui.open.insert("codex:t-root".to_string(), true);
        select(&mut ui, "codex:t-scout");
        assert_eq!(press(&view, &mut ui, KeyCode::Char(code)), Effect::None);
        assert_eq!(ui.focus, Pane::Content, "{code}");
        assert_eq!(ui.detail_section, want, "{code}");
        // Explicit: no presenter override, even for an empty section.
        assert_eq!(screen(&view, &ui).detail.unwrap().section, want);
    }

    // Session node: its root agent's detail (the navigator stays on the session).
    let mut ui = self::ui();
    select(&mut ui, "codex:t-root");
    press(&view, &mut ui, KeyCode::Char('n'));
    let vm = screen(&view, &ui);
    assert_eq!(vm.selected_row().unwrap().key, "codex:t-root");
    assert!(matches!(&vm.content, ContentVm::Agent { id } if id == "codex:t-root"));
    assert!(vm.detail.unwrap().title.contains("/root") || ui.root_detail);
    // In the content, the keys switch sections.
    press(&view, &mut ui, KeyCode::Char('t'));
    assert_eq!(ui.detail_section, DetailSection::Timeline);
    assert_eq!(ui.focus, Pane::Content);
    // Moving the selection shows the next node's own content again.
    press(&view, &mut ui, Esc);
    press(&view, &mut ui, Up);
    press(&view, &mut ui, Down);
    assert!(matches!(
        screen(&view, &ui).content,
        ContentVm::Session { .. }
    ));

    // Top node: the first session's root.
    let mut ui = self::ui();
    press(&view, &mut ui, KeyCode::Char('i'));
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));
    assert!(matches!(
        screen(&view, &ui).content,
        ContentVm::Agent { .. }
    ));

    // Folded group: its first item, the group opened.
    let mut ui = self::ui();
    ui.open.insert("claude:c-team".to_string(), true);
    select(&mut ui, &fold_key("claude:c-team"));
    press(&view, &mut ui, KeyCode::Char('r'));
    // Its first item in tree order: the earlier `/clear` transcript.
    assert_eq!(ui.selected.as_deref(), Some("claude:c-team-clear"));
    assert_eq!(selected(&view, &ui), "claude:c-team-clear");
    assert_eq!(ui.open.get(&fold_key("claude:c-team")), Some(&true));

    // A process without a transcript has no detail.
    let mut ui = self::ui();
    select(&mut ui, "claude:f00dcafe-job");
    press(&view, &mut ui, KeyCode::Char('n'));
    assert_eq!(toast(&ui), "f00dcafe has no transcript yet");
    assert_eq!(ui.focus, Pane::Navigator);
}

/// Regression (user report: "the timeline isn't fully visible when I open the
/// detail"): the detail opens where the agent's state is: Now while running, the
/// result once ended with one, else the timeline; never an empty section.
#[test]
fn detail_opens_on_a_sensible_section() {
    let view = fixture::workspace();
    for (id, want) in [
        ("claude:s-audit-a", DetailSection::Timeline),
        ("claude:s-lead/a7ac2e91", DetailSection::Result),
        ("codex:t-judge", DetailSection::Now),
        ("codex:t-scout", DetailSection::Result),
    ] {
        let mut ui = self::ui();
        ui.open.insert("claude:s-lead".to_string(), true);
        ui.open.insert("codex:t-root".to_string(), true);
        select(&mut ui, id);
        assert_eq!(opened_section(&view, &mut ui), want, "{id}");
    }
}

/// The section picked with i / n / r / t is kept for the next agents' details,
/// unless that agent has nothing there.
#[test]
fn detail_section_is_remembered_across_agents() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    ui.open.insert("claude:s-lead".to_string(), true);
    ui.open.insert("codex:t-root".to_string(), true);
    select(&mut ui, "codex:t-judge");
    press(&view, &mut ui, KeyCode::Char('t'));
    assert_eq!(ui.detail_section, DetailSection::Timeline);
    press(&view, &mut ui, Esc);
    select(&mut ui, "claude:s-lead/a7ac2e91");
    assert_eq!(opened_section(&view, &mut ui), DetailSection::Timeline);

    press(&view, &mut ui, KeyCode::Char('r'));
    press(&view, &mut ui, Esc);
    select(&mut ui, "codex:t-scout");
    assert_eq!(opened_section(&view, &mut ui), DetailSection::Result);

    // A running agent has no result yet: the default applies, the choice is kept.
    select(&mut ui, "claude:s-lead");
    ui.root_detail = true;
    assert_eq!(opened_section(&view, &mut ui), DetailSection::Now);
    assert_eq!(ui.detail_pref, Some(DetailSection::Result));
}

// ------------------------------------------------------------------ scrolling

/// Content focus: j/k/PgUp/PgDn/G/g scroll the detail section (clamped to the
/// last frame's wrapped text) or the overview lines; messages scroll too.
#[test]
fn content_and_feed_scroll() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    ui.open.insert("claude:s-lead".to_string(), true);
    select(&mut ui, "claude:s-lead/a7ac2e91");
    press(&view, &mut ui, KeyCode::Char('i'));
    ui.viewport.detail = [
        SectionMetrics {
            total: 10,
            height: 4,
        },
        SectionMetrics {
            total: 2,
            height: 3,
        },
        SectionMetrics {
            total: 1,
            height: 1,
        },
        SectionMetrics {
            total: 20,
            height: 5,
        },
    ];
    press(&view, &mut ui, KeyCode::Char('j'));
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(1));
    press(&view, &mut ui, KeyCode::PageDown);
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(5));
    presses(&view, &mut ui, &[Down, Down]);
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(6), "clamped at the end");
    press(&view, &mut ui, KeyCode::Char('g'));
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(0));
    press(&view, &mut ui, KeyCode::Char('t'));
    assert_eq!(ui.detail_scroll[3], Scroll::Follow);
    press(&view, &mut ui, KeyCode::Char('k'));
    assert_eq!(ui.detail_scroll[3], Scroll::Offset(14));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.detail_scroll[3], Scroll::Follow);
    assert_eq!(
        ui.selected.as_deref(),
        Some("claude:s-lead/a7ac2e91"),
        "no move"
    );

    // Overview lines (top node).
    let mut ui = self::ui();
    ui.show_done = true;
    let out = draw(&view, &mut ui, 80, 24);
    let m = ui.viewport.content;
    assert!(m.total > m.height, "{m:?}\n{out}");
    press(&view, &mut ui, Enter);
    press(&view, &mut ui, KeyCode::PageDown);
    assert_eq!(ui.content_scroll, m.height.min(m.total - m.height));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.content_scroll, m.total - m.height);
    let out = draw(&view, &mut ui, 80, 24);
    assert!(out.contains('↑'), "{out}");
    press(&view, &mut ui, KeyCode::Char('g'));
    assert_eq!(ui.content_scroll, 0);
    assert_eq!(ui.selected.as_deref(), Some(NAV_TOP));

    // Messages.
    ui.viewport.feed = 3;
    presses(&view, &mut ui, &[KeyCode::Tab, KeyCode::Char('k')]);
    assert_eq!(ui.focus, Pane::Feed);
    assert!(matches!(ui.feed_scroll, Scroll::Offset(_)));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.feed_scroll, Scroll::Follow);
}

/// Navigator focus: PgUp/PgDn page, C-u/C-d half-page, g/G first/last row.
#[test]
fn navigator_pages() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    ui.show_done = true;
    for id in ["claude:s-lead", "codex:t-root", "claude:c-team"] {
        ui.open.insert(id.to_string(), true);
    }
    ui.viewport = Viewport {
        nav: 6,
        ..Default::default()
    };
    let order: Vec<String> = keys(&screen(&view, &ui))
        .into_iter()
        .map(str::to_string)
        .collect();
    press(&view, &mut ui, KeyCode::PageDown);
    assert_eq!(ui.selected.as_deref(), Some(order[6].as_str()));
    let vm = screen(&view, &ui);
    apply(
        &mut ui,
        action_for(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)).unwrap(),
        &vm,
        Instant::now(),
    );
    assert_eq!(ui.selected.as_deref(), Some(order[3].as_str()));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.selected.as_deref(), order.last().map(String::as_str));
    press(&view, &mut ui, KeyCode::PageUp);
    assert_eq!(
        ui.selected.as_deref(),
        Some(order[order.len() - 7].as_str())
    );
}

// ------------------------------------------------------------------ filter / toggles

/// `/` filters the navigator (matches plus their ancestors, folds and collapses
/// ignored), selects the first match, ↑/↓ move, Enter keeps the filter, Esc
/// clears it.
#[test]
fn slash_filters_the_navigator() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    press(&view, &mut ui, KeyCode::Char('/'));
    assert!(ui.filter_editing);
    // Keys are text now: `D` / `A` do not toggle anything, `q` does not quit.
    for c in "STEP D".chars() {
        assert_eq!(press(&view, &mut ui, KeyCode::Char(c)), Effect::None);
    }
    assert!(!ui.show_done && !ui.auto_select);
    let vm = screen(&view, &ui);
    sync_selection(&mut ui, &vm);
    assert_eq!(
        keys(&vm),
        [
            NAV_TOP,
            "claude:c-team",
            "claude:c-team/d0",
            "claude:c-team/d1",
            "claude:c-team/d2"
        ],
        "case-insensitive match, ancestors kept, folds ignored"
    );
    assert_eq!(vm.status.matches, 3);
    assert_eq!(ui.selected.as_deref(), Some("claude:c-team/d0"));
    press(&view, &mut ui, Down);
    assert_eq!(ui.selected.as_deref(), Some("claude:c-team/d1"));
    press(&view, &mut ui, KeyCode::Backspace);
    assert_eq!(ui.filter, "STEP ");

    // Enter keeps the filter; Esc then clears it before anything else.
    press(&view, &mut ui, Enter);
    assert!(!ui.filter_editing);
    assert_eq!(
        screen(&view, &ui).nav.rows.len(),
        14,
        "top, session, 12 steps"
    );
    press(&view, &mut ui, Esc);
    assert!(ui.filter.is_empty());
    assert_eq!(toast(&ui), "filter cleared");

    // Esc while typing drops the filter; no match leaves only the top node.
    presses(
        &view,
        &mut ui,
        &[KeyCode::Char('/'), KeyCode::Char('z'), KeyCode::Char('z')],
    );
    let vm = screen(&view, &ui);
    assert_eq!(keys(&vm), [NAV_TOP]);
    assert!(draw(&view, &mut ui, 80, 24).contains("no match"));
    press(&view, &mut ui, Esc);
    assert!(!ui.filter_editing && ui.filter.is_empty());
}

/// `d` shows / folds finished agents, `A` follows the most active agent, `s`
/// hides the navigator on narrow terminals only, `?` swallows keys.
#[test]
fn toggles_raise_toasts_and_help_swallows_keys() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    press(&view, &mut ui, KeyCode::Char('d'));
    assert!(ui.show_done);
    assert_eq!(toast(&ui), "finished agents shown in place — d to fold");
    press(&view, &mut ui, KeyCode::Char('d'));
    assert_eq!(toast(&ui), "finished agents folded — d to show");

    press(&view, &mut ui, KeyCode::Char('A'));
    assert!(ui.auto_select);
    assert_eq!(toast(&ui), "follow the most active agent: on");
    // The most recently active node shown: the lead's session (Bash at 12:04:50).
    assert_eq!(selected(&view, &ui), "claude:s-lead");
    press(&view, &mut ui, KeyCode::Char('A'));
    assert_eq!(toast(&ui), "follow the most active agent: off");

    press(&view, &mut ui, KeyCode::Char('s'));
    assert!(!ui.nav_hidden);
    assert_eq!(toast(&ui), "the navigator hides only below 80 columns");
    ui.viewport.narrow = true;
    press(&view, &mut ui, KeyCode::Char('s'));
    assert!(ui.nav_hidden);
    assert_eq!(ui.focus, Pane::Content);
    assert_eq!(toast(&ui), "navigator hidden — s to show");

    press(&view, &mut ui, KeyCode::Char('?'));
    assert!(ui.show_help);
    press(&view, &mut ui, KeyCode::Char('d'));
    press(&view, &mut ui, KeyCode::Char(' '));
    assert!(!ui.show_done && ui.open.is_empty());
    press(&view, &mut ui, Esc);
    assert!(!ui.show_help);
    assert_eq!(press(&view, &mut ui, KeyCode::Char('R')), Effect::Rescan);
    assert_eq!(toast(&ui), "rescanning…");
    assert_eq!(press(&view, &mut ui, KeyCode::Char('q')), Effect::Quit);
}

/// space expands / collapses like → / ←, and explains when it cannot.
#[test]
fn space_toggles_and_explains() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    press(&view, &mut ui, KeyCode::Char(' '));
    assert_eq!(toast(&ui), "the top node is always open");
    press(&view, &mut ui, Down);
    press(&view, &mut ui, KeyCode::Char(' '));
    assert_eq!(ui.open.get("claude:s-lead"), Some(&true));
    select(&mut ui, "claude:s-audit-a");
    press(&view, &mut ui, KeyCode::Char(' '));
    assert_eq!(toast(&ui), "audit-A has no children");
}

/// `+` / `-` (and `]` / `[`) step the overview activity window and say so.
#[test]
fn window_keys_zoom_the_activity_lanes() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    assert_eq!(ui.window, LaneWindow::M60);
    press(&view, &mut ui, KeyCode::Char('+'));
    assert_eq!(ui.window, LaneWindow::H4);
    assert_eq!(toast(&ui), "activity window: last 4h");
    press(&view, &mut ui, KeyCode::Char(']'));
    assert_eq!(ui.window, LaneWindow::All);
    press(&view, &mut ui, KeyCode::Char('+'));
    assert_eq!(
        toast(&ui),
        "activity window: all (since the oldest agent started) — widest"
    );
    for _ in 0..3 {
        press(&view, &mut ui, KeyCode::Char('-'));
    }
    assert_eq!(ui.window, LaneWindow::M15);
    press(&view, &mut ui, KeyCode::Char('['));
    assert_eq!(toast(&ui), "activity window: last 15m — narrowest");
}

#[test]
fn toast_expires() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    let now = Instant::now();
    ui.toast = Some(Toast::new("view reset", now));
    assert_eq!(screen(&view, &ui).toast.as_deref(), Some("view reset"));
    assert!(!expire_toast(&mut ui, now + Duration::from_millis(1500)));
    assert!(ui.toast.is_some());
    let old = now
        .checked_sub(TOAST_TTL + Duration::from_millis(1))
        .expect("monotonic clock past the TTL");
    ui.toast = Some(Toast::new("view reset", old));
    assert!(screen(&view, &ui).toast.is_none());
    assert!(expire_toast(&mut ui, now));
    assert!(ui.toast.is_none());
    assert!(!expire_toast(&mut ui, now));
}

// ------------------------------------------------------------------ overview / detail presenters

/// Overview rows: two roots with headers, lanes with compaction markers, and the
/// "now" cell for running / idle / done agents.
#[test]
fn presenter_overview_rows() {
    let view = fixture::workspace();
    let mut ui = ui15();
    ui.viewport = viewport(Rect::new(0, 0, 140, 40), &ui);
    let vm = screen(&view, &ui);
    let ov = vm.overview.expect("overview");
    assert_eq!(ov.cell, "1m");
    let roots: Vec<&str> = ov
        .rows
        .iter()
        .filter_map(|r| r.root.as_ref().map(|h| h.label.as_str()))
        .collect();
    assert_eq!(
        roots,
        vec![
            "Audit the parser with a team of reviewers",
            "triage the flaky tests"
        ]
    );
    for r in &ov.rows {
        assert_eq!(
            r.lane.chars().count(),
            r.lane_tones.chars().count(),
            "{}",
            r.id
        );
    }
    let lead = ov.rows.iter().find(|r| r.id == "claude:s-lead").unwrap();
    assert!(
        lead.lane.contains('⟲'),
        "compaction marker: {:?}",
        lead.lane
    );
    insta::assert_json_snapshot!(ov);
}

#[test]
fn overview_lane_follows_the_window() {
    let view = fixture::workspace();
    let mut ui = ui15();
    ui.viewport = viewport(Rect::new(0, 0, 140, 40), &ui);
    let cols = ui.viewport.lane_cols;
    assert!(cols >= 30, "{cols}");
    for (w, cell) in [
        (LaneWindow::M15, "1m"),
        (LaneWindow::M60, "2m"),
        (LaneWindow::H4, "8m"),
        (LaneWindow::All, "1m"),
    ] {
        ui.window = w;
        let ov = screen(&view, &ui).overview.unwrap();
        assert_eq!(ov.cell, cell, "{w:?}");
        assert!(ov.rows.iter().all(|r| r.lane.chars().count() <= cols));
    }
}

/// Regression (real data): a session that went quiet and is done by staleness
/// (not recorded in the signal history) was drawn "running" (`·`) until now.
#[test]
fn overview_lane_stops_when_a_quiet_session_is_done() {
    use agtrace::presentation::view_models::watch::StatusVm;
    use agtrace_sdk::workspace::WorkspaceEvent;
    use agtrace_testing::synth::{AgentBuilder, EventLog, ts};

    let now = ts(3 * 3600);
    let mut view = WorkspaceView::new();
    let lead = AgentBuilder::claude_main("s-quiet").started(0);
    let id = lead.id();
    view.apply(
        WorkspaceEvent::AgentDiscovered(lead.build()),
        &fixture::resolve,
        now,
    );
    let mut l = EventLog::new(&id);
    view.apply(
        WorkspaceEvent::Events {
            agent: id.clone(),
            events: vec![l.at(0).user("go"), l.at(30).assistant("working")],
            reset: false,
        },
        &fixture::resolve,
        now,
    );
    assert_eq!(
        view.agents[&id].status,
        agtrace_sdk::workspace::AgentStatus::Done
    );
    let mut ui = UiState {
        window: LaneWindow::H4,
        ..ui()
    };
    ui.viewport = viewport(Rect::new(0, 0, 140, 40), &ui);
    let ov = build_screen(&view, &ui, now).overview.unwrap();
    let row = &ov.rows[0];
    assert_eq!(row.status, StatusVm::Done);
    assert!(!row.lane.contains('·'), "lane: {:?}", row.lane);
    assert!(row.lane.trim().starts_with('▁'), "lane: {:?}", row.lane);
}

/// Between tools, the overview's "now" shows the task in progress (its active
/// form) instead of the last text; an open tool still wins.
#[test]
fn overview_now_prefers_the_task_in_progress_between_tools() {
    use agtrace_sdk::types::{EventPayload, PlanItem, PlanItemStatus, PlanPayload};
    use agtrace_sdk::workspace::WorkspaceEvent;
    use agtrace_testing::synth::{AgentBuilder, EventLog};

    let mut view = WorkspaceView::new();
    let a = AgentBuilder::claude_main("s-t").started(0);
    let id = a.id();
    view.apply(
        WorkspaceEvent::AgentDiscovered(a.build()),
        &fixture::resolve,
        fixture::now(),
    );
    let mut l = EventLog::new(&id);
    let call = l.at(290).bash("cargo test");
    let feed = |view: &mut WorkspaceView, events| {
        view.apply(
            WorkspaceEvent::Events {
                agent: id.clone(),
                events,
                reset: false,
            },
            &fixture::resolve,
            fixture::now(),
        )
    };
    feed(
        &mut view,
        vec![
            l.at(280).user("fix it"),
            l.at(281).push(EventPayload::Plan(PlanPayload::Items {
                items: vec![
                    PlanItem {
                        id: None,
                        subject: "Read the code".into(),
                        active_form: Some("Reading the code".into()),
                        status: PlanItemStatus::Completed,
                    },
                    PlanItem {
                        id: None,
                        subject: "Run the tests".into(),
                        active_form: Some("Running the tests".into()),
                        status: PlanItemStatus::InProgress,
                    },
                ],
            })),
            l.at(282).assistant("Now the tests."),
            call.clone(),
        ],
    );
    let now_of = |view: &WorkspaceView| {
        let vm = screen(view, &ui15());
        vm.overview.unwrap().rows[0].now.clone()
    };
    assert!(
        matches!(now_of(&view), NowVm::Tool { ref name, .. } if name == "Bash"),
        "an open tool wins"
    );
    feed(&mut view, vec![l.at(295).tool_result(call.id, "ok", false)]);
    assert_eq!(
        now_of(&view),
        NowVm::Task {
            text: "Running the tests".into()
        }
    );
}

/// A killed agent without a reported result shows its last message and why it ended.
#[test]
fn detail_result_of_a_killed_agent_is_its_last_message() {
    use agtrace::presentation::view_models::watch::{ResultVm, StatusVm};
    use agtrace_sdk::types::{AgentLifecyclePayload, EventPayload, LifecycleTransition};
    use agtrace_sdk::workspace::WorkspaceEvent;
    use agtrace_testing::synth::{AgentBuilder, EventLog, handle};

    let mut view = WorkspaceView::new();
    let mut apply = |e: WorkspaceEvent| view.apply(e, &fixture::resolve, fixture::now());
    let lead = AgentBuilder::claude_main("s-k").started(0);
    let sub = AgentBuilder::claude_subagent("s-k", "a9").started(5);
    let (lead_id, sub_id) = (lead.id(), sub.id());
    apply(WorkspaceEvent::AgentDiscovered(lead.build()));
    apply(WorkspaceEvent::AgentDiscovered(sub.build()));
    let mut s = EventLog::new(&sub_id);
    apply(WorkspaceEvent::Events {
        agent: sub_id.clone(),
        events: vec![
            s.at(5).user("render stills"),
            s.at(9).assistant("report: /tmp/r.md"),
        ],
        reset: false,
    });
    let mut l = EventLog::new(&lead_id);
    apply(WorkspaceEvent::Events {
        agent: lead_id.clone(),
        events: vec![
            l.at(20)
                .push(EventPayload::AgentLifecycle(AgentLifecyclePayload {
                    target: handle::native("a9"),
                    transition: LifecycleTransition::Killed,
                    reason: Some("stopped by lead".to_string()),
                    usage: None,
                })),
        ],
        reset: false,
    });
    let mut ui = self::ui();
    select(&mut ui, sub_id.as_str());
    match screen(&view, &ui).detail.unwrap().result {
        ResultVm::LastMessage {
            text,
            status,
            reason,
            ..
        } => {
            assert_eq!(text, "report: /tmp/r.md");
            assert_eq!(status, StatusVm::Killed);
            assert_eq!(reason.as_deref(), Some("stopped by lead"));
        }
        other => panic!("{other:?}"),
    }
}

/// UI state with `id` selected (its trees open) and the content focused.
fn agent_ui(id: &str) -> UiState {
    let mut ui = self::ui();
    ui.open.insert("claude:s-lead".to_string(), true);
    ui.open.insert("codex:t-root".to_string(), true);
    select(&mut ui, id);
    ui.focus = Pane::Content;
    ui
}

/// Codex child: encrypted task (sender, path, no body) and a plaintext FINAL_ANSWER.
#[test]
fn presenter_detail_codex_child_encrypted_task_and_final_answer() {
    let view = fixture::workspace();
    let ui = agent_ui("codex:t-scout");
    let d = screen(&view, &ui).detail.unwrap();
    let task = &d.instructions[0];
    assert!(task.encrypted && task.text.is_none(), "{task:?}");
    assert!(
        task.note
            .as_deref()
            .is_some_and(|n| n.starts_with("[encrypted by Codex] NEW_TASK from /root")),
        "{task:?}"
    );
    insta::assert_json_snapshot!(d);
}

/// Claude lead: effort in the header; the team's task list in "Now", with the task
/// a teammate works on attributed to it.
#[test]
fn detail_claude_lead_plan() {
    use agtrace::presentation::view_models::watch::TaskStatusVm;
    let view = fixture::workspace();
    let mut ui = agent_ui("claude:s-lead");
    ui.root_detail = true;
    ui.detail_section = DetailSection::Now;
    ui.detail_auto = false;
    let vm = screen(&view, &ui);
    let d = vm.detail.as_ref().unwrap();
    assert_eq!(d.effort.as_deref(), Some("high"));
    let tasks: Vec<(TaskStatusVm, &str, Option<&str>)> = d
        .now
        .plan
        .tasks
        .iter()
        .map(|t| (t.status, t.text.as_str(), t.by.as_deref()))
        .collect();
    assert_eq!(
        tasks,
        vec![
            (
                TaskStatusVm::InProgress,
                "Reviewing the parser",
                Some("audit-A")
            ),
            (TaskStatusVm::Completed, "Scan for panics", Some("audit-B")),
            (TaskStatusVm::Pending, "Write the summary", None),
        ]
    );
    insta::assert_snapshot!(
        "render_detail_claude_lead_plan_140x40",
        draw(&view, &mut ui, 140, 40)
    );
}

/// Codex root: goal and plan-mode plan text in "Now".
#[test]
fn detail_codex_root_goal_and_plan() {
    let view = fixture::workspace();
    let mut ui = agent_ui("codex:t-root");
    ui.root_detail = true;
    ui.detail_section = DetailSection::Now;
    ui.detail_auto = false;
    let vm = screen(&view, &ui);
    let d = vm.detail.as_ref().unwrap();
    assert_eq!(d.effort.as_deref(), Some("medium"));
    assert_eq!(
        d.now.plan.goal,
        Some((
            "Make the watch tests deterministic".to_string(),
            Some("active".to_string())
        ))
    );
    assert!(d.now.plan.text.as_deref().unwrap().starts_with("# Triage"));
}

// ------------------------------------------------------------------ rendering

/// Top node: the navigator and the multi-session overview.
#[test]
fn render_top_node_80x24_and_140x40() {
    let view = fixture::many_sessions();
    insta::assert_snapshot!("render_top_80x24", draw(&view, &mut ui15(), 80, 24));
    let out = draw(&view, &mut ui15(), 140, 40);
    assert!(
        out.contains("▸ 1 older session — in the navigator"),
        "{out}"
    );
    assert!(
        out.contains("✓ 3 finished · ⊘ 8 killed · 1 earlier transcript  (d to show)"),
        "{out}"
    );
    assert!(!out.contains("step k0"), "{out}");
    insta::assert_snapshot!("render_top_140x40", out);
}

/// Session node: the session's overview (header, its agents, the folded line)
/// and its messages; the navigator shows it expanded with its folded group.
#[test]
fn render_session_node_80x24_and_140x40() {
    let view = fixture::many_sessions();
    let mut ui = ui15();
    ui.open.insert("claude:c-team".to_string(), true);
    select(&mut ui, "claude:c-team");
    let out = draw(&view, &mut ui, 140, 40);
    assert!(out.contains("Messages · this session"), "{out}");
    assert!(out.contains("▸ ⊘ 8 killed · ✓ 3 done"), "{out}");
    insta::assert_snapshot!("render_session_140x40", out);
    insta::assert_snapshot!("render_session_80x24", draw(&view, &mut ui, 80, 24));
}

/// Agent node: the agent detail next to the navigator; the breadcrumb names the
/// section.
#[test]
fn render_agent_node_80x24_and_140x40() {
    let view = fixture::workspace();
    let mut ui = agent_ui("claude:s-audit-a");
    let out = draw(&view, &mut ui, 140, 40);
    assert!(out.contains("audit-A · TIMELINE"), "{out}");
    insta::assert_snapshot!("render_agent_140x40", out);
    let mut ui = agent_ui("codex:t-scout");
    insta::assert_snapshot!("render_agent_80x24", draw(&view, &mut ui, 80, 24));
}

/// Folded group node: its items as overview rows; expanded in the navigator.
#[test]
fn render_fold_node_80x24_and_140x40() {
    let view = fixture::many_sessions();
    let mut ui = ui15();
    ui.open.insert("claude:c-team".to_string(), true);
    select(&mut ui, &fold_key("claude:c-team"));
    let out = draw(&view, &mut ui, 140, 40);
    assert!(out.contains("step k0"), "{out}");
    assert!(out.contains("under Ship the release"), "{out}");
    insta::assert_snapshot!("render_fold_140x40", out);
    ui.open.insert(fold_key("claude:c-team"), true);
    insta::assert_snapshot!("render_fold_expanded_80x24", draw(&view, &mut ui, 80, 24));
}

/// Older sessions node and a process without a transcript.
#[test]
fn render_older_and_process_only_sessions() {
    let view = fixture::many_sessions();
    let mut ui = self::ui();
    select(&mut ui, NAV_OLDER);
    let out = draw(&view, &mut ui, 100, 24);
    assert!(out.contains("Older sessions · 1"), "{out}");
    assert!(out.contains("bump the lockfile"), "{out}");
    select(&mut ui, "claude:f00dcafe-job");
    let out = draw(&view, &mut ui, 100, 24);
    assert!(
        out.contains("a live process without a transcript yet"),
        "{out}"
    );
}

#[test]
fn render_80x24_help_overlay() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    ui.show_help = true;
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

#[test]
fn render_empty_workspace() {
    let view = WorkspaceView::new();
    insta::assert_snapshot!(draw(&view, &mut ui(), 80, 24));
}

/// Below 80 columns `s` hides the navigator (content full width) and the hint
/// says how to bring it back.
#[test]
fn render_narrow_terminal_hides_the_navigator() {
    let view = fixture::workspace();
    let mut ui = agent_ui("claude:s-audit-a");
    let out = draw(&view, &mut ui, 70, 24);
    assert!(out.contains("Navigator"), "{out}");
    press(&view, &mut ui, KeyCode::Char('s'));
    assert!(ui.nav_hidden);
    let out = draw(&view, &mut ui, 70, 24);
    assert!(!out.contains("Navigator"), "{out}");
    assert!(out.contains("s nav"), "{out}");
    insta::assert_snapshot!("render_narrow_nav_hidden_70x24", out);
    // Wide again: the navigator is always shown.
    assert!(draw(&view, &mut ui, 100, 24).contains("Navigator"));
}

/// The filter prompt replaces the status bar while typing; matches stand out.
#[test]
fn render_filter_prompt_80x24() {
    let view = fixture::workspace();
    let mut ui = self::ui();
    press(&view, &mut ui, KeyCode::Char('/'));
    for c in "scout".chars() {
        press(&view, &mut ui, KeyCode::Char(c));
    }
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

// ------------------------------------------------------------------ source / console

#[test]
fn shared_workspace_bumps_generation_on_update() {
    use agtrace::watch::WorkspaceSource;
    let src = SharedWorkspace::new(WorkspaceView::new()).with_clock(fixture::now());
    assert_eq!(src.generation(), 0);
    assert_eq!(build(&src, &ui()).nav.rows.len(), 1, "the top node only");
    src.update(|v| {
        for e in fixture::events() {
            v.apply(e, &fixture::resolve, fixture::now());
        }
    });
    assert_eq!(src.generation(), 1);
    assert_eq!(build(&src, &ui()).nav.rows.len(), 3);
}

fn console_output(source: &SharedWorkspace) -> String {
    let mut out = Vec::new();
    agtrace::watch::run_console(source, ui(), &mut out, Some(std::time::Duration::ZERO)).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn console_prints_tree_timelines_and_feed_once() {
    let source = SharedWorkspace::new(fixture::workspace()).with_clock(fixture::now());
    let text = console_output(&source);
    insta::assert_snapshot!("console_initial", text);

    // Encrypted bodies never leak; every agent is announced once.
    assert!(!text.contains("gAAAA"));
    let added = text.lines().filter(|l| l.starts_with("+ ")).count();
    let agents = fixture::workspace().agents.len();
    assert_eq!(added, agents);
}

#[test]
fn console_printer_emits_only_new_rows() {
    use agtrace::presentation::presenters::watch::build_console;
    use agtrace::watch::ConsolePrinter;

    let events = fixture::events();
    let half = events.len() / 2;
    let mut view = WorkspaceView::new();
    let mut printer = ConsolePrinter::default();

    for e in events[..half].iter().cloned() {
        view.apply(e, &fixture::resolve, fixture::now());
    }
    let first = printer.lines(&build_console(&view, &ui(), fixture::now()));
    assert!(!first.is_empty());
    // Nothing changed: nothing printed.
    assert!(
        printer
            .lines(&build_console(&view, &ui(), fixture::now()))
            .is_empty()
    );

    for e in events[half..].iter().cloned() {
        view.apply(e, &fixture::resolve, fixture::now());
    }
    let second = printer.lines(&build_console(&view, &ui(), fixture::now()));
    assert!(!second.is_empty());
    let repeated: Vec<&String> = second
        .iter()
        .filter(|l| !l.starts_with("~ ") && first.contains(l))
        .collect();
    assert!(repeated.is_empty(), "reprinted rows: {repeated:#?}");
}
