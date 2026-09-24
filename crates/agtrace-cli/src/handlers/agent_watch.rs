//! Multi-agent watch TUI loop (design §6.3).
//!
//! The loop is independent of where the [`WorkspaceView`] comes from: anything
//! implementing [`WorkspaceSource`] (a generation counter + read access to the
//! view) can drive it. The runtime workspace watcher plugs in here; tests and the
//! preview example use [`SharedWorkspace`].
//!
//! Redraws happen at most every 100 ms (10 fps), and only when the generation
//! changed, a key was pressed, the terminal was resized, or once per second so
//! that elapsed times keep ticking.

use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use agtrace_sdk::workspace::WorkspaceView;
use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use crate::presentation::presenters::agent_watch::build_screen;
use crate::presentation::view_models::agent_watch::{UiState, WatchScreenVm};
use crate::presentation::views::agent_watch::input::{Effect, action_for, apply, sync_selection};
use crate::presentation::views::agent_watch::{layout, render};

/// Minimum interval between two frames.
const FRAME: Duration = Duration::from_millis(100);
/// Redraw at least this often (elapsed-time counters).
const CLOCK_REDRAW: Duration = Duration::from_secs(1);

/// A live (or static) workspace the TUI can display.
pub trait WorkspaceSource {
    /// Counter bumped whenever the view changed; the TUI redraws when it differs
    /// from the last drawn value.
    fn generation(&self) -> u64;

    /// Run `f` with read access to the current view (keep it short: it may hold a lock).
    fn with_view(&self, f: &mut dyn FnMut(&WorkspaceView));

    /// Clock used for elapsed times.
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    /// `r` key: run a discovery pass now.
    fn rescan(&self) {}
}

/// A `WorkspaceView` behind `Arc<RwLock>` plus a generation counter; writers call
/// [`SharedWorkspace::update`], which bumps the generation.
#[derive(Clone, Default)]
pub struct SharedWorkspace {
    view: Arc<RwLock<WorkspaceView>>,
    generation: Arc<AtomicU64>,
    clock: Option<DateTime<Utc>>,
}

impl SharedWorkspace {
    pub fn new(view: WorkspaceView) -> Self {
        Self {
            view: Arc::new(RwLock::new(view)),
            generation: Arc::new(AtomicU64::new(0)),
            clock: None,
        }
    }

    /// Freeze the clock (fixtures whose timestamps are not "now").
    pub fn with_clock(mut self, now: DateTime<Utc>) -> Self {
        self.clock = Some(now);
        self
    }

    pub fn update(&self, f: impl FnOnce(&mut WorkspaceView)) {
        let mut guard = self.view.write().unwrap_or_else(|e| e.into_inner());
        f(&mut guard);
        self.generation.fetch_add(1, Ordering::Release);
    }
}

impl WorkspaceSource for SharedWorkspace {
    fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    fn with_view(&self, f: &mut dyn FnMut(&WorkspaceView)) {
        let guard = self.view.read().unwrap_or_else(|e| e.into_inner());
        f(&guard);
    }

    fn now(&self) -> DateTime<Utc> {
        self.clock.unwrap_or_else(Utc::now)
    }
}

/// Build the screen for the current source state (also used by tests).
pub fn build(source: &dyn WorkspaceSource, ui: &UiState) -> WatchScreenVm {
    let now = source.now();
    let mut out = None;
    source.with_view(&mut |v| out = Some(build_screen(v, ui, now)));
    out.expect("with_view must call the closure")
}

/// Restores the terminal on drop (also on panic via the hook installed in `enter`).
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous(info);
        }));
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        terminal.hide_cursor()?;
        terminal.clear()?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        restore_terminal();
    }
}

/// Run the TUI until the user quits.
pub fn run(source: &dyn WorkspaceSource, mut ui: UiState) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let terminal = &mut guard.terminal;

    let mut screen: Option<WatchScreenVm> = None;
    let mut drawn_generation: Option<u64> = None;
    let mut last_draw: Option<Instant> = None;
    let mut dirty = true;

    loop {
        let generation = source.generation();
        let since_draw = last_draw.map(|t| t.elapsed());
        if drawn_generation != Some(generation) || since_draw.is_none_or(|d| d >= CLOCK_REDRAW) {
            dirty = true;
        }
        if dirty && since_draw.is_none_or(|d| d >= FRAME) {
            let size = terminal.size()?;
            ui.viewport = layout(Rect::new(0, 0, size.width, size.height)).viewport();
            let vm = build(source, &ui);
            sync_selection(&mut ui, &vm);
            terminal.draw(|f| render(f, &vm))?;
            screen = Some(vm);
            drawn_generation = Some(generation);
            last_draw = Some(Instant::now());
            dirty = false;
        }

        let wait = match last_draw {
            Some(t) if dirty => FRAME.saturating_sub(t.elapsed()),
            _ => FRAME,
        };
        if !event::poll(wait)? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => {
                let (Some(action), Some(vm)) = (action_for(key), screen.as_ref()) else {
                    continue;
                };
                match apply(&mut ui, action, vm) {
                    Effect::Quit => break,
                    Effect::Rescan => source.rescan(),
                    Effect::None => {}
                }
                dirty = true;
            }
            Event::Resize(_, _) => dirty = true,
            _ => {}
        }
    }
    Ok(())
}
