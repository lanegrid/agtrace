//! Ratatui views of the multi-agent watch TUI (design §6.1).
//!
//! ```text
//! ┌ Agents ─────────┐┌ lead · model · 42% of 1.0M [1m] ──┐
//! │▶ lead   ● busy 42%││ now ▸ Bash  mise run test (12s)  │
//! │  ├ T audit-A ...  ││ 12:01 ▸ Bash  mise run test       │
//! └──────────────────┘└───────────────────────────────────┘
//! ┌ Messages ─────────────────────────────────────────────┐
//! │ 12:01 audit-A → lead   MESSAGE   "done, found 3 bugs" │
//! └───────────────────────────────────────────────────────┘
//!  scope · agents · diagnostics · ?:help
//! ```
//!
//! Views are stateless: everything comes from [`WatchScreenVm`] (scroll positions
//! included); [`layout`] is shared with the handler so it can size page scrolls.

mod feed;
mod focus;
mod help;
pub mod input;
mod status_bar;
mod style;
mod tree;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::presentation::view_models::agent_watch::{Viewport, WatchScreenVm};

/// Screen areas of the watch TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchLayout {
    pub tree: Rect,
    pub focus: Rect,
    pub feed: Rect,
    pub status: Rect,
}

impl WatchLayout {
    /// Content rows of the scrollable panes (inside borders; the focus pane's
    /// first row is the pinned activity line).
    pub fn viewport(&self) -> Viewport {
        let inner = |r: Rect| r.height.saturating_sub(2) as usize;
        Viewport {
            tree: inner(self.tree),
            timeline: inner(self.focus).saturating_sub(1),
            feed: inner(self.feed),
        }
    }
}

/// Tree width `clamp(28, 34%, 48)`, feed `clamp(25% height, 5, 10)` rows incl.
/// borders, one status line.
pub fn layout(area: Rect) -> WatchLayout {
    let feed_h = (area.height / 4).clamp(5, 10);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(feed_h),
            Constraint::Length(1),
        ])
        .split(area);
    let tree_w = (area.width * 34 / 100).clamp(28, 48).min(area.width);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(tree_w), Constraint::Min(0)])
        .split(rows[0]);
    WatchLayout {
        tree: top[0],
        focus: top[1],
        feed: rows[1],
        status: rows[2],
    }
}

/// Draw the whole screen.
pub fn render(f: &mut Frame, vm: &WatchScreenVm) {
    let l = layout(f.area());
    let vp = l.viewport();
    tree::render(f, l.tree, vm, vp.tree);
    focus::render(f, l.focus, vm, vp.timeline);
    feed::render(f, l.feed, vm, vp.feed);
    status_bar::render(f, l.status, vm);
    if vm.show_help {
        help::render(f, f.area());
    }
}
