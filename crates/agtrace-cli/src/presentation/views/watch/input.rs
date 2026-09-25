//! Keybindings (design §6.2) and the UI-state reducer.
//!
//! [`action_for`] maps a key to an [`Action`]; [`apply`] updates the [`UiState`]
//! against the last rendered screen (row order, row counts) and reports effects
//! that leave the UI (quit, rescan).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::view_models::watch::{FeedFilter, Pane, Scroll, UiState, WatchScreenVm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    /// Focus the selected agent's timeline.
    Open,
    ToggleCollapse,
    Collapse,
    Expand,
    NextPane,
    PrevPane,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Tail,
    Top,
    ToggleFeedFilter,
    ToggleHideDone,
    ToggleAutoSelect,
    Rescan,
    ToggleHelp,
    /// Close help, or reset every view toggle back to the default screen.
    Back,
    Quit,
}

/// Effect of an action outside the UI state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    Quit,
    Rescan,
}

pub fn action_for(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('u') if ctrl => Action::HalfPageUp,
        KeyCode::Char('d') if ctrl => Action::HalfPageDown,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Up => Action::Up,
        KeyCode::Enter => Action::Open,
        KeyCode::Char(' ') => Action::ToggleCollapse,
        KeyCode::Left => Action::Collapse,
        KeyCode::Right => Action::Expand,
        KeyCode::Tab => Action::NextPane,
        KeyCode::BackTab => Action::PrevPane,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Char('G') | KeyCode::End => Action::Tail,
        KeyCode::Char('g') | KeyCode::Home => Action::Top,
        KeyCode::Char('f') => Action::ToggleFeedFilter,
        KeyCode::Char('h') => Action::ToggleHideDone,
        KeyCode::Char('a') => Action::ToggleAutoSelect,
        KeyCode::Char('r') => Action::Rescan,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Esc => Action::Back,
        _ => return None,
    })
}

/// Scrollable pane addressed by scroll keys: the tree has no scroll of its own,
/// so its scroll keys move the timeline.
fn scroll_target(ui: &UiState) -> Pane {
    match ui.focus {
        Pane::Feed => Pane::Feed,
        _ => Pane::Timeline,
    }
}

fn scroll(ui: &mut UiState, vm: &WatchScreenVm, pane: Pane, up: bool, n: usize) {
    let (state, total, height) = match pane {
        Pane::Feed => (&mut ui.feed_scroll, vm.feed.len(), ui.viewport.feed),
        _ => (
            &mut ui.timeline_scroll,
            vm.focus.rows.len(),
            ui.viewport.timeline,
        ),
    };
    let max = total.saturating_sub(height);
    let cur = state.start(total, height);
    *state = if up {
        Scroll::Offset(cur.saturating_sub(n))
    } else if cur + n >= max {
        Scroll::Follow
    } else {
        Scroll::Offset(cur + n)
    };
}

fn set_scroll(ui: &mut UiState, pane: Pane, s: Scroll) {
    match pane {
        Pane::Feed => ui.feed_scroll = s,
        _ => ui.timeline_scroll = s,
    }
}

fn select(ui: &mut UiState, vm: &WatchScreenVm, idx: usize) {
    if let Some(row) = vm.tree.get(idx) {
        if ui.selected.as_deref() != Some(row.id.as_str()) {
            ui.timeline_scroll = Scroll::Follow;
        }
        ui.selected = Some(row.id.clone());
        ui.auto_select = false;
    }
}

/// The selected row, if collapsing it can make sense: it has children and it is
/// not the only root (folding the sole root would hide the whole tree and make a
/// single-session watch look like an empty one).
fn collapsible_selected(
    vm: &WatchScreenVm,
) -> Option<&crate::presentation::view_models::watch::AgentRowVm> {
    let row = vm.selected_index().and_then(|i| vm.tree.get(i))?;
    if !row.has_children {
        return None;
    }
    let roots = vm.tree.iter().filter(|r| r.depth == 0).count();
    (row.depth > 0 || roots > 1).then_some(row)
}

/// Esc: back to the default screen (selection and auto-select are kept).
fn reset_view(ui: &mut UiState) {
    ui.collapsed.clear();
    ui.hide_done = false;
    ui.feed_filter = FeedFilter::All;
    ui.focus = Pane::Tree;
    ui.timeline_scroll = Scroll::Follow;
    ui.feed_scroll = Scroll::Follow;
}

/// Apply `action` to `ui`; `vm` is the screen currently displayed.
pub fn apply(ui: &mut UiState, action: Action, vm: &WatchScreenVm) -> Effect {
    if ui.show_help {
        match action {
            Action::Quit => return Effect::Quit,
            Action::ToggleHelp | Action::Back => ui.show_help = false,
            _ => {}
        }
        return Effect::None;
    }
    let page = |h: usize| h.max(1);
    let target = scroll_target(ui);
    let height = match target {
        Pane::Feed => ui.viewport.feed,
        _ => ui.viewport.timeline,
    };
    match action {
        Action::Quit => return Effect::Quit,
        Action::Rescan => return Effect::Rescan,
        Action::Up | Action::Down if ui.focus == Pane::Tree => {
            let cur = vm.selected_index();
            let next = match (cur, action) {
                (None, _) => 0,
                (Some(i), Action::Up) => i.saturating_sub(1),
                (Some(i), _) => (i + 1).min(vm.tree.len().saturating_sub(1)),
            };
            select(ui, vm, next);
        }
        Action::Up => scroll(ui, vm, target, true, 1),
        Action::Down => scroll(ui, vm, target, false, 1),
        Action::PageUp => scroll(ui, vm, target, true, page(height)),
        Action::PageDown => scroll(ui, vm, target, false, page(height)),
        Action::HalfPageUp => scroll(ui, vm, target, true, page(height / 2)),
        Action::HalfPageDown => scroll(ui, vm, target, false, page(height / 2)),
        Action::Tail => set_scroll(ui, target, Scroll::Follow),
        Action::Top => set_scroll(ui, target, Scroll::Offset(0)),
        Action::Open => ui.focus = Pane::Timeline,
        Action::ToggleCollapse | Action::Collapse | Action::Expand => {
            if let Some(row) = collapsible_selected(vm) {
                let collapsed = ui.collapsed.contains(&row.id);
                let want = match action {
                    Action::Collapse => true,
                    Action::Expand => false,
                    _ => !collapsed,
                };
                if want {
                    ui.collapsed.insert(row.id.clone());
                } else {
                    ui.collapsed.remove(&row.id);
                }
            }
        }
        Action::NextPane => ui.focus = ui.focus.next(),
        Action::PrevPane => ui.focus = ui.focus.prev(),
        Action::ToggleFeedFilter => {
            ui.feed_filter = match ui.feed_filter {
                FeedFilter::All => FeedFilter::Selected,
                FeedFilter::Selected => FeedFilter::All,
            };
            ui.feed_scroll = Scroll::Follow;
        }
        Action::ToggleHideDone => ui.hide_done = !ui.hide_done,
        Action::ToggleAutoSelect => ui.auto_select = !ui.auto_select,
        Action::ToggleHelp => ui.show_help = true,
        Action::Back => reset_view(ui),
    }
    Effect::None
}

/// Remember the selection the presenter resolved (first row, auto-select, or the
/// nearest shown ancestor), so relative moves start from what is displayed.
pub fn sync_selection(ui: &mut UiState, vm: &WatchScreenVm) {
    if let Some(id) = &vm.focus.agent_id
        && ui.selected.as_ref() != Some(id)
    {
        if ui.selected.is_some() {
            ui.timeline_scroll = Scroll::Follow;
        }
        ui.selected = Some(id.clone());
    }
}
