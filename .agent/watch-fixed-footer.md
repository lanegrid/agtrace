# Add Fixed Footer to Watch Command (TUI Mode)

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with `.agent/PLANS.md`.


## Purpose / Big Picture

After this change, users running `agtrace watch` will see a terminal user interface (TUI) with a scrollable event log area and a fixed footer displaying real-time Context Window usage statistics. Currently, the watch command prints events sequentially without dedicated space for the Context Window summary, making it hard to track token usage at a glance. With this change, the footer will remain visible at the bottom of the terminal, continuously updating to show token consumption progress, while events scroll above it.

To see it working: run `agtrace watch` in a terminal, observe events streaming in the main area, and see the Context Window bar persistently displayed at the bottom, updating whenever a TokenUsage event arrives.


## Progress

- [x] (2025-01-20) Add crossterm 0.28 dependency to agtrace-cli/Cargo.toml
- [x] (2025-01-20) Create TUI module skeleton in crates/agtrace-cli/src/ui/tui.rs with TuiWatchView struct
- [x] (2025-01-20) Add tui module to ui/mod.rs exports
- [x] (2025-01-20) Verify compilation succeeds with stub implementation (Milestone 1 complete)
- [x] (2025-01-20) Add interior mutability (Mutex) for WatchView trait compatibility
- [x] (2025-01-20) Implement render() method with fixed footer logic
- [x] (2025-01-20) Implement render_stream_update() to populate buffers and update footer
- [x] (2025-01-20) Verify compilation and all tests pass (Milestone 2 complete)
- [ ] Integrate TUI mode into watch handler with TTY detection
- [ ] Add ctrlc dependency for signal handling
- [ ] Implement cleanup on Ctrl+C
- [ ] Test TUI and non-TUI modes (Milestone 3)
- [ ] Run full test suite
- [ ] Update documentation


## Surprises & Discoveries

- Observation: Crossterm 0.28.1 was installed (latest 0.29.0 available but 0.28 series is stable)
  Evidence: Cargo output showed "Adding crossterm v0.28.1 (available: v0.29.0)"

- Observation: Interior mutability pattern worked seamlessly with WatchView trait
  Evidence: Using Mutex<TuiWatchViewInner> allowed render_stream_update(&self) to mutate state while maintaining trait compatibility. All tests pass without modification.


## Decision Log

(To be filled during implementation)


## Outcomes & Retrospective

(To be filled at completion)


## Context and Orientation

The agtrace CLI tool is a Rust workspace located at `/Users/zawakin/go/src/github.com/lanegrid/agtrace`. The watch command is implemented in `crates/agtrace-cli/src/handlers/watch.rs` and uses a view trait (`WatchView`) defined in `crates/agtrace-cli/src/ui/traits.rs`. The current implementation uses `ConsoleTraceView` (in `crates/agtrace-cli/src/ui/console.rs`) which prints events to stdout sequentially.

There is already infrastructure for refreshing displays in `crates/agtrace-cli/src/ui/refresh.rs`, including:
- `WatchBuffer`: stores events and session state
- `RefreshingWatchView`: a view implementation that uses basic ANSI escape codes
- `TerminalWriter` trait: abstraction for terminal output

The Context Window summary is rendered by `format_token_summary()` in `crates/agtrace-cli/src/views/session/compact.rs`, which returns a `Vec<String>` containing formatted lines showing token usage progress bars and percentages.

Token usage events arrive via the `render_stream_update()` method in the WatchView trait. When a `TokenUsage` event is encountered, the current implementation calls `format_token_summary()` and prints the result to stdout.

The goal is to enhance the watch command to use a TUI (terminal user interface) mode where:
1. The main area scrolls upward as new events arrive (like `tail -f`)
2. The bottom N lines are reserved for a fixed footer showing Context Window usage
3. The footer updates in-place without scrolling

TUI stands for "text-based user interface" — it means manipulating the terminal cursor and screen directly (similar to editors like vim or tools like htop) rather than just appending lines to stdout.


## Plan of Work

We will implement this in three milestones:

**Milestone 1: Add crossterm dependency and create TUI infrastructure**

Add the crossterm crate (version 0.28) to `crates/agtrace-cli/Cargo.toml`. Crossterm is a cross-platform library for terminal manipulation that allows us to move the cursor, clear specific areas, and enter "raw mode" (where the terminal doesn't automatically scroll or echo input).

Create a new module `crates/agtrace-cli/src/ui/tui.rs` containing:
- `TuiWatchView`: a new implementation of the `WatchView` trait that uses crossterm
- Helper functions to calculate terminal dimensions using the existing `terminal_size` crate (already a dependency)
- Logic to divide the screen into a scrollable content area and a fixed footer area

**Milestone 2: Implement fixed footer rendering**

Modify `TuiWatchView` to maintain an in-memory buffer of recent events and re-render the screen on each update:
1. On `render_stream_update()`, append new events to the buffer
2. Calculate how many lines are available for event display (terminal height minus footer height)
3. Take the last N events that fit in the content area
4. Render events to the content area using `format_event_with_start()` (already pure functions)
5. Render the footer using `format_token_summary()`
6. Position the cursor at the start of the footer area and write footer lines

**Milestone 3: Integrate TUI mode into watch command**

Modify `crates/agtrace-cli/src/handlers/watch.rs` to:
1. Detect if stdout is a TTY using the `is-terminal` crate (already a dependency)
2. If TTY, use `TuiWatchView`; otherwise, use `ConsoleTraceView`
3. Optionally add a `--tui` / `--no-tui` flag to override auto-detection

Add signal handling to restore terminal state on Ctrl+C. Crossterm provides utilities for this.


## Concrete Steps

**Milestone 1: Add crossterm and create TUI module**

1. Edit `crates/agtrace-cli/Cargo.toml` and add crossterm to the dependencies section:

       [dependencies]
       # ... existing dependencies ...
       crossterm = "0.28"

2. Run `cargo build` from the repository root to download and compile crossterm. Expected output: compilation succeeds with no errors.

3. Create a new file `crates/agtrace-cli/src/ui/tui.rs` with this skeleton:

       use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
       use crate::token_limits::TokenLimits;
       use crate::ui::traits::WatchView;
       use crate::views::session::{format_event_with_start, format_token_summary};
       use agtrace_runtime::reactor::{Reaction, SessionState};
       use agtrace_types::{AgentEvent, EventPayload};
       use anyhow::Result;
       use crossterm::{
           cursor, execute, queue, terminal,
           terminal::{EnterAlternateScreen, LeaveAlternateScreen},
       };
       use std::collections::VecDeque;
       use std::io::{self, Write};
       use std::path::Path;

       pub struct TuiWatchView {
           events_buffer: VecDeque<String>,
           footer_lines: Vec<String>,
           session_start_time: Option<chrono::DateTime<chrono::Utc>>,
           turn_count: usize,
           project_root: Option<std::path::PathBuf>,
       }

       impl TuiWatchView {
           pub fn new() -> Result<Self> {
               // Enter alternate screen so we don't mess up the user's shell history
               execute!(io::stdout(), EnterAlternateScreen)?;
               terminal::enable_raw_mode()?;

               Ok(Self {
                   events_buffer: VecDeque::new(),
                   footer_lines: Vec::new(),
                   session_start_time: None,
                   turn_count: 0,
                   project_root: None,
               })
           }

           fn render(&mut self) -> Result<()> {
               // To be implemented in Milestone 2
               Ok(())
           }
       }

       impl Drop for TuiWatchView {
           fn drop(&mut self) {
               // Restore terminal state when view is dropped
               let _ = terminal::disable_raw_mode();
               let _ = execute!(io::stdout(), LeaveAlternateScreen);
           }
       }

       impl WatchView for TuiWatchView {
           // Minimal stubs for now - to be implemented in Milestone 2
           fn render_watch_start(&self, _start: &crate::ui::models::WatchStart) -> Result<()> {
               Ok(())
           }

           fn on_watch_attached(&self, _display_name: &str) -> Result<()> {
               Ok(())
           }

           fn on_watch_initial_summary(&self, _summary: &crate::ui::models::WatchSummary) -> Result<()> {
               Ok(())
           }

           fn on_watch_rotated(&self, _old_path: &Path, _new_path: &Path) -> Result<()> {
               Ok(())
           }

           fn on_watch_waiting(&self, _message: &str) -> Result<()> {
               Ok(())
           }

           fn on_watch_error(&self, _message: &str, _fatal: bool) -> Result<()> {
               Ok(())
           }

           fn on_watch_orphaned(&self, _orphaned: usize, _total_events: usize) -> Result<()> {
               Ok(())
           }

           fn on_watch_token_warning(&self, _warning: &str) -> Result<()> {
               Ok(())
           }

           fn on_watch_reactor_error(&self, _reactor_name: &str, _error: &str) -> Result<()> {
               Ok(())
           }

           fn on_watch_reaction_error(&self, _error: &str) -> Result<()> {
               Ok(())
           }

           fn on_watch_reaction(&self, _reaction: &Reaction) -> Result<()> {
               Ok(())
           }

           fn render_stream_update(&self, _state: &SessionState, _new_events: &[AgentEvent]) -> Result<()> {
               Ok(())
           }
       }

4. Edit `crates/agtrace-cli/src/ui/mod.rs` and add the tui module:

       pub mod console;
       pub mod models;
       pub mod refresh;
       pub mod traits;
       pub mod tui;

5. Run `cargo build` from the repository root. Expected output: compilation succeeds. At this point, TuiWatchView exists but does nothing.

**Acceptance for Milestone 1:**
Running `cargo build` succeeds. The TuiWatchView type exists and implements WatchView. Running the existing tests with `cargo test` should still pass because we haven't changed any existing behavior yet.

**Milestone 2: Implement fixed footer rendering**

1. Edit `crates/agtrace-cli/src/ui/tui.rs` and implement the `render()` method:

       fn render(&mut self) -> Result<()> {
           let (term_width, term_height) = terminal::size()?;
           let term_height = term_height as usize;

           // Reserve bottom lines for footer
           let footer_height = self.footer_lines.len().max(1);
           let content_height = term_height.saturating_sub(footer_height + 1); // +1 for separator

           // Clear screen and move cursor to top
           execute!(
               io::stdout(),
               terminal::Clear(terminal::ClearType::All),
               cursor::MoveTo(0, 0)
           )?;

           // Render content area (recent events)
           let start_idx = self.events_buffer.len().saturating_sub(content_height);
           for (i, line) in self.events_buffer.iter().skip(start_idx).enumerate() {
               queue!(
                   io::stdout(),
                   cursor::MoveTo(0, i as u16),
                   terminal::Clear(terminal::ClearType::CurrentLine)
               )?;
               print!("{}", line);
           }

           // Render separator line
           let separator_row = content_height as u16;
           queue!(
               io::stdout(),
               cursor::MoveTo(0, separator_row),
               terminal::Clear(terminal::ClearType::CurrentLine)
           )?;
           println!("{}", "─".repeat(term_width as usize));

           // Render footer
           for (i, line) in self.footer_lines.iter().enumerate() {
               let row = (separator_row + 1 + i as u16).min(term_height as u16 - 1);
               queue!(
                   io::stdout(),
                   cursor::MoveTo(0, row),
                   terminal::Clear(terminal::ClearType::CurrentLine)
               )?;
               print!("{}", line);
           }

           io::stdout().flush()?;
           Ok(())
       }

2. Implement `render_stream_update()` to populate buffers and call render():

       fn render_stream_update(&mut self, state: &SessionState, new_events: &[AgentEvent]) -> Result<()> {
           // Update tracking state
           if self.session_start_time.is_none() {
               self.session_start_time = Some(state.start_time);
           }
           self.turn_count = state.turn_count;
           self.project_root = state.project_root.clone();

           // Format and buffer new events
           for event in new_events {
               if let Some(line) = format_event_with_start(
                   event,
                   self.turn_count,
                   self.project_root.as_deref(),
                   self.session_start_time,
               ) {
                   self.events_buffer.push_back(line);

                   // Keep buffer size manageable (last 1000 events)
                   if self.events_buffer.len() > 1000 {
                       self.events_buffer.pop_front();
                   }
               }

               // Update footer on TokenUsage events
               if matches!(event.payload, EventPayload::TokenUsage(_)) {
                   let token_limits = TokenLimits::new();
                   let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));

                   let limit = state
                       .context_window_limit
                       .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

                   let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

                   let summary = TokenSummaryDisplay {
                       input: state.total_input_side_tokens(),
                       output: state.total_output_side_tokens(),
                       cache_creation: state.current_usage.cache_creation.0,
                       cache_read: state.current_usage.cache_read.0,
                       total: state.total_context_window_tokens(),
                       limit,
                       model: state.model.clone(),
                       compaction_buffer_pct,
                   };

                   let opts = DisplayOptions {
                       enable_color: true,
                       relative_time: false,
                       truncate_text: None,
                   };

                   self.footer_lines = format_token_summary(&summary, &opts);
               }
           }

           self.render()?;
           Ok(())
       }

   Note: `render_stream_update()` takes `&mut self` but the WatchView trait declares it as `&self`. We will need to use interior mutability. Change the struct to:

       pub struct TuiWatchView {
           inner: std::sync::Mutex<TuiWatchViewInner>,
       }

       struct TuiWatchViewInner {
           events_buffer: VecDeque<String>,
           footer_lines: Vec<String>,
           session_start_time: Option<chrono::DateTime<chrono::Utc>>,
           turn_count: usize,
           project_root: Option<std::path::PathBuf>,
       }

   And update the implementation accordingly. Use `self.inner.lock().unwrap()` to access the inner state.

3. Run `cargo build` to verify compilation. Expected output: build succeeds.

4. Manually test by running `agtrace watch` (after integrating in Milestone 3) and observing the fixed footer.

**Acceptance for Milestone 2:**
The TuiWatchView type can render events in a scrollable area and display a fixed footer. The footer updates when TokenUsage events arrive. The display remains stable (no flickering or cursor jumping).

**Milestone 3: Integrate TUI mode into watch command**

1. Edit `crates/agtrace-cli/src/handlers/watch.rs`. Import the TUI module:

       use crate::ui::tui::TuiWatchView;
       use is_terminal::IsTerminal;

2. Modify the `handle()` function to detect TTY and choose the appropriate view:

       pub fn handle(ctx: &ExecutionContext, target: WatchTarget, view: &dyn WatchView) -> Result<()> {
           // ... existing provider resolution logic ...

           // Auto-select TUI mode if stdout is a TTY
           let use_tui = std::io::stdout().is_terminal();

           let view_box: Box<dyn WatchView> = if use_tui {
               Box::new(TuiWatchView::new()?)
           } else {
               // Use the passed-in view (ConsoleTraceView)
               // This requires changing the function signature - see below
               Box::new(ConsoleTraceView::new())
           };

           // ... rest of the function uses &*view_box instead of view ...
       }

   Actually, the current signature is `handle(ctx, target, view: &dyn WatchView)`, which means the caller chooses the view. We should change this to let `handle()` choose the view internally. Update the signature to:

       pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()>

   And remove the `view` parameter from all call sites in `crates/agtrace-cli/src/commands.rs`.

3. Edit `crates/agtrace-cli/src/commands.rs`. Find the `watch` command handling code (likely in a match arm for `Commands::Session(SessionCommand::Watch { ... })`) and remove the view parameter:

       // Before:
       // let view = ConsoleTraceView::new();
       // handlers::watch::handle(&ctx, target, &view)?;

       // After:
       handlers::watch::handle(&ctx, target)?;

4. Add signal handling for graceful cleanup. In `crates/agtrace-cli/src/ui/tui.rs`, add:

       impl TuiWatchView {
           pub fn new() -> Result<Self> {
               // Enter alternate screen
               execute!(io::stdout(), EnterAlternateScreen)?;
               terminal::enable_raw_mode()?;

               // Set up Ctrl+C handler
               ctrlc::set_handler(move || {
                   let _ = terminal::disable_raw_mode();
                   let _ = execute!(io::stdout(), LeaveAlternateScreen);
                   std::process::exit(0);
               })?;

               Ok(Self {
                   inner: std::sync::Mutex::new(TuiWatchViewInner {
                       events_buffer: VecDeque::new(),
                       footer_lines: Vec::new(),
                       session_start_time: None,
                       turn_count: 0,
                       project_root: None,
                   }),
               })
           }
       }

   Note: `ctrlc` is not a dependency yet. Add it to `Cargo.toml`:

       [dependencies]
       ctrlc = "0.8"

5. Run `cargo build` to verify compilation. Expected output: build succeeds.

6. Test manually:
   - Run `agtrace watch` in a terminal. Expected: TUI mode activates, events scroll, footer is fixed.
   - Run `agtrace watch | cat`. Expected: non-TUI mode (because stdout is not a TTY), output prints normally.
   - Press Ctrl+C during `agtrace watch`. Expected: terminal restores cleanly, no leftover escape codes.

**Acceptance for Milestone 3:**
Running `agtrace watch` in a terminal shows the TUI with a fixed footer. Running `agtrace watch | cat` uses non-TUI mode. Ctrl+C cleanly exits and restores the terminal. All existing tests pass (`cargo test`).


## Validation and Acceptance

After completing all milestones:

1. Run `cargo test` from the repository root. Expected: all tests pass.

2. Run `cargo build --release` to build an optimized binary. Expected: compilation succeeds.

3. Start a watch session:

       ./target/release/agtrace watch

   Expected output: TUI mode with scrolling events and a fixed footer showing Context Window usage.

4. Verify the footer updates correctly:
   - Wait for a TokenUsage event to arrive.
   - Observe the footer changing to reflect new token counts and percentages.

5. Verify terminal restoration:
   - Press Ctrl+C.
   - Observe that the terminal returns to normal mode without leftover UI artifacts.

6. Verify non-TUI fallback:

       ./target/release/agtrace watch | tee watch.log

   Expected: events print to stdout without TUI escape codes. The file `watch.log` should contain plain text.

7. Compare TUI and non-TUI output formats:
   - Run `agtrace watch` (TUI mode) and observe the event format.
   - Run `agtrace watch | cat` (non-TUI mode) and observe the event format.
   - Both should display events using the same formatting (Timeline style with relative timestamps).

Acceptance criteria: Users can run `agtrace watch` and see a stable TUI with a fixed footer. The footer updates in place. Ctrl+C restores the terminal. Non-TTY output works correctly. All tests pass.


## Idempotence and Recovery

All steps can be repeated safely. If a build fails midway, you can re-run `cargo build` after fixing errors. The code changes are purely additive: we add a new module (`tui.rs`) and modify the watch handler to auto-select the view. Existing functionality (ConsoleTraceView) remains unchanged and can be used as a fallback.

If the terminal gets into a bad state during manual testing (e.g., raw mode stuck on), run:

    reset

This command (available on Unix systems) will restore the terminal to a clean state.


## Artifacts and Notes

Expected Cargo.toml diff:

    [dependencies]
    # ... existing dependencies ...
    +crossterm = "0.28"
    +ctrlc = "0.8"

Expected directory structure after completion:

    crates/agtrace-cli/src/ui/
    ├── console.rs       (existing, unchanged)
    ├── models.rs        (existing, unchanged)
    ├── mod.rs           (modified to add `pub mod tui;`)
    ├── refresh.rs       (existing, unchanged)
    ├── traits.rs        (existing, unchanged)
    └── tui.rs           (new file)


## Interfaces and Dependencies

**External crates:**
- `crossterm` version 0.28: for terminal manipulation (cursor movement, screen clearing, raw mode)
- `ctrlc` version 0.8: for signal handling (Ctrl+C cleanup)
- `is-terminal` (already a dependency): to detect if stdout is a TTY
- `terminal_size` (already a dependency): to get terminal dimensions (though crossterm also provides this)

**Internal modules:**
- `crate::ui::traits::WatchView`: trait that TuiWatchView will implement
- `crate::views::session::format_event_with_start`: pure function to format a single event with relative time
- `crate::views::session::format_token_summary`: pure function to format the Context Window summary
- `crate::display_model::{DisplayOptions, TokenSummaryDisplay}`: data types for rendering configuration
- `crate::token_limits::TokenLimits`: to look up token limits by model name

**Key types and signatures:**

In `crates/agtrace-cli/src/ui/tui.rs`:

    pub struct TuiWatchView {
        inner: std::sync::Mutex<TuiWatchViewInner>,
    }

    struct TuiWatchViewInner {
        events_buffer: VecDeque<String>,
        footer_lines: Vec<String>,
        session_start_time: Option<chrono::DateTime<chrono::Utc>>,
        turn_count: usize,
        project_root: Option<std::path::PathBuf>,
    }

    impl TuiWatchView {
        pub fn new() -> Result<Self>;
    }

    impl WatchView for TuiWatchView {
        fn render_stream_update(&self, state: &SessionState, new_events: &[AgentEvent]) -> Result<()>;
        // ... other methods from WatchView trait ...
    }

In `crates/agtrace-cli/src/handlers/watch.rs`:

    pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()>

(Note: signature changes from `handle(..., view: &dyn WatchView)` to `handle(...)` because the handler now chooses the view internally.)
