//! Keybindings (design §6.2) and the UI-state reducer.
//!
//! Three screens: `1` overview (home), `2` agents (tree + timeline + feed), and
//! the agent detail. Drill-down model: Enter / → / l open the selected agent's
//! detail (from the overview or the agents screen), Esc / ← / h go back one
//! level (close help, leave the detail to where it was opened from, return to the
//! tree, then reset the view toggles). j/k act on the focused pane or section.
//!
//! Direct keys: `i` `n` `r` `t` focus the Instructions / Now / Result / Timeline
//! section of the detail (from the overview or the agents screen they open the
//! detail at that section); `J` / `K` step to the next / previous agent; `/` types
//! a name filter (`Enter` opens the selected match, `Esc` clears it).
//!
//! [`action_in`] maps a key to an [`Action`] (the filter prompt takes typed text);
//! [`apply`] updates the [`UiState`]
//! against the last rendered screen (row order, row counts), raises a toast for
//! every state change, and reports effects that leave the UI (quit, rescan).

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::presentation::view_models::watch::{
    AgentRowVm, DetailSection, FeedFilter, LaneWindow, Pane, Screen, Scroll, Toast, UiState,
    WatchScreenVm, initial_detail_scroll,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    /// Dive into the selected agent: open its detail screen.
    Open,
    /// Back one level: close help, leave the detail, return to the tree, or reset
    /// the view.
    Back,
    ShowOverview,
    ShowAgents,
    /// Focus a detail section (opens the detail from the other screens).
    Section(DetailSection),
    /// Next / previous agent in tree order (detail: open its detail instead).
    NextAgent,
    PrevAgent,
    /// `/`: start typing the name filter.
    StartFilter,
    /// While typing the filter.
    FilterChar(char),
    FilterBackspace,
    /// Enter: keep the filter and open the selected match.
    FilterAccept,
    /// Esc: clear the filter.
    FilterCancel,
    /// Overview activity window: next wider / narrower span.
    WindowWider,
    WindowNarrower,
    ToggleCollapse,
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
        KeyCode::Char('1') => Action::ShowOverview,
        KeyCode::Char('2') => Action::ShowAgents,
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => Action::WindowWider,
        KeyCode::Char('-') | KeyCode::Char('[') => Action::WindowNarrower,
        KeyCode::Char('j') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Up => Action::Up,
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Action::Open,
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => Action::Back,
        KeyCode::Char(' ') => Action::ToggleCollapse,
        KeyCode::Tab => Action::NextPane,
        KeyCode::BackTab => Action::PrevPane,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Char('G') | KeyCode::End => Action::Tail,
        KeyCode::Char('g') | KeyCode::Home => Action::Top,
        KeyCode::Char('f') => Action::ToggleFeedFilter,
        KeyCode::Char('d') => Action::ToggleHideDone,
        KeyCode::Char('a') => Action::ToggleAutoSelect,
        KeyCode::Char('i') => Action::Section(DetailSection::Instructions),
        KeyCode::Char('n') => Action::Section(DetailSection::Now),
        KeyCode::Char('r') => Action::Section(DetailSection::Result),
        KeyCode::Char('t') => Action::Section(DetailSection::Timeline),
        KeyCode::Char('J') => Action::NextAgent,
        KeyCode::Char('K') => Action::PrevAgent,
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

/// Row of the current selection. Several keys can arrive between two frames, all
/// applied against the same (last drawn) screen, so `ui.selected` — updated by
/// each move — is authoritative; the screen's highlight is the fallback.
fn cursor(ui: &UiState, vm: &WatchScreenVm) -> Option<usize> {
    ui.selected
        .as_deref()
        .and_then(|id| vm.tree.iter().position(|r| r.id == id))
        .or_else(|| vm.selected_index())
}

/// Toggle the collapse state of the selected row and describe the outcome.
///
/// Guarded: a leaf has nothing to fold, and the sole root stays expanded (folding
/// it would hide the whole tree and make a single-session watch look empty).
fn toggle_collapse(ui: &mut UiState, vm: &WatchScreenVm) -> Option<String> {
    let idx = cursor(ui, vm)?;
    let row = &vm.tree[idx];
    // `ui.collapsed`, not the (possibly stale) row flag: space space between two
    // frames must fold and unfold.
    if ui.collapsed.remove(&row.id) {
        return Some(format!("▾ expanded {}", row.label));
    }
    if !row.has_children {
        return Some(format!("{} has no children", row.label));
    }
    let roots = vm.tree.iter().filter(|r| r.depth == 0).count();
    if row.depth == 0 && roots == 1 {
        return Some("can't collapse the only session".to_string());
    }
    ui.collapsed.insert(row.id.clone());
    Some(format!(
        "▸ collapsed {} (+{} hidden)",
        row.label,
        subtree_size(&vm.tree, idx)
    ))
}

/// Descendants of `tree[idx]` (shown rows plus those folded under them).
fn subtree_size(tree: &[AgentRowVm], idx: usize) -> usize {
    let depth = tree[idx].depth;
    tree[idx + 1..]
        .iter()
        .take_while(|r| r.depth > depth)
        .map(|r| 1 + r.hidden_descendants)
        .sum()
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

/// Open the detail screen of the agent under the cursor, at `section` or (None)
/// at the section the presenter picks (the remembered one if it has content).
fn open_detail(ui: &mut UiState, vm: &WatchScreenVm, section: Option<DetailSection>) {
    let Some(row) = cursor(ui, vm).and_then(|i| vm.tree.get(i)) else {
        return;
    };
    ui.detail_return = ui.screen;
    show_detail(ui, &row.id.clone(), section);
}

/// Point the detail screen at agent `id` (scrolls reset).
fn show_detail(ui: &mut UiState, id: &str, section: Option<DetailSection>) {
    ui.selected = Some(id.to_string());
    ui.detail_agent = Some(id.to_string());
    ui.screen = Screen::Detail;
    ui.detail_scroll = initial_detail_scroll();
    match section {
        Some(s) => focus_section(ui, s),
        None => ui.detail_auto = true,
    }
}

/// Focus `s` and remember it for the next agents' details.
fn focus_section(ui: &mut UiState, s: DetailSection) {
    ui.detail_section = s;
    ui.detail_pref = Some(s);
    ui.detail_auto = false;
}

/// Move the selection one row down / up the tree (saturating).
fn step_selection(ui: &mut UiState, vm: &WatchScreenVm, down: bool) {
    let next = match cursor(ui, vm) {
        None => 0,
        Some(i) if down => (i + 1).min(vm.tree.len().saturating_sub(1)),
        Some(i) => i.saturating_sub(1),
    };
    select(ui, vm, next);
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

/// Esc on the tree: back to the default screen (selection and auto-select are kept).
fn reset_view(ui: &mut UiState) {
    ui.collapsed.clear();
    ui.filter.clear();
    ui.hide_done = false;
    ui.feed_filter = FeedFilter::All;
    ui.focus = Pane::Tree;
    ui.timeline_scroll = Scroll::Follow;
    ui.feed_scroll = Scroll::Follow;
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
    if ui.screen == Screen::Detail {
        return apply_detail(ui, action, vm, now);
    }
    let page = |h: usize| h.max(1);
    let target = scroll_target(ui);
    let height = match target {
        Pane::Feed => ui.viewport.feed,
        _ => ui.viewport.timeline,
    };
    let mut toast: Option<String> = None;
    let mut effect = Effect::None;
    match action {
        Action::Quit => return Effect::Quit,
        Action::Rescan => {
            toast = Some("rescanning…".to_string());
            effect = Effect::Rescan;
        }
        Action::ShowOverview => ui.screen = Screen::Overview,
        Action::ShowAgents => ui.screen = Screen::Agents,
        Action::WindowWider | Action::WindowNarrower => {
            let before = ui.window;
            ui.window = match action {
                Action::WindowWider => before.wider(),
                _ => before.narrower(),
            };
            toast = Some(window_toast(before, ui.window));
        }
        Action::Up | Action::Down
            if ui.focus == Pane::Tree || ui.screen == Screen::Overview || ui.filter_editing =>
        {
            step_selection(ui, vm, action == Action::Down);
        }
        Action::NextAgent => step_selection(ui, vm, true),
        Action::PrevAgent => step_selection(ui, vm, false),
        Action::Section(s) => open_detail(ui, vm, Some(s)),
        Action::StartFilter => {
            ui.filter_editing = true;
            ui.focus = Pane::Tree;
        }
        Action::FilterChar(c) => {
            ui.filter.push(c);
            // Re-pick: the presenter selects the first match.
            ui.selected = None;
            ui.auto_select = false;
        }
        Action::FilterBackspace => {
            ui.filter.pop();
            if !ui.filter.is_empty() {
                ui.selected = None;
            }
        }
        Action::FilterAccept => {
            ui.filter_editing = false;
            if !ui.filter.is_empty() {
                open_detail(ui, vm, None);
            }
        }
        Action::FilterCancel => {
            ui.filter_editing = false;
            if !ui.filter.is_empty() {
                ui.filter.clear();
                toast = Some("filter cleared".to_string());
            }
        }
        // The overview has no scrollable pane: page / end keys move the selection.
        Action::PageUp
        | Action::PageDown
        | Action::HalfPageUp
        | Action::HalfPageDown
        | Action::Tail
        | Action::Top
            if ui.screen == Screen::Overview =>
        {
            let last = vm.tree.len().saturating_sub(1);
            let cur = cursor(ui, vm).unwrap_or(0);
            let jump = ui.viewport.tree.max(2) / 2;
            let next = match action {
                Action::PageUp | Action::HalfPageUp => cur.saturating_sub(jump),
                Action::PageDown | Action::HalfPageDown => (cur + jump).min(last),
                Action::Tail => last,
                _ => 0,
            };
            select(ui, vm, next);
        }
        Action::NextPane | Action::PrevPane if ui.screen == Screen::Overview => {}
        Action::Up => scroll(ui, vm, target, true, 1),
        Action::Down => scroll(ui, vm, target, false, 1),
        Action::PageUp => scroll(ui, vm, target, true, page(height)),
        Action::PageDown => scroll(ui, vm, target, false, page(height)),
        Action::HalfPageUp => scroll(ui, vm, target, true, page(height / 2)),
        Action::HalfPageDown => scroll(ui, vm, target, false, page(height / 2)),
        Action::Tail => set_scroll(ui, target, Scroll::Follow),
        Action::Top => set_scroll(ui, target, Scroll::Offset(0)),
        Action::Open => open_detail(ui, vm, None),
        Action::Back => match ui.focus {
            Pane::Timeline | Pane::Feed if ui.screen == Screen::Agents => ui.focus = Pane::Tree,
            _ if !ui.filter.is_empty() => {
                ui.filter.clear();
                toast = Some("filter cleared".to_string());
            }
            _ => {
                reset_view(ui);
                toast = Some("view reset".to_string());
            }
        },
        Action::ToggleCollapse => toast = toggle_collapse(ui, vm),
        Action::NextPane => ui.focus = ui.focus.next(),
        Action::PrevPane => ui.focus = ui.focus.prev(),
        Action::ToggleFeedFilter => {
            ui.feed_filter = match ui.feed_filter {
                FeedFilter::All => FeedFilter::Selected,
                FeedFilter::Selected => FeedFilter::All,
            };
            ui.feed_scroll = Scroll::Follow;
            toast = Some(match ui.feed_filter {
                FeedFilter::Selected => format!("messages: {} only — f for all", vm.focus.title),
                FeedFilter::All => "messages: all".to_string(),
            });
        }
        Action::ToggleHideDone => {
            ui.hide_done = !ui.hide_done;
            toast = Some(if ui.hide_done {
                format!(
                    "done agents hidden ({}) — d to show",
                    vm.status.done_hideable
                )
            } else {
                "done agents shown".to_string()
            });
        }
        Action::ToggleAutoSelect => {
            ui.auto_select = !ui.auto_select;
            toast = Some(format!(
                "auto-select {}",
                if ui.auto_select { "on" } else { "off" }
            ));
        }
        Action::ToggleHelp => ui.show_help = true,
    }
    if let Some(text) = toast {
        ui.toast = Some(Toast::new(text, now));
    }
    effect
}

/// Keys on the detail screen: sections instead of panes, Esc back to where the
/// detail was opened from; the view toggles of the other screens do not apply.
fn apply_detail(ui: &mut UiState, action: Action, vm: &WatchScreenVm, now: Instant) -> Effect {
    commit_detail_section(ui, vm);
    let i = ui.detail_section.index();
    let page = ui.viewport.detail[i].height.max(1);
    match action {
        Action::Quit => return Effect::Quit,
        Action::Rescan => {
            ui.toast = Some(Toast::new("rescanning…", now));
            return Effect::Rescan;
        }
        Action::Back => ui.screen = ui.detail_return,
        Action::ShowOverview => ui.screen = Screen::Overview,
        Action::ShowAgents => ui.screen = Screen::Agents,
        Action::NextPane => focus_section(ui, ui.detail_section.next()),
        Action::PrevPane => focus_section(ui, ui.detail_section.prev()),
        Action::Section(s) => focus_section(ui, s),
        Action::NextAgent | Action::PrevAgent => {
            let cur = ui
                .detail_agent
                .as_deref()
                .and_then(|id| vm.tree.iter().position(|r| r.id == id));
            let next = match (cur, action) {
                (Some(i), Action::NextAgent) => (i + 1 < vm.tree.len()).then_some(i + 1),
                (Some(i), _) => i.checked_sub(1),
                (None, _) => (!vm.tree.is_empty()).then_some(0),
            };
            match next.and_then(|n| vm.tree.get(n)) {
                Some(row) => {
                    let id = row.id.clone();
                    show_detail(ui, &id, None);
                    ui.auto_select = false;
                }
                None => {
                    let edge = if action == Action::NextAgent {
                        "last"
                    } else {
                        "first"
                    };
                    ui.toast = Some(Toast::new(format!("already at the {edge} agent"), now));
                }
            }
        }
        Action::StartFilter => {
            ui.screen = ui.detail_return;
            ui.filter_editing = true;
            ui.focus = Pane::Tree;
        }
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
        Action::ToggleHelp => ui.show_help = true,
        Action::WindowWider | Action::WindowNarrower => {
            // The window belongs to the overview; apply it for when we return.
            let before = ui.window;
            ui.window = match action {
                Action::WindowWider => before.wider(),
                _ => before.narrower(),
            };
            ui.toast = Some(Toast::new(window_toast(before, ui.window), now));
        }
        Action::Open
        | Action::ToggleCollapse
        | Action::ToggleFeedFilter
        | Action::ToggleHideDone
        | Action::ToggleAutoSelect
        | Action::FilterChar(_)
        | Action::FilterBackspace
        | Action::FilterAccept
        | Action::FilterCancel => {}
    }
    Effect::None
}

/// Adopt the section the presenter picked for a just-opened detail (once shown).
fn commit_detail_section(ui: &mut UiState, vm: &WatchScreenVm) {
    if ui.detail_auto
        && let Some(d) = &vm.detail
        && ui.detail_agent.as_deref() == Some(d.agent_id.as_str())
    {
        ui.detail_section = d.section;
        ui.detail_auto = false;
    }
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

/// Remember the selection (and detail section) the presenter resolved (first row, auto-select, or the
/// nearest shown ancestor), so relative moves start from what is displayed.
pub fn sync_selection(ui: &mut UiState, vm: &WatchScreenVm) {
    commit_detail_section(ui, vm);
    if let Some(id) = &vm.focus.agent_id
        && ui.selected.as_ref() != Some(id)
    {
        if ui.selected.is_some() {
            ui.timeline_scroll = Scroll::Follow;
        }
        ui.selected = Some(id.clone());
    }
}
