//! Keybindings (design §6.2) and the UI-state reducer.
//!
//! The navigator is always there; keys act on the focused pane (`Tab` cycles
//! navigator → content → messages):
//!
//! - **navigator**: ↑/↓ move the selection (the content follows at once); → expands
//!   a node, then goes to its first child, and on a leaf agent focuses the content;
//!   ← collapses, else goes to the parent; Enter focuses the content; Esc clears the
//!   filter, else goes to the top node.
//! - **content** / **messages**: ↑/↓, PgUp/PgDn, C-u/C-d, g/G scroll; ← or Esc go
//!   back to the navigator.
//! - anywhere: `i` `n` `r` `t` show that section of the selected agent's detail (a
//!   session's root agent; a group's first item) and focus the content; `/` filters
//!   the navigator; `d` shows / folds finished agents; `s` hides the navigator on
//!   narrow terminals.
//!
//! [`action_in`] maps a key to an [`Action`] (the filter prompt takes typed text);
//! [`apply`] updates the [`UiState`] against the last rendered screen (row order),
//! raises a toast for every state change, and reports effects that leave the UI
//! (quit, rescan).

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::view_models::watch::{
    ContentVm, DetailSection, LaneWindow, NAV_TOP, NavKind, NavRowVm, Pane, Scroll, Toast, UiState,
    WatchScreenVm, initial_detail_scroll,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    /// → / l: expand, go to the first child, or read a leaf.
    Right,
    /// ← / h: collapse or go to the parent; back to the navigator.
    Left,
    /// Enter: focus the content.
    Enter,
    /// Esc: close help, back to the navigator, clear the filter, go to the top.
    Back,
    /// Show a detail section of the selected agent and focus the content.
    Section(DetailSection),
    /// `/`: start typing the name filter.
    StartFilter,
    /// While typing the filter.
    FilterChar(char),
    FilterBackspace,
    /// Enter: keep the filter.
    FilterAccept,
    /// Esc: clear the filter.
    FilterCancel,
    /// Overview activity window: next wider / narrower span.
    WindowWider,
    WindowNarrower,
    /// space: expand / collapse the selected node.
    ToggleExpand,
    NextPane,
    PrevPane,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Tail,
    Top,
    ToggleShowDone,
    ToggleAutoSelect,
    /// `s`: hide / show the navigator (narrow terminals).
    ToggleNav,
    Rescan,
    ToggleHelp,
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
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => Action::WindowWider,
        KeyCode::Char('-') | KeyCode::Char('[') => Action::WindowNarrower,
        KeyCode::Char('j') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Up => Action::Up,
        KeyCode::Right | KeyCode::Char('l') => Action::Right,
        KeyCode::Left | KeyCode::Char('h') => Action::Left,
        KeyCode::Enter => Action::Enter,
        KeyCode::Esc => Action::Back,
        KeyCode::Char(' ') => Action::ToggleExpand,
        KeyCode::Tab => Action::NextPane,
        KeyCode::BackTab => Action::PrevPane,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Char('G') | KeyCode::End => Action::Tail,
        KeyCode::Char('g') | KeyCode::Home => Action::Top,
        KeyCode::Char('d') => Action::ToggleShowDone,
        KeyCode::Char('A') => Action::ToggleAutoSelect,
        KeyCode::Char('s') => Action::ToggleNav,
        KeyCode::Char('i') => Action::Section(DetailSection::Instructions),
        KeyCode::Char('n') => Action::Section(DetailSection::Now),
        KeyCode::Char('r') => Action::Section(DetailSection::Result),
        KeyCode::Char('t') => Action::Section(DetailSection::Timeline),
        KeyCode::Char('/') => Action::StartFilter,
        KeyCode::Char('R') => Action::Rescan,
        KeyCode::Char('?') => Action::ToggleHelp,
        _ => return None,
    })
}

/// Key → action in the current UI state: while the `/` filter is typed, text keys
/// edit it (arrows still move the selection); otherwise [`action_for`].
pub fn action_in(ui: &UiState, key: KeyEvent) -> Option<Action> {
    if !ui.filter_editing || ui.show_help {
        return action_for(key);
    }
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char(_) if ctrl => return None,
        KeyCode::Char(c) => Action::FilterChar(c),
        KeyCode::Backspace => Action::FilterBackspace,
        KeyCode::Enter => Action::FilterAccept,
        KeyCode::Esc => Action::FilterCancel,
        KeyCode::Down => Action::Down,
        KeyCode::Up => Action::Up,
        _ => return None,
    })
}

/// Row of the current selection. Several keys can arrive between two frames, all
/// applied against the same (last drawn) screen, so `ui.selected` — updated by
/// each move — is authoritative; the screen's highlight is the fallback.
fn cursor(ui: &UiState, vm: &WatchScreenVm) -> Option<usize> {
    ui.selected
        .as_deref()
        .and_then(|k| vm.nav.rows.iter().position(|r| r.key == k))
        .or_else(|| vm.selected_index())
}

/// Select navigator node `key`; a new node resets the content (the detail picks
/// its section again, scrolls start over, the feed follows).
fn select(ui: &mut UiState, key: &str) {
    if ui.selected.as_deref() != Some(key) {
        reset_content(ui);
    }
    ui.selected = Some(key.to_string());
    ui.auto_select = false;
}

fn reset_content(ui: &mut UiState) {
    ui.root_detail = false;
    ui.detail_auto = true;
    ui.detail_scroll = initial_detail_scroll();
    ui.content_scroll = 0;
    ui.feed_scroll = Scroll::Follow;
}

/// Move the navigator selection to row `idx` (clamped).
fn select_row(ui: &mut UiState, vm: &WatchScreenVm, idx: usize) {
    let rows = &vm.nav.rows;
    if let Some(r) = rows.get(idx.min(rows.len().saturating_sub(1))) {
        select(ui, &r.key.clone());
    }
}

/// Expanded state of `row` as the user sees it (overrides applied between frames).
fn is_open(ui: &UiState, row: &NavRowVm) -> bool {
    match ui.open.get(&row.key) {
        Some(open) if ui.filter.is_empty() => *open,
        _ => row.expanded,
    }
}

/// Expand / collapse `row` (not the top node) and say so.
fn set_open(ui: &mut UiState, row: &NavRowVm, open: bool) -> String {
    ui.open.insert(row.key.clone(), open);
    let label = super::style::clip(&row.label, 48);
    if open {
        format!("▾ expanded {label}")
    } else {
        format!("▸ collapsed {label}")
    }
}

/// Focus `s` and remember it for the next agents' details.
fn focus_section(ui: &mut UiState, s: DetailSection) {
    ui.detail_section = s;
    ui.detail_pref = Some(s);
    ui.detail_auto = false;
}

/// `i` `n` `r` `t`: the detail of the selected agent at section `s` (a session's
/// root agent; a folded group's first item; the first session from the top
/// node), with the content focused.
fn show_section(ui: &mut UiState, vm: &WatchScreenVm, s: DetailSection) -> Option<String> {
    let row = cursor(ui, vm).and_then(|i| vm.nav.rows.get(i))?.clone();
    match row.kind {
        NavKind::Agent => {}
        NavKind::Session => {
            if !has_transcript(vm, &row.key) {
                return Some(format!("{} has no transcript yet", row.label));
            }
            ui.root_detail = true;
        }
        NavKind::Top => {
            let Some(first) = vm
                .nav
                .rows
                .iter()
                .find(|r| r.kind == NavKind::Session && has_transcript(vm, &r.key))
            else {
                return Some("no session to show".to_string());
            };
            select(ui, &first.key.clone());
            ui.root_detail = true;
        }
        NavKind::Fold | NavKind::Older => {
            let item = row.first_item.clone()?;
            ui.open.insert(row.key.clone(), true);
            select(ui, &item);
            ui.root_detail = row.kind == NavKind::Older;
        }
    }
    focus_section(ui, s);
    ui.focus = Pane::Content;
    None
}

fn has_transcript(vm: &WatchScreenVm, id: &str) -> bool {
    vm.sessions
        .rows
        .iter()
        .find(|s| s.id == id)
        .is_some_and(|s| s.has_transcript)
}

fn window_toast(before: LaneWindow, after: LaneWindow) -> String {
    let what = match after {
        LaneWindow::All => "all (since the oldest agent started)".to_string(),
        w => format!("last {}", w.label()),
    };
    let limit = if before == after {
        match after {
            LaneWindow::All => " — widest",
            _ => " — narrowest",
        }
    } else {
        ""
    };
    format!("activity window: {what}{limit}")
}

/// Scroll the focused detail section by `n` lines (clamped against the last frame;
/// the timeline follows again at its end).
fn scroll_detail(ui: &mut UiState, up: bool, n: usize) {
    let section = ui.detail_section;
    let i = section.index();
    let m = ui.viewport.detail[i];
    let max = m.total.saturating_sub(m.height);
    let cur = ui.detail_scroll[i].start(m.total, m.height);
    ui.detail_scroll[i] = if up {
        Scroll::Offset(cur.saturating_sub(n))
    } else if cur + n >= max && section == DetailSection::Timeline {
        Scroll::Follow
    } else {
        Scroll::Offset((cur + n).min(max))
    };
}

/// Scroll keys in the content pane: the detail section, or the overview lines.
fn scroll_content(ui: &mut UiState, vm: &WatchScreenVm, action: Action) {
    if matches!(vm.content, ContentVm::Agent { .. }) {
        let i = ui.detail_section.index();
        let page = ui.viewport.detail[i].height.max(1);
        match action {
            Action::Up => scroll_detail(ui, true, 1),
            Action::Down => scroll_detail(ui, false, 1),
            Action::PageUp => scroll_detail(ui, true, page),
            Action::PageDown => scroll_detail(ui, false, page),
            Action::HalfPageUp => scroll_detail(ui, true, (page / 2).max(1)),
            Action::HalfPageDown => scroll_detail(ui, false, (page / 2).max(1)),
            Action::Tail => {
                ui.detail_scroll[i] = match ui.detail_section {
                    DetailSection::Timeline => Scroll::Follow,
                    _ => Scroll::Offset(usize::MAX),
                }
            }
            Action::Top => ui.detail_scroll[i] = Scroll::Offset(0),
            _ => {}
        }
        return;
    }
    let m = ui.viewport.content;
    let max = m.total.saturating_sub(m.height);
    let page = m.height.max(1);
    let cur = ui.content_scroll.min(max);
    ui.content_scroll = match action {
        Action::Up => cur.saturating_sub(1),
        Action::Down => (cur + 1).min(max),
        Action::PageUp => cur.saturating_sub(page),
        Action::PageDown => (cur + page).min(max),
        Action::HalfPageUp => cur.saturating_sub((page / 2).max(1)),
        Action::HalfPageDown => (cur + (page / 2).max(1)).min(max),
        Action::Tail => max,
        Action::Top => 0,
        _ => cur,
    };
}

/// Scroll keys in the messages pane (newest at the bottom; the end follows).
fn scroll_feed(ui: &mut UiState, vm: &WatchScreenVm, action: Action) {
    let (total, height) = (vm.feed.len(), ui.viewport.feed);
    let max = total.saturating_sub(height);
    let cur = ui.feed_scroll.start(total, height);
    let page = height.max(1);
    let to = |n: usize| {
        if n >= max {
            Scroll::Follow
        } else {
            Scroll::Offset(n)
        }
    };
    ui.feed_scroll = match action {
        Action::Up => Scroll::Offset(cur.saturating_sub(1)),
        Action::Down => to(cur + 1),
        Action::PageUp => Scroll::Offset(cur.saturating_sub(page)),
        Action::PageDown => to(cur + page),
        Action::HalfPageUp => Scroll::Offset(cur.saturating_sub((page / 2).max(1))),
        Action::HalfPageDown => to(cur + (page / 2).max(1)),
        Action::Tail => Scroll::Follow,
        Action::Top => Scroll::Offset(0),
        _ => ui.feed_scroll,
    };
}

/// Movement keys with the navigator focused.
fn navigate(ui: &mut UiState, vm: &WatchScreenVm, action: Action) -> Option<String> {
    let rows = &vm.nav.rows;
    let idx = cursor(ui, vm).unwrap_or(0);
    let row = rows.get(idx)?.clone();
    let jump = ui.viewport.nav.max(2);
    match action {
        Action::Up => select_row(ui, vm, idx.saturating_sub(1)),
        Action::Down => select_row(ui, vm, idx + 1),
        Action::PageUp => select_row(ui, vm, idx.saturating_sub(jump)),
        Action::PageDown => select_row(ui, vm, idx + jump),
        Action::HalfPageUp => select_row(ui, vm, idx.saturating_sub(jump / 2)),
        Action::HalfPageDown => select_row(ui, vm, idx + jump / 2),
        Action::Top => select_row(ui, vm, 0),
        Action::Tail => select_row(ui, vm, rows.len().saturating_sub(1)),
        Action::Right => {
            if row.expandable && !is_open(ui, &row) {
                return Some(set_open(ui, &row, true));
            }
            let child = rows
                .get(idx + 1)
                .filter(|c| c.parent.as_deref() == Some(row.key.as_str()));
            match child {
                // Opened within this frame: its children are drawn next frame.
                None if row.expandable => {}
                Some(c) => select(ui, &c.key.clone()),
                None => {
                    // A leaf: read it. A session without children shows its root.
                    if row.kind == NavKind::Session && has_transcript(vm, &row.key) {
                        ui.root_detail = true;
                    }
                    ui.focus = Pane::Content;
                }
            }
        }
        Action::Left => {
            if row.kind != NavKind::Top
                && row.expandable
                && ui.filter.is_empty()
                && is_open(ui, &row)
            {
                return Some(set_open(ui, &row, false));
            }
            if let Some(p) = &row.parent {
                select(ui, &p.clone());
            }
        }
        Action::ToggleExpand => {
            if row.kind == NavKind::Top {
                return Some("the top node is always open".to_string());
            }
            if !row.expandable {
                return Some(format!(
                    "{} has no children",
                    super::style::clip(&row.label, 32)
                ));
            }
            let open = !is_open(ui, &row);
            return Some(set_open(ui, &row, open));
        }
        Action::Enter => ui.focus = Pane::Content,
        Action::Back => {
            if !ui.filter.is_empty() {
                ui.filter.clear();
                return Some("filter cleared".to_string());
            }
            if row.kind != NavKind::Top {
                select(ui, NAV_TOP);
            } else if ui.show_done || !ui.open.is_empty() {
                ui.show_done = false;
                ui.open.clear();
                return Some("view reset".to_string());
            }
        }
        _ => {}
    }
    None
}

/// Apply `action` to `ui`; `vm` is the screen currently displayed and `now`
/// stamps the toast raised for the change.
pub fn apply(ui: &mut UiState, action: Action, vm: &WatchScreenVm, now: Instant) -> Effect {
    if ui.show_help {
        match action {
            Action::Quit => return Effect::Quit,
            Action::ToggleHelp | Action::Back => ui.show_help = false,
            _ => {}
        }
        return Effect::None;
    }
    let mut toast: Option<String> = None;
    let mut effect = Effect::None;
    match action {
        Action::Quit => return Effect::Quit,
        Action::Rescan => {
            toast = Some("rescanning…".to_string());
            effect = Effect::Rescan;
        }
        Action::ToggleHelp => ui.show_help = true,
        Action::WindowWider | Action::WindowNarrower => {
            let before = ui.window;
            ui.window = match action {
                Action::WindowWider => before.wider(),
                _ => before.narrower(),
            };
            toast = Some(window_toast(before, ui.window));
        }
        Action::ToggleShowDone => {
            ui.show_done = !ui.show_done;
            toast = Some(if ui.show_done {
                "finished agents shown in place — d to fold".to_string()
            } else {
                "finished agents folded — d to show".to_string()
            });
        }
        Action::ToggleAutoSelect => {
            ui.auto_select = !ui.auto_select;
            toast = Some(format!(
                "follow the most active agent: {}",
                if ui.auto_select { "on" } else { "off" }
            ));
        }
        Action::ToggleNav => {
            toast = Some(if !ui.viewport.narrow {
                "the navigator hides only below 80 columns".to_string()
            } else {
                ui.nav_hidden = !ui.nav_hidden;
                if ui.nav_hidden {
                    if ui.focus == Pane::Navigator {
                        ui.focus = Pane::Content;
                    }
                    "navigator hidden — s to show".to_string()
                } else {
                    "navigator shown — s to hide".to_string()
                }
            });
        }
        Action::NextPane => ui.focus = ui.focus.next(),
        Action::PrevPane => ui.focus = ui.focus.prev(),
        Action::Section(s) => toast = show_section(ui, vm, s),
        Action::StartFilter => {
            ui.filter_editing = true;
            ui.focus = Pane::Navigator;
        }
        Action::FilterChar(c) => {
            ui.filter.push(c);
            // Re-pick: the presenter selects the first match.
            ui.selected = None;
            ui.auto_select = false;
            reset_content(ui);
        }
        Action::FilterBackspace => {
            ui.filter.pop();
            if !ui.filter.is_empty() {
                ui.selected = None;
                reset_content(ui);
            }
        }
        Action::FilterAccept => ui.filter_editing = false,
        Action::FilterCancel => {
            ui.filter_editing = false;
            if !ui.filter.is_empty() {
                ui.filter.clear();
                toast = Some("filter cleared".to_string());
            }
        }
        Action::Up
        | Action::Down
        | Action::Right
        | Action::Left
        | Action::Enter
        | Action::Back
        | Action::ToggleExpand
        | Action::PageUp
        | Action::PageDown
        | Action::HalfPageUp
        | Action::HalfPageDown
        | Action::Tail
        | Action::Top => match ui.focus {
            // While typing the filter, ↑/↓ move between matches.
            _ if ui.filter_editing => toast = navigate(ui, vm, action),
            Pane::Navigator => toast = navigate(ui, vm, action),
            Pane::Content | Pane::Feed if matches!(action, Action::Left | Action::Back) => {
                ui.focus = Pane::Navigator;
            }
            Pane::Content => scroll_content(ui, vm, action),
            Pane::Feed => scroll_feed(ui, vm, action),
        },
    }
    if let Some(text) = toast {
        ui.toast = Some(Toast::new(text, now));
    }
    effect
}

/// Drop the toast once it expired; true when it was visible until now (the
/// screen needs a redraw to clear it).
pub fn expire_toast(ui: &mut UiState, now: Instant) -> bool {
    if ui.toast.as_ref().is_some_and(|t| !t.is_live(now)) {
        ui.toast = None;
        return true;
    }
    false
}

/// Remember the selection the presenter resolved (top node, a shown ancestor of a
/// hidden agent, the first filter match, auto-select) and the detail section it
/// picked, so relative moves start from what is displayed.
pub fn sync_selection(ui: &mut UiState, vm: &WatchScreenVm) {
    if let Some(row) = vm.selected_row()
        && ui.selected.as_deref() != Some(row.key.as_str())
    {
        if ui.selected.is_some() {
            reset_content(ui);
        }
        ui.selected = Some(row.key.clone());
    }
    if ui.detail_auto
        && let Some(d) = &vm.detail
        && ui.selected.as_deref() == Some(d.agent_id.as_str())
    {
        ui.detail_section = d.section;
        ui.detail_auto = false;
    }
}
