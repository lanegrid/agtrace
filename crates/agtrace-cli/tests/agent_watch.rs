//! Multi-agent watch TUI: presenter snapshots, rendered buffers (80x24, 120x40)
//! and keybinding reducer, on a synthetic workspace.

#[path = "support/watch_fixture.rs"]
mod fixture;

use agtrace::agent_watch::{SharedWorkspace, build};
use agtrace::presentation::presenters::agent_watch::build_screen;
use agtrace::presentation::view_models::agent_watch::{
    FeedFilter, Pane, RowKind, Scroll, UiState, Viewport, WatchScreenVm,
};
use agtrace::presentation::views::agent_watch::input::{Effect, action_for, apply, sync_selection};
use agtrace::presentation::views::agent_watch::{layout, render};
use agtrace_sdk::workspace::WorkspaceView;
use chrono::FixedOffset;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

fn ui() -> UiState {
    UiState::new("project demo-project", FixedOffset::east_opt(0).unwrap())
}

fn screen(view: &WorkspaceView, ui: &UiState) -> WatchScreenVm {
    build_screen(view, ui, fixture::now())
}

fn select(ui: &mut UiState, id: &str) {
    ui.selected = Some(id.to_string());
}

fn draw(view: &WorkspaceView, ui: &mut UiState, w: u16, h: u16) -> String {
    ui.viewport = layout(Rect::new(0, 0, w, h)).viewport();
    let vm = screen(view, ui);
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| render(f, &vm)).unwrap();
    t.backend().to_string()
}

// ------------------------------------------------------------------ presenter

#[test]
fn presenter_default_screen() {
    let view = fixture::workspace();
    insta::assert_json_snapshot!(screen(&view, &ui()));
}

#[test]
fn presenter_codex_child_selected_with_feed_filter() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "codex:t-judge");
    ui.feed_filter = FeedFilter::Selected;
    let vm = screen(&view, &ui);
    insta::assert_json_snapshot!(vm);
    assert!(vm.feed.iter().all(|r| r.involves_selected));
}

#[test]
fn presenter_tree_hide_done_and_collapse() {
    let view = fixture::workspace();
    let mut ui = ui();
    ui.hide_done = true;
    ui.collapsed.insert("codex:t-root".to_string());
    let vm = screen(&view, &ui);
    let rows: Vec<String> = vm
        .tree
        .iter()
        .map(|r| {
            format!(
                "{}{} {:?} collapsed={} hidden={}",
                "  ".repeat(r.depth as usize),
                r.label,
                r.status,
                r.collapsed,
                r.hidden_descendants
            )
        })
        .collect();
    insta::assert_snapshot!(rows.join("\n"));
}

#[test]
fn encrypted_bodies_are_never_shown() {
    let view = fixture::workspace();
    for id in [None, Some("codex:t-judge"), Some("codex:t-root")] {
        let mut ui = ui();
        if let Some(id) = id {
            select(&mut ui, id);
        }
        let vm = screen(&view, &ui);
        let enc: Vec<_> = vm.feed.iter().filter(|r| r.encrypted).collect();
        assert!(!enc.is_empty());
        assert!(enc.iter().all(|r| r.text.is_none()));
        for row in &vm.focus.rows {
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
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    let vm = screen(&view, &ui);
    let text: Vec<String> = vm
        .focus
        .rows
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
        assert!(vm.focus.rows.iter().any(|r| r.kind == kind), "{kind:?}");
    }
}

// ------------------------------------------------------------------ rendering

#[test]
fn render_80x24() {
    let view = fixture::workspace();
    insta::assert_snapshot!(draw(&view, &mut ui(), 80, 24));
}

#[test]
fn render_120x40() {
    let view = fixture::workspace();
    insta::assert_snapshot!(draw(&view, &mut ui(), 120, 40));
}

#[test]
fn render_120x40_codex_child_focused_timeline() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "codex:t-judge");
    ui.focus = Pane::Timeline;
    insta::assert_snapshot!(draw(&view, &mut ui, 120, 40));
}

#[test]
fn render_80x24_claude_lead_selected() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

#[test]
fn render_80x24_help_overlay() {
    let view = fixture::workspace();
    let mut ui = ui();
    ui.show_help = true;
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

#[test]
fn render_empty_workspace() {
    let view = WorkspaceView::new();
    insta::assert_snapshot!(draw(&view, &mut ui(), 80, 24));
}

// ------------------------------------------------------------------ input

fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn press(view: &WorkspaceView, ui: &mut UiState, code: KeyCode) -> Effect {
    let vm = screen(view, ui);
    sync_selection(ui, &vm);
    let action = action_for(key(code)).expect("bound key");
    apply(ui, action, &vm)
}

#[test]
fn keys_move_selection_and_collapse() {
    let view = fixture::workspace();
    let mut ui = ui();
    ui.viewport = Viewport {
        tree: 10,
        timeline: 5,
        feed: 4,
    };
    let order: Vec<String> = screen(&view, &ui)
        .tree
        .iter()
        .map(|r| r.id.clone())
        .collect();

    press(&view, &mut ui, KeyCode::Char('j'));
    assert_eq!(ui.selected.as_deref(), Some(order[1].as_str()));
    press(&view, &mut ui, KeyCode::Char('k'));
    press(&view, &mut ui, KeyCode::Char('k'));
    assert_eq!(ui.selected.as_deref(), Some(order[0].as_str()));

    // Collapse the first root: its descendants disappear, then come back.
    let before = screen(&view, &ui).tree.len();
    press(&view, &mut ui, KeyCode::Enter);
    let collapsed = screen(&view, &ui);
    assert!(collapsed.tree.len() < before);
    assert!(collapsed.tree[0].collapsed);
    assert_eq!(
        collapsed.tree[0].hidden_descendants,
        before - collapsed.tree.len()
    );
    press(&view, &mut ui, KeyCode::Char(' '));
    assert_eq!(screen(&view, &ui).tree.len(), before);

    assert_eq!(press(&view, &mut ui, KeyCode::Char('q')), Effect::Quit);
    assert_eq!(press(&view, &mut ui, KeyCode::Char('r')), Effect::Rescan);
}

#[test]
fn scrolling_stops_follow_and_tail_restores_it() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    ui.viewport = Viewport {
        tree: 10,
        timeline: 4,
        feed: 3,
    };
    let rows = screen(&view, &ui).focus.rows.len();
    assert!(rows > 8);

    press(&view, &mut ui, KeyCode::PageUp);
    assert_eq!(ui.timeline_scroll, Scroll::Offset(rows - 4 - 4));
    assert!(!screen(&view, &ui).focus.follow);
    press(&view, &mut ui, KeyCode::PageDown);
    assert_eq!(ui.timeline_scroll, Scroll::Follow);
    press(&view, &mut ui, KeyCode::Char('g'));
    assert_eq!(ui.timeline_scroll, Scroll::Offset(0));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.timeline_scroll, Scroll::Follow);

    // Feed pane: Tab twice, then scroll the feed.
    press(&view, &mut ui, KeyCode::Tab);
    press(&view, &mut ui, KeyCode::Tab);
    assert_eq!(ui.focus, Pane::Feed);
    press(&view, &mut ui, KeyCode::Char('k'));
    assert!(matches!(ui.feed_scroll, Scroll::Offset(_)));
    assert_eq!(ui.timeline_scroll, Scroll::Follow);
}

#[test]
fn toggles_and_help() {
    let view = fixture::workspace();
    let mut ui = ui();
    press(&view, &mut ui, KeyCode::Char('f'));
    assert_eq!(ui.feed_filter, FeedFilter::Selected);
    press(&view, &mut ui, KeyCode::Char('h'));
    assert!(ui.hide_done);
    press(&view, &mut ui, KeyCode::Char('a'));
    assert!(ui.auto_select);
    // Auto-select picks the most recently active agent (lead's Bash at 12:04:50).
    let vm = screen(&view, &ui);
    assert_eq!(vm.focus.agent_id.as_deref(), Some("claude:s-lead"));

    press(&view, &mut ui, KeyCode::Char('?'));
    assert!(ui.show_help);
    // While help is open other keys are swallowed.
    press(&view, &mut ui, KeyCode::Char('h'));
    assert!(ui.hide_done);
    press(&view, &mut ui, KeyCode::Esc);
    assert!(!ui.show_help);
}

#[test]
fn shared_workspace_bumps_generation_on_update() {
    use agtrace::agent_watch::WorkspaceSource;
    let src = SharedWorkspace::new(WorkspaceView::new()).with_clock(fixture::now());
    assert_eq!(src.generation(), 0);
    assert!(build(&src, &ui()).tree.is_empty());
    src.update(|v| {
        for e in fixture::events() {
            v.apply(e, &fixture::resolve, fixture::now());
        }
    });
    assert_eq!(src.generation(), 1);
    assert_eq!(build(&src, &ui()).tree.len(), 9);
}
