//! `agtrace watch`: multi-agent TUI loop and console line printer (design §6.3).
//!
//! Both loops are independent of where the [`WorkspaceView`] comes from: anything
//! implementing [`WorkspaceSource`] (a generation counter + read access to the
//! view) can drive them. The SDK's [`LiveWorkspace`] plugs in through
//! [`LiveSource`]; tests and the preview example use [`SharedWorkspace`].
//!
//! Redraws happen at most every 100 ms (10 fps), and only when the generation
//! changed, a key was pressed, the terminal was resized, a toast expired, or once
//! per second so that elapsed times keep ticking.

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use agtrace_sdk::Client;
use agtrace_sdk::types::AgentId;
use agtrace_sdk::watch::{LiveWorkspace, WatchScope};
use agtrace_sdk::workspace::WorkspaceView;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Offset, Utc};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use crate::args::WatchFormat;
use crate::presentation::presenters::watch::{build_console, build_screen};
use crate::presentation::view_models::watch::{ConsoleVm, StatusVm, UiState, WatchScreenVm};
use crate::presentation::views::watch::input::{
    Effect, action_in, apply, expire_toast, sync_selection,
};
use crate::presentation::views::watch::{console, measure, render, viewport};

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

    /// `R` key: run a discovery pass now.
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
        if drawn_generation != Some(generation)
            || since_draw.is_none_or(|d| d >= CLOCK_REDRAW)
            || expire_toast(&mut ui, Instant::now())
        {
            dirty = true;
        }
        if dirty && since_draw.is_none_or(|d| d >= FRAME) {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            ui.viewport = viewport(area, &ui);
            let vm = build(source, &ui);
            // Content wraps / folds its text: scroll keys clamp against this frame.
            measure(area, &vm, &mut ui.viewport);
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
                let (Some(action), Some(vm)) = (action_in(&ui, key), screen.as_ref()) else {
                    continue;
                };
                match apply(&mut ui, action, vm, Instant::now()) {
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

// ============================================================================
// Live source + command entry
// ============================================================================

/// [`WorkspaceSource`] over the SDK's [`LiveWorkspace`] (a newtype: both are foreign
/// to the view layer's trait otherwise).
pub struct LiveSource(pub LiveWorkspace);

impl WorkspaceSource for LiveSource {
    fn generation(&self) -> u64 {
        self.0.generation()
    }

    fn with_view(&self, f: &mut dyn FnMut(&WorkspaceView)) {
        self.0.with_view(|v| f(v));
    }

    fn rescan(&self) {
        self.0.rescan();
    }
}

/// What `agtrace watch` watches.
pub enum WatchTarget {
    /// Live root agents of a project (cwd under `root`) active within `since`.
    Project { root: PathBuf, since: Duration },
    /// One root session (index session id / prefix, or an agent id) and its tree.
    Session(String),
}

/// `agtrace watch`.
pub fn handle(client: &Client, target: WatchTarget, mode: WatchFormat) -> Result<()> {
    let (scope, label) = resolve_scope(client, target)?;
    let live = client
        .watch_workspace(scope)
        .context("failed to start the workspace watcher")?;
    let source = LiveSource(live);
    let ui = UiState::new(label, Local::now().offset().fix());
    match mode {
        WatchFormat::Tui => run(&source, ui),
        WatchFormat::Console => run_console(&source, ui, &mut io::stdout().lock(), None),
    }
}

fn resolve_scope(client: &Client, target: WatchTarget) -> Result<(WatchScope, String)> {
    match target {
        WatchTarget::Project { root, since } => {
            let name = if root.parent().is_none() {
                "all projects".to_string()
            } else {
                let name = root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| root.display().to_string());
                format!("project {name}")
            };
            let label = format!("{name} · since {}", format_since(since));
            Ok((WatchScope::Project { root, since }, label))
        }
        WatchTarget::Session(id) => {
            let agent = match AgentId::parse(&id) {
                Some(agent) => agent,
                None => {
                    let handle = client.sessions().get(&id).with_context(|| {
                        format!("session {id} not found in the index (run `agtrace init`)")
                    })?;
                    let meta = handle
                        .metadata()?
                        .ok_or_else(|| anyhow::anyhow!("session {id}: no metadata"))?;
                    match meta.provider.as_str() {
                        "codex" => AgentId::codex_thread(&meta.session_id),
                        _ => AgentId::claude_session(&meta.session_id),
                    }
                }
            };
            let short: String = agent.native_session_id().chars().take(8).collect();
            Ok((WatchScope::Root(agent), format!("session {short}")))
        }
    }
}

fn format_since(d: Duration) -> String {
    let secs = d.as_secs();
    match secs {
        s if s % 86_400 == 0 && s > 0 => format!("{}d", s / 86_400),
        s if s % 3_600 == 0 && s > 0 => format!("{}h", s / 3_600),
        s if s % 60 == 0 && s > 0 => format!("{}m", s / 60),
        s => format!("{s}s"),
    }
}

// ============================================================================
// Console mode
// ============================================================================

/// How often the console printer checks the generation counter.
const CONSOLE_POLL: Duration = Duration::from_millis(250);

/// Remembers what was printed so each agent / row / feed entry appears once.
#[derive(Default)]
pub struct ConsolePrinter {
    agents: HashMap<String, StatusVm>,
    rows: HashSet<String>,
    feed: HashSet<String>,
}

impl ConsolePrinter {
    /// Lines for everything new in `vm` since the previous call.
    pub fn lines(&mut self, vm: &ConsoleVm) -> Vec<String> {
        let mut out = Vec::new();
        for row in &vm.tree {
            match self.agents.insert(row.id.clone(), row.status) {
                None => out.push(console::agent_added(row)),
                Some(prev) if prev != row.status => out.push(console::agent_status(row)),
                Some(_) => {}
            }
        }
        for t in &vm.timelines {
            for r in &t.rows {
                if self.rows.insert(format!("{}#{}", t.agent_id, r.key)) {
                    out.push(console::timeline(&t.label, &r.row));
                }
            }
        }
        for (key, row) in vm.feed_keys.iter().zip(&vm.feed) {
            if self.feed.insert(key.clone()) {
                out.push(console::feed(row));
            }
        }
        out
    }
}

/// Print the workspace as lines until interrupted (or `limit` elapses; tests).
pub fn run_console(
    source: &dyn WorkspaceSource,
    ui: UiState,
    out: &mut dyn Write,
    limit: Option<Duration>,
) -> Result<()> {
    writeln!(out, "watching {} (Ctrl-C to stop)", ui.scope)?;
    let started = Instant::now();
    let mut printer = ConsolePrinter::default();
    let mut printed: Option<u64> = None;
    loop {
        let generation = source.generation();
        if printed != Some(generation) {
            let now = source.now();
            let mut vm = None;
            source.with_view(&mut |v| vm = Some(build_console(v, &ui, now)));
            if let Some(vm) = vm {
                for line in printer.lines(&vm) {
                    writeln!(out, "{line}")?;
                }
                out.flush()?;
            }
            printed = Some(generation);
        }
        if limit.is_some_and(|l| started.elapsed() >= l) {
            return Ok(());
        }
        std::thread::sleep(CONSOLE_POLL);
    }
}
