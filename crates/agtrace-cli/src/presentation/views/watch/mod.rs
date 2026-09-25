//! Ratatui views of the multi-agent watch TUI (design §6.1).
//!
//! An always-visible navigator on the left ([`navigator`]); on the right the
//! content of the selected node — the overview ([`overview`]: top, session or
//! folded group), the agent detail ([`detail`]) or the older sessions
//! ([`sessions`]) — above the message feed scoped to the selection.
//!
//! ```text
//! ┏ ▶ Navigator ━━━━━━━━━━━━━┓┌ Projects… · claude · ○ idle (bg) · ctx 44% of 1.0M ─┐
//! ┃ ◆ yohaku-studio  3 live  ┃│ agent         status  context  activity   now       │
//! ┃ ▸ ● PR 1693 の継続       ┃│ Projects…     ○ idle  ███░ 44%  ··▃··▅▃  idle 56m   │
//! ┃▶▾ ○ Projects制作のボト… ┃│   ⊘ 20 killed · 3 earlier transcripts  (d to show) │
//! ┃   ├ ✓ S explore call s… ┃├ Messages · this session ────────────────────────────┤
//! ┃   └ ▸ ⊘ 20 killed (d)   ┃│ 12:01 v8fix1 → lead  MESSAGE "done, 3 bugs"         │
//! ┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛└─────────────────────────────────────────────────────┘
//!  yohaku-studio › Projects制作…   2 sessions · …        ↑↓ move · → open · ? help
//! ```
//!
//! Views are stateless: everything comes from [`WatchScreenVm`] (scroll positions
//! included); [`layout`] and [`viewport`] / [`measure`] are shared with the
//! handler so it can size scrolls and pages.

pub mod console;
pub mod detail;
mod feed;
mod help;
pub mod input;
mod navigator;
pub mod overview;
pub mod sessions;
mod status_bar;
mod style;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::presentation::view_models::watch::{
    ContentVm, NARROW_WIDTH, SectionMetrics, UiState, Viewport, WatchScreenVm,
};

/// Screen areas of the watch TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchLayout {
    /// Navigator (zero width when hidden).
    pub nav: Rect,
    pub content: Rect,
    pub feed: Rect,
    pub status: Rect,
    pub area: Rect,
}

/// Navigator `clamp(24, 30%, 40)` columns (hidden with `s` below
/// [`NARROW_WIDTH`]); on the right the content above the feed
/// (`clamp(25% height, 5, 10)` rows incl. borders); one status line.
pub fn layout(area: Rect, nav_hidden: bool) -> WatchLayout {
    let [main, status] = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
    let narrow = area.width < NARROW_WIDTH;
    let nav_w = if narrow && nav_hidden {
        0
    } else {
        (area.width * 30 / 100).clamp(24, 40).min(area.width)
    };
    let [nav, right] =
        Layout::horizontal([Constraint::Length(nav_w), Constraint::Min(0)]).areas(main);
    let feed_h = (main.height / 4)
        .clamp(5, 10)
        .min(main.height.saturating_sub(3));
    let [content, feed] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(feed_h)]).areas(right);
    WatchLayout {
        nav,
        content,
        feed,
        status,
        area,
    }
}

/// Pane sizes for building the next frame (the content-dependent metrics are
/// filled in by [`measure`] once the frame is built).
pub fn viewport(area: Rect, ui: &UiState) -> Viewport {
    let l = layout(area, ui.nav_hidden);
    let inner = |r: Rect| r.height.saturating_sub(2) as usize;
    Viewport {
        nav: inner(l.nav),
        feed: inner(l.feed),
        lane_cols: overview::columns(l.content.width.saturating_sub(2) as usize).lane,
        narrow: area.width < NARROW_WIDTH,
        content: SectionMetrics::default(),
        detail: Default::default(),
    }
}

/// Fill in the content metrics of the frame `vm` (wrapped detail sections,
/// overview lines), so scroll keys clamp against what is drawn.
pub fn measure(area: Rect, vm: &WatchScreenVm, vp: &mut Viewport) {
    let l = layout(area, vm.nav_hidden);
    vp.detail = detail::metrics(l.content, vm);
    vp.content = match vm.content {
        ContentVm::Older { .. } => sessions::metrics(l.content, vm),
        ContentVm::Agent { .. } => SectionMetrics::default(),
        _ => overview::metrics(l.content, vm),
    };
}

/// Draw the whole screen.
pub fn render(f: &mut Frame, vm: &WatchScreenVm) {
    let l = layout(f.area(), vm.nav_hidden);
    if l.nav.width > 0 {
        navigator::render(f, l.nav, vm);
    }
    match vm.content {
        ContentVm::Agent { .. } => detail::render(f, l.content, vm),
        ContentVm::Older { .. } => sessions::render_older(f, l.content, vm),
        _ => overview::render(f, l.content, vm),
    }
    feed::render(f, l.feed, vm);
    status_bar::render(f, l.status, vm);
    if vm.show_help {
        help::render(f, f.area());
    }
}
