//! Multi-agent watch TUI: presenter snapshots, rendered buffers (80x24, 120x40)
//! and keybinding reducer, on a synthetic workspace.

#[path = "support/watch_fixture.rs"]
mod fixture;

use agtrace::presentation::presenters::watch::build_screen;
use std::time::{Duration, Instant};

use agtrace::presentation::view_models::watch::{
    FeedFilter, Pane, RowKind, Screen, Scroll, TOAST_TTL, Toast, UiState, Viewport, WatchScreenVm,
};
use agtrace::presentation::views::watch::input::{
    Effect, action_for, apply, expire_toast, sync_selection,
};
use agtrace::presentation::views::watch::{layout, render};
use agtrace::watch::{SharedWorkspace, build};
use agtrace_sdk::workspace::WorkspaceView;
use chrono::FixedOffset;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

/// UI state on the agents screen (`2`), which most tests below exercise.
fn ui() -> UiState {
    UiState {
        screen: Screen::Agents,
        ..home()
    }
}

/// UI state as `watch` starts it: the overview screen.
fn home() -> UiState {
    UiState::new("project demo-project", FixedOffset::east_opt(0).unwrap())
}

fn screen(view: &WorkspaceView, ui: &UiState) -> WatchScreenVm {
    build_screen(view, ui, fixture::now())
}

fn select(ui: &mut UiState, id: &str) {
    ui.selected = Some(id.to_string());
}

fn draw_buffer(view: &WorkspaceView, ui: &mut UiState, w: u16, h: u16) -> Buffer {
    ui.viewport = layout(Rect::new(0, 0, w, h)).viewport();
    let vm = screen(view, ui);
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| render(f, &vm)).unwrap();
    t.backend().buffer().clone()
}

fn draw(view: &WorkspaceView, ui: &mut UiState, w: u16, h: u16) -> String {
    ui.viewport = layout(Rect::new(0, 0, w, h)).viewport();
    let vm = screen(view, ui);
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| render(f, &vm)).unwrap();
    t.backend().to_string()
}

/// Text of buffer row `y`.
fn line(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
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
fn render_80x24_timeline_focused() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    ui.focus = Pane::Timeline;
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

#[test]
fn render_120x40_timeline_focused() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    ui.focus = Pane::Timeline;
    insta::assert_snapshot!(draw(&view, &mut ui, 120, 40));
}

#[test]
fn render_80x24_feed_focused() {
    let view = fixture::workspace();
    let mut ui = ui();
    ui.focus = Pane::Feed;
    insta::assert_snapshot!(draw(&view, &mut ui, 80, 24));
}

/// Collapse `codex:t-root` with space: the tree folds and the status bar shows
/// the toast plus the `Esc:reset` hint.
#[test]
fn render_toast_after_collapse() {
    let view = fixture::workspace();
    for (w, h) in [(80, 24), (120, 40)] {
        let mut ui = ui();
        select(&mut ui, "codex:t-root");
        press(&view, &mut ui, KeyCode::Char(' '));
        insta::assert_snapshot!(format!("render_{w}x{h}_toast"), draw(&view, &mut ui, w, h));
    }
}

/// With the timeline focused, feed rows involving its agent are highlighted
/// (cyan bold route) and the others dimmed, not filtered out.
#[test]
fn timeline_focus_highlights_agent_feed_rows() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    ui.focus = Pane::Timeline;
    let vm = screen(&view, &ui);
    assert!(vm.feed.iter().any(|r| !r.involves_selected), "not filtered");
    let buf = draw_buffer(&view, &mut ui, 120, 40);
    let feed = layout(Rect::new(0, 0, 120, 40)).feed;
    let height = ui.viewport.feed;
    let start = vm.feed_scroll.start(vm.feed.len(), height);
    let (mut hot, mut cold) = (0, 0);
    for (i, r) in vm.feed.iter().skip(start).take(height).enumerate() {
        let y = feed.y + 1 + i as u16;
        let text = line(&buf, y);
        assert!(text.contains(&r.from), "{text} / {r:?}");
        // Route column starts after `│hh:mm `.
        let cell = &buf[(feed.x + 7, y)];
        if r.involves_selected {
            assert_eq!(cell.fg, Color::Cyan, "{text}");
            assert!(cell.modifier.contains(Modifier::BOLD), "{text}");
            hot += 1;
        } else {
            assert!(cell.modifier.contains(Modifier::DIM), "{text}");
            cold += 1;
        }
    }
    assert!(hot > 0 && cold > 0, "hot={hot} cold={cold}");
}

#[test]
fn render_empty_workspace() {
    let view = WorkspaceView::new();
    insta::assert_snapshot!(draw(&view, &mut ui(), 80, 24));
}

// ------------------------------------------------------------------ overview / detail

/// Overview at the fixture clock with the 15m window (the scenario spans 5 minutes).
fn overview_ui() -> UiState {
    use agtrace::presentation::view_models::watch::LaneWindow;
    UiState {
        window: LaneWindow::M15,
        ..home()
    }
}

#[test]
fn render_overview_80x24() {
    let view = fixture::workspace();
    insta::assert_snapshot!(draw(&view, &mut overview_ui(), 80, 24));
}

#[test]
fn render_overview_120x40() {
    let view = fixture::workspace();
    let mut ui = overview_ui();
    select(&mut ui, "codex:t-judge");
    insta::assert_snapshot!(draw(&view, &mut ui, 120, 40));
}

/// Overview rows: two roots with headers, lanes with compaction markers, and the
/// "now" cell for running / idle / done agents.
#[test]
fn presenter_overview_rows() {
    let view = fixture::workspace();
    let mut ui = overview_ui();
    ui.viewport = layout(Rect::new(0, 0, 120, 40)).viewport();
    let vm = screen(&view, &ui);
    let ov = vm.overview.expect("overview");
    assert_eq!(ov.cell, "1m");
    let roots: Vec<&str> = ov
        .rows
        .iter()
        .filter_map(|r| r.root.as_ref().map(|h| h.label.as_str()))
        .collect();
    assert_eq!(roots, vec!["/root", "s-lead"]);
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
    use agtrace::presentation::view_models::watch::LaneWindow;
    let view = fixture::workspace();
    let mut ui = overview_ui();
    ui.viewport = layout(Rect::new(0, 0, 120, 40)).viewport();
    let cols = ui.viewport.lane_cols;
    assert!(cols >= 30, "{cols}");
    for (w, cell) in [
        (LaneWindow::M15, "1m"),
        (LaneWindow::M60, "2m"),
        (LaneWindow::H4, "6m"),
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
    use agtrace::presentation::view_models::watch::{LaneWindow, StatusVm};
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
        ..home()
    };
    ui.viewport = layout(Rect::new(0, 0, 120, 40)).viewport();
    let ov = build_screen(&view, &ui, now).overview.unwrap();
    let row = &ov.rows[0];
    assert_eq!(row.status, StatusVm::Done);
    assert!(!row.lane.contains('·'), "lane: {:?}", row.lane);
    assert!(row.lane.trim().starts_with('▁'), "lane: {:?}", row.lane);
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
    let vm = screen(&view, &detail_ui(sub_id.as_str()));
    match vm.detail.unwrap().result {
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

fn detail_ui(id: &str) -> UiState {
    let mut ui = ui();
    select(&mut ui, id);
    ui.screen = Screen::Detail;
    ui.detail_agent = Some(id.to_string());
    ui
}

/// Claude teammate: its task arrived as a NEW_TASK from the lead.
#[test]
fn render_detail_claude_teammate_120x40() {
    let view = fixture::workspace();
    let mut ui = detail_ui("claude:s-audit-a");
    let vm = screen(&view, &ui);
    let d = vm.detail.as_ref().unwrap();
    assert_eq!(d.relation, "teammate of s-lead (team audit)");
    assert_eq!(d.instructions[0].text.as_deref(), Some("review parser"));
    insta::assert_snapshot!(draw(&view, &mut ui, 120, 40));
}

/// Claude subagent: prompt from its own log, result from the lead's task notification.
#[test]
fn render_detail_claude_subagent_with_result_120x40() {
    let view = fixture::workspace();
    let mut ui = detail_ui("claude:s-lead/a7ac2e91");
    insta::assert_snapshot!(draw(&view, &mut ui, 120, 40));
}

/// Codex child: encrypted task (sender, path, no body) and a plaintext FINAL_ANSWER.
#[test]
fn presenter_detail_codex_child_encrypted_task_and_final_answer() {
    let view = fixture::workspace();
    let mut ui = detail_ui("codex:t-scout");
    let vm = screen(&view, &ui);
    let d = vm.detail.clone().unwrap();
    let task = &d.instructions[0];
    assert!(task.encrypted && task.text.is_none(), "{task:?}");
    assert!(
        task.note
            .as_deref()
            .is_some_and(|n| n.starts_with("[encrypted by Codex] NEW_TASK from /root")),
        "{task:?}"
    );
    insta::assert_json_snapshot!(d);
    insta::assert_snapshot!(
        "render_detail_codex_child_80x24",
        draw(&view, &mut ui, 80, 24)
    );
}

/// Claude lead: effort in the header; the team's task list in "Now", with the task
/// a teammate works on attributed to it.
#[test]
fn render_detail_claude_lead_plan_100x30() {
    use agtrace::presentation::view_models::watch::{DetailSection, TaskStatusVm};
    let view = fixture::workspace();
    let mut ui = detail_ui("claude:s-lead");
    ui.detail_section = DetailSection::Now;
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
    insta::assert_snapshot!(draw(&view, &mut ui, 100, 30));
}

/// Codex root: goal and plan-mode plan text in "Now".
#[test]
fn render_detail_codex_root_goal_and_plan_100x30() {
    use agtrace::presentation::view_models::watch::DetailSection;
    let view = fixture::workspace();
    let mut ui = detail_ui("codex:t-root");
    ui.detail_section = DetailSection::Now;
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
    insta::assert_snapshot!(draw(&view, &mut ui, 100, 30));
}

/// Between tools, the overview's "now" shows the task in progress (its active
/// form) instead of the last text; an open tool still wins.
#[test]
fn overview_now_prefers_the_task_in_progress_between_tools() {
    use agtrace::presentation::view_models::watch::NowVm;
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
        let vm = screen(view, &overview_ui());
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

// ------------------------------------------------------------------ input

fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn press(view: &WorkspaceView, ui: &mut UiState, code: KeyCode) -> Effect {
    let vm = screen(view, ui);
    sync_selection(ui, &vm);
    let action = action_for(key(code)).expect("bound key");
    apply(ui, action, &vm, Instant::now())
}

fn toast(ui: &UiState) -> &str {
    ui.toast.as_ref().map(|t| t.text.as_str()).unwrap_or("")
}

#[test]
fn keys_move_selection_and_collapse() {
    let view = fixture::workspace();
    let mut ui = ui();
    ui.viewport = Viewport {
        tree: 10,
        timeline: 5,
        feed: 4,
        ..Default::default()
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
    press(&view, &mut ui, KeyCode::Char(' '));
    let collapsed = screen(&view, &ui);
    assert!(collapsed.tree.len() < before);
    assert!(collapsed.tree[0].collapsed);
    let hidden = collapsed.tree[0].hidden_descendants;
    assert_eq!(hidden, before - collapsed.tree.len());
    assert_eq!(toast(&ui), format!("▸ collapsed /root (+{hidden} hidden)"));
    press(&view, &mut ui, KeyCode::Char(' '));
    assert_eq!(screen(&view, &ui).tree.len(), before);
    assert_eq!(toast(&ui), "▾ expanded /root");

    assert_eq!(press(&view, &mut ui, KeyCode::Char('q')), Effect::Quit);
    assert_eq!(press(&view, &mut ui, KeyCode::Char('r')), Effect::Rescan);
    assert_eq!(toast(&ui), "rescanning…");
}

/// Regression: keys pressed between two frames are applied against the same
/// (last drawn) screen; `j j j` must still move three rows, not one.
#[test]
fn moves_between_frames_accumulate() {
    let view = fixture::workspace();
    let mut ui = ui();
    let vm = screen(&view, &ui);
    sync_selection(&mut ui, &vm);
    let now = Instant::now();
    let hit = |ui: &mut UiState, code| apply(ui, action_for(key(code)).unwrap(), &vm, now);
    for _ in 0..3 {
        hit(&mut ui, KeyCode::Char('j'));
    }
    assert_eq!(ui.selected.as_deref(), Some(vm.tree[3].id.as_str()));
    // Space acts on the row the selection moved to (`x`, a leaf), not the stale highlight.
    hit(&mut ui, KeyCode::Char('k'));
    hit(&mut ui, KeyCode::Char(' '));
    assert_eq!(toast(&ui), format!("{} has no children", vm.tree[2].label));

    // Fold and unfold the root within one frame.
    let mut ui = self::ui();
    sync_selection(&mut ui, &vm);
    hit(&mut ui, KeyCode::Char(' '));
    assert!(ui.collapsed.contains(&vm.tree[0].id));
    hit(&mut ui, KeyCode::Char(' '));
    assert!(ui.collapsed.is_empty());
    assert!(toast(&ui).starts_with("▾ expanded"));
}

/// Enter / → / l open the selected agent's detail (from the agents tree, its
/// timeline or feed, and the overview); they never collapse. Esc returns to the
/// screen the detail was opened from, with the selection kept.
#[test]
fn open_keys_open_the_detail_and_esc_returns() {
    let view = fixture::workspace();
    let before = screen(&view, &ui()).tree.len();
    for code in [KeyCode::Enter, KeyCode::Right, KeyCode::Char('l')] {
        for (start, pane) in [
            (Screen::Agents, Pane::Tree),
            (Screen::Agents, Pane::Timeline),
            (Screen::Agents, Pane::Feed),
            (Screen::Overview, Pane::Tree),
        ] {
            let mut ui = ui();
            ui.screen = start;
            ui.focus = pane;
            select(&mut ui, "claude:s-audit-a");
            press(&view, &mut ui, code);
            assert_eq!(ui.screen, Screen::Detail, "{code:?} {start:?} {pane:?}");
            assert_eq!(ui.detail_agent.as_deref(), Some("claude:s-audit-a"));
            assert!(ui.collapsed.is_empty(), "{code:?}");
            let vm = screen(&view, &ui);
            assert_eq!(vm.detail.as_ref().unwrap().title, "audit-A");
            // Already in the detail: stays there.
            press(&view, &mut ui, code);
            assert_eq!(ui.screen, Screen::Detail, "{code:?}");
            press(&view, &mut ui, KeyCode::Esc);
            assert_eq!(ui.screen, start, "{code:?}: back to where it was opened");
            assert_eq!(ui.focus, pane, "{code:?}: pane focus kept");
            assert_eq!(ui.selected.as_deref(), Some("claude:s-audit-a"));
            assert_eq!(screen(&view, &ui).tree.len(), before, "{code:?}");
        }
    }
}

/// `1` / `2` switch screens, also from the detail; the default is the overview.
#[test]
fn number_keys_switch_screens() {
    let view = fixture::workspace();
    let mut ui = home();
    assert_eq!(ui.screen, Screen::Overview);
    let vm = screen(&view, &ui);
    assert!(vm.overview.is_some() && vm.detail.is_none());
    press(&view, &mut ui, KeyCode::Char('2'));
    assert_eq!(ui.screen, Screen::Agents);
    assert!(screen(&view, &ui).overview.is_none());
    press(&view, &mut ui, KeyCode::Enter);
    assert_eq!(ui.screen, Screen::Detail);
    press(&view, &mut ui, KeyCode::Char('1'));
    assert_eq!(ui.screen, Screen::Overview);
    // j/k move the selection on the overview.
    let order: Vec<String> = screen(&view, &ui)
        .tree
        .iter()
        .map(|r| r.id.clone())
        .collect();
    select(&mut ui, &order[0]);
    press(&view, &mut ui, KeyCode::Char('j'));
    assert_eq!(ui.selected.as_deref(), Some(order[1].as_str()));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.selected.as_deref(), order.last().map(String::as_str));
    // Esc on the overview resets the view toggles.
    ui.hide_done = true;
    press(&view, &mut ui, KeyCode::Esc);
    assert!(!ui.hide_done);
    assert_eq!(toast(&ui), "view reset");
}

/// `+` / `-` (and `]` / `[`) step the overview activity window and say so.
#[test]
fn window_keys_zoom_the_activity_lanes() {
    use agtrace::presentation::view_models::watch::LaneWindow;
    let view = fixture::workspace();
    let mut ui = home();
    assert_eq!(ui.window, LaneWindow::M60);
    press(&view, &mut ui, KeyCode::Char('+'));
    assert_eq!(ui.window, LaneWindow::H4);
    assert_eq!(toast(&ui), "activity window: last 4h");
    press(&view, &mut ui, KeyCode::Char(']'));
    assert_eq!(ui.window, LaneWindow::All);
    assert_eq!(
        toast(&ui),
        "activity window: all (since the oldest agent started)"
    );
    press(&view, &mut ui, KeyCode::Char('+'));
    assert_eq!(
        toast(&ui),
        "activity window: all (since the oldest agent started) — widest"
    );
    for _ in 0..3 {
        press(&view, &mut ui, KeyCode::Char('-'));
    }
    assert_eq!(ui.window, LaneWindow::M15);
    assert_eq!(toast(&ui), "activity window: last 15m");
    press(&view, &mut ui, KeyCode::Char('['));
    assert_eq!(toast(&ui), "activity window: last 15m — narrowest");
}

/// Detail keys: Tab cycles sections, j/k/G/g scroll the focused one (clamped to
/// the last frame's wrapped text), panes and toggles of other screens do not act.
#[test]
fn detail_sections_cycle_and_scroll() {
    use agtrace::presentation::view_models::watch::{DetailSection, SectionMetrics};
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    press(&view, &mut ui, KeyCode::Enter);
    assert_eq!(ui.detail_section, DetailSection::Instructions);
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
    press(&view, &mut ui, KeyCode::Char('j'));
    press(&view, &mut ui, KeyCode::Char('j'));
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(6), "clamped at the end");
    press(&view, &mut ui, KeyCode::Char('g'));
    assert_eq!(ui.detail_scroll[0], Scroll::Offset(0));

    press(&view, &mut ui, KeyCode::BackTab);
    assert_eq!(ui.detail_section, DetailSection::Timeline);
    assert_eq!(ui.detail_scroll[3], Scroll::Follow);
    press(&view, &mut ui, KeyCode::Char('k'));
    assert_eq!(ui.detail_scroll[3], Scroll::Offset(14));
    press(&view, &mut ui, KeyCode::Char('G'));
    assert_eq!(ui.detail_scroll[3], Scroll::Follow);
    press(&view, &mut ui, KeyCode::Tab);
    assert_eq!(ui.detail_section, DetailSection::Instructions);

    // Toggles of the tree screens are ignored here.
    press(&view, &mut ui, KeyCode::Char(' '));
    press(&view, &mut ui, KeyCode::Char('d'));
    assert!(ui.collapsed.is_empty() && !ui.hide_done);
    assert_eq!(ui.screen, Screen::Detail);
}

/// Esc / ← / h go back one level: help → closed, timeline / feed → tree, tree →
/// view reset (selection kept).
#[test]
fn back_keys_step_out_one_level() {
    let view = fixture::workspace();
    for code in [KeyCode::Esc, KeyCode::Left, KeyCode::Char('h')] {
        let mut ui = ui();
        ui.viewport = Viewport {
            tree: 10,
            timeline: 5,
            feed: 4,
            ..Default::default()
        };
        select(&mut ui, "codex:t-judge");
        ui.collapsed.insert("claude:s-lead".to_string());
        ui.hide_done = true;
        ui.feed_filter = FeedFilter::Selected;
        ui.feed_scroll = Scroll::Offset(0);
        ui.timeline_scroll = Scroll::Offset(0);
        ui.focus = Pane::Timeline;
        ui.show_help = true;

        press(&view, &mut ui, code);
        assert!(!ui.show_help, "{code:?}: help closes first");
        assert_eq!(ui.focus, Pane::Timeline, "{code:?}");

        press(&view, &mut ui, code);
        assert_eq!(ui.focus, Pane::Tree, "{code:?}: timeline → tree");
        assert!(ui.hide_done, "{code:?}: toggles survive the first step");
        assert!(ui.toast.is_none(), "{code:?}");

        press(&view, &mut ui, code);
        assert!(ui.collapsed.is_empty(), "{code:?}");
        assert!(!ui.hide_done, "{code:?}");
        assert_eq!(ui.feed_filter, FeedFilter::All, "{code:?}");
        assert_eq!(ui.timeline_scroll, Scroll::Follow, "{code:?}");
        assert_eq!(ui.feed_scroll, Scroll::Follow, "{code:?}");
        assert_eq!(ui.selected.as_deref(), Some("codex:t-judge"), "{code:?}");
        assert_eq!(toast(&ui), "view reset", "{code:?}");

        // Feed → tree as well.
        ui.focus = Pane::Feed;
        press(&view, &mut ui, code);
        assert_eq!(ui.focus, Pane::Tree, "{code:?}: feed → tree");
    }
}

/// ← / → no longer fold: space is the only collapse key.
#[test]
fn arrows_do_not_collapse() {
    let view = fixture::workspace();
    let mut ui = ui();
    press(&view, &mut ui, KeyCode::Left);
    press(&view, &mut ui, KeyCode::Right);
    press(&view, &mut ui, KeyCode::Esc);
    press(&view, &mut ui, KeyCode::Left);
    assert!(ui.collapsed.is_empty());
}

/// Regression: with a single root (e.g. `watch --session`), folding that root
/// hid the whole tree and made the watch look like a one-row session list.
#[test]
fn sole_root_cannot_be_collapsed() {
    use agtrace_sdk::workspace::WorkspaceEvent;
    // Only the Claude tree: one root with teammates, a subagent and a fork.
    let mut view = WorkspaceView::new();
    for e in fixture::events() {
        let keep = match &e {
            WorkspaceEvent::AgentDiscovered(a) => a.id.as_str().starts_with("claude:"),
            WorkspaceEvent::Events { agent, .. } => agent.as_str().starts_with("claude:"),
            _ => true,
        };
        if keep {
            view.apply(e, &fixture::resolve, fixture::now());
        }
    }
    let mut ui = ui();
    let vm = screen(&view, &ui);
    assert_eq!(vm.tree.iter().filter(|r| r.depth == 0).count(), 1);
    let before = vm.tree.len();
    assert!(before > 1);

    press(&view, &mut ui, KeyCode::Char(' '));
    assert!(ui.collapsed.is_empty());
    assert_eq!(screen(&view, &ui).tree.len(), before);
    assert_eq!(toast(&ui), "can't collapse the only session");
}

#[test]
fn collapsing_a_leaf_explains_why_nothing_happened() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "codex:t-scout");
    press(&view, &mut ui, KeyCode::Char(' '));
    assert!(ui.collapsed.is_empty());
    assert_eq!(toast(&ui), "scout has no children");
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
        ..Default::default()
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

    // Timeline focused: j/k scroll it instead of moving the selection.
    press(&view, &mut ui, KeyCode::Tab);
    press(&view, &mut ui, KeyCode::Char('k'));
    assert_eq!(ui.selected.as_deref(), Some("claude:s-lead"));
    assert!(matches!(ui.timeline_scroll, Scroll::Offset(_)));
    press(&view, &mut ui, KeyCode::Char('G'));

    // Feed pane: Tab, then scroll the feed.
    press(&view, &mut ui, KeyCode::Tab);
    assert_eq!(ui.focus, Pane::Feed);
    press(&view, &mut ui, KeyCode::Char('k'));
    assert!(matches!(ui.feed_scroll, Scroll::Offset(_)));
    assert_eq!(ui.timeline_scroll, Scroll::Follow);
}

#[test]
fn toggles_raise_toasts_and_help_swallows_keys() {
    let view = fixture::workspace();
    let mut ui = ui();
    select(&mut ui, "claude:s-lead");
    press(&view, &mut ui, KeyCode::Char('f'));
    assert_eq!(ui.feed_filter, FeedFilter::Selected);
    assert_eq!(toast(&ui), "messages: s-lead only — f for all");
    press(&view, &mut ui, KeyCode::Char('f'));
    assert_eq!(toast(&ui), "messages: all");

    let hideable = screen(&view, &ui).status.done_hideable;
    assert!(hideable > 0);
    press(&view, &mut ui, KeyCode::Char('d'));
    assert!(ui.hide_done);
    assert_eq!(
        toast(&ui),
        format!("done agents hidden ({hideable}) — d to show")
    );
    assert_eq!(screen(&view, &ui).status.hidden, hideable);
    press(&view, &mut ui, KeyCode::Char('d'));
    assert!(!ui.hide_done);
    assert_eq!(toast(&ui), "done agents shown");
    press(&view, &mut ui, KeyCode::Char('d'));

    press(&view, &mut ui, KeyCode::Char('a'));
    assert!(ui.auto_select);
    assert_eq!(toast(&ui), "auto-select on");
    // Auto-select picks the most recently active agent (lead's Bash at 12:04:50).
    let vm = screen(&view, &ui);
    assert_eq!(vm.focus.agent_id.as_deref(), Some("claude:s-lead"));
    press(&view, &mut ui, KeyCode::Char('a'));
    assert_eq!(toast(&ui), "auto-select off");

    press(&view, &mut ui, KeyCode::Char('?'));
    assert!(ui.show_help);
    // While help is open other keys are swallowed.
    press(&view, &mut ui, KeyCode::Char('d'));
    assert!(ui.hide_done);
    press(&view, &mut ui, KeyCode::Char(' '));
    assert!(ui.collapsed.is_empty());
    press(&view, &mut ui, KeyCode::Esc);
    assert!(!ui.show_help);
    assert!(ui.hide_done, "Esc only closed help");
}

#[test]
fn toast_expires() {
    let view = fixture::workspace();
    let mut ui = ui();
    let now = Instant::now();
    ui.toast = Some(Toast::new("view reset", now));
    assert_eq!(screen(&view, &ui).toast.as_deref(), Some("view reset"));
    assert!(!expire_toast(&mut ui, now + Duration::from_millis(1500)));
    assert!(ui.toast.is_some());

    // Raised long enough ago: the presenter hides it, the loop drops it.
    let old = now
        .checked_sub(TOAST_TTL + Duration::from_millis(1))
        .expect("monotonic clock past the TTL");
    ui.toast = Some(Toast::new("view reset", old));
    assert!(screen(&view, &ui).toast.is_none());
    assert!(expire_toast(&mut ui, now));
    assert!(ui.toast.is_none());
    assert!(!expire_toast(&mut ui, now));
}

#[test]
fn shared_workspace_bumps_generation_on_update() {
    use agtrace::watch::WorkspaceSource;
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

// ------------------------------------------------------------------ console mode

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
    let vm = screen(&view, &ui());
    let rows: Vec<(u32, &str)> = vm
        .tree
        .iter()
        .map(|r| (r.depth as u32, r.label.as_str()))
        .collect();
    assert_eq!(
        rows,
        vec![(0, "Same title"), (1, "Same title (earlier transcript)")]
    );
}
