# Simplify Watch Functionality: Focus on Core Monitoring

This ExecPlan is a living document maintained according to `.agent/PLANS.md`. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.


## Purpose / Big Picture

After this change, `agtrace watch` will remain fully functional for real-time monitoring of agent sessions while reducing code complexity by ~40% (from 2500 lines to ~1500 lines). The watch command will still provide:

1. Real-time context window monitoring with token usage display
2. Live event stream showing agent actions (tool calls, messages, reasoning)
3. Session rotation detection and automatic reattachment
4. Token limit warnings

Users verify success by running `agtrace watch` and observing the same clean, updating display they see today, but the implementation will be dramatically simpler and easier to maintain.


## Progress

- [x] (2025-12-20) Phase 1: Renamed StreamEvent → WatchEvent (mechanical refactor)
  - Updated: watcher.rs, runtime.rs, streaming/mod.rs, lib.rs, watch_test.rs
  - All tests pass (cargo test)
  - RuntimeEvent still exists (will be removed in Phase 3)
- [ ] Phase 2: Merge UI rendering layers (refresh.rs + console.rs → watch_ui.rs)
- [ ] Phase 3: Inline reactor logic into runtime
- [ ] Phase 4: Clean up unused abstractions
- [ ] Phase 5: Validate all watch modes work correctly


## Surprises & Discoveries

- Phase 1 was simpler than expected: Just a mechanical rename across 5 files. All tests passed immediately after the rename.


## Decision Log

- Decision: Preserve existing watch UX completely while refactoring internals
  Rationale: Users rely on current display format; simplification is internal quality improvement
  Date: 2025-12-20

- Decision: Keep reactor functionality but remove trait abstraction
  Rationale: Three specific reactors don't justify trait-based plugin architecture; inline functions are clearer
  Date: 2025-12-20

- Decision: Maintain separation between agtrace-runtime and agtrace-cli
  Rationale: Runtime crate is reusable; only simplify the wiring, not the crate boundary
  Date: 2025-12-20


## Outcomes & Retrospective

(To be filled at completion)


## Context and Orientation

The `agtrace watch` command provides real-time monitoring of agent sessions. It watches log files written by AI providers (Claude, Gemini, Codex), assembles them into structured sessions, and displays a continuously updating view showing:

- Token usage (input, output, cache creation, cache read)
- Event stream (user messages, tool calls, reasoning)
- Session metadata (project, model, turn count)

**Current architecture (2500+ lines across 6 files):**

1. **File watching layer** (`agtrace-runtime/src/streaming/watcher.rs`, 489 lines)
   - Uses `notify` crate to watch filesystem
   - Detects new/modified log files
   - Emits `StreamEvent` enum

2. **Runtime orchestration layer** (`agtrace-runtime/src/runtime.rs`, 326 lines)
   - Receives `StreamEvent` from watcher
   - Converts to `RuntimeEvent`
   - Runs reactor trait objects
   - Sends `RuntimeEvent` to CLI

3. **Reactor system** (660 lines total)
   - `agtrace-runtime/src/reactor.rs` (259 lines): Trait definition, SessionState
   - `agtrace-cli/src/reactors/token_usage_monitor.rs` (303 lines)
   - `agtrace-cli/src/reactors/safety_guard.rs` (284 lines)
   - `agtrace-cli/src/reactors/stall_detector.rs` (72 lines)

4. **UI rendering layer** (1328 lines total)
   - `agtrace-cli/src/ui/refresh.rs` (653 lines): WatchBuffer, formatting, RefreshingWatchView
   - `agtrace-cli/src/ui/console.rs` (675 lines): Console utilities, mixed with other commands

5. **Handler entry point** (`agtrace-cli/src/handlers/watch.rs`, 104 lines)

**Key complexity issues:**

- Three event types: `StreamEvent` → `RuntimeEvent` → UI rendering (unnecessary conversions)
- Two large UI files with overlapping responsibilities
- Reactor trait abstraction for exactly 3 implementations that never change
- SessionState duplicated across layers


## Plan of Work

**Phase 1: Unify Event Types**

Current flow has unnecessary conversion:
- Watcher emits `StreamEvent`
- Runtime converts to `RuntimeEvent`
- CLI receives `RuntimeEvent`

Simplify to:
- Watcher emits `WatchEvent`
- Runtime forwards `WatchEvent` directly
- CLI receives `WatchEvent`

Changes:
1. In `agtrace-runtime/src/streaming/watcher.rs`:
   - Rename `StreamEvent` to `WatchEvent`
   - Add reactor-relevant fields directly to variants (e.g., `StateUpdated` includes token warnings)

2. In `agtrace-runtime/src/runtime.rs`:
   - Remove `RuntimeEvent` enum entirely
   - Change `Runtime::receiver()` to return `Receiver<WatchEvent>`
   - Inline reactor logic directly in event handling loop
   - Keep reactor functions (token monitoring, stall detection, safety checks) but remove trait

3. In `agtrace-cli/src/handlers/watch.rs`:
   - Change event loop to receive `WatchEvent` instead of `RuntimeEvent`
   - Map variants directly to view calls

**Phase 2: Merge UI Layers**

Current state:
- `ui/refresh.rs`: WatchBuffer, event formatting, RefreshingWatchView trait impl
- `ui/console.rs`: Console helpers, but also used by other commands

Create new `ui/watch_ui.rs` (~400 lines):
1. Move watch-specific code from `refresh.rs` and `console.rs`
2. Create simple `WatchDisplay` struct that owns:
   - Event buffer (VecDeque<AgentEvent>)
   - Current SessionState
   - Terminal writer
3. Implement single `render()` method that:
   - Formats header (session info)
   - Formats events (tool calls, messages, etc.)
   - Formats footer (token summary)
4. Keep `ui/console.rs` for shared utilities (color formatting, etc.)
5. Delete `ui/refresh.rs` entirely

**Phase 3: Inline Reactor Logic**

Remove trait abstraction:
1. Delete `agtrace-runtime/src/reactor.rs`
2. Delete `agtrace-cli/src/reactors/` directory
3. In `agtrace-runtime/src/runtime.rs`, add three inline functions:
   ```rust
   fn check_token_usage(state: &SessionState) -> Option<String>
   fn check_stall(state: &SessionState, threshold_secs: u64) -> Option<String>
   fn check_safety(event: &AgentEvent, state: &SessionState) -> Option<String>
   ```
4. Call these functions directly in the event loop
5. Include warnings in `WatchEvent::StateUpdated` variant

**Phase 4: Clean Up**

1. Remove unused imports
2. Update `ui/mod.rs` to export new `watch_ui` module
3. Update `ui/traits.rs` if WatchView trait can be simplified
4. Run `cargo fmt` and `cargo clippy`

**Phase 5: Validation**

Test all watch modes:
1. `agtrace watch` (auto-detect active session)
2. `agtrace watch --session <id>` (explicit session)
3. Session rotation (start new session while watching)
4. Token warnings trigger correctly
5. Error handling (missing files, corrupt logs)


## Concrete Steps

All commands run from repository root `/Users/zawakin/go/src/github.com/lanegrid/agtrace`.

**Before starting:**
```bash
cargo test
cargo build
```

Ensure all tests pass. This gives us a baseline.

**Phase 1: Unify event types**

1. Rename `StreamEvent` to `WatchEvent`:
   ```bash
   # In crates/agtrace-runtime/src/streaming/watcher.rs
   # Change: pub enum StreamEvent → pub enum WatchEvent
   ```

2. Update `runtime.rs` to use `WatchEvent`:
   - Remove `RuntimeEvent` enum definition
   - Change `Runtime::receiver()` return type
   - Update event loop to forward `WatchEvent` directly

3. Update CLI handler:
   ```bash
   # In crates/agtrace-cli/src/handlers/watch.rs
   # Change event matching to use WatchEvent variants
   ```

4. Verify compilation:
   ```bash
   cargo build
   ```

Expected: Compilation errors will guide remaining renames. Fix systematically.

**Phase 2: Merge UI layers**

1. Create new file `crates/agtrace-cli/src/ui/watch_ui.rs`
2. Move watch-specific code from `refresh.rs`
3. Simplify to single `WatchDisplay` struct
4. Update `handlers/watch.rs` to use new `WatchDisplay`
5. Delete `ui/refresh.rs`
6. Update `ui/mod.rs`

Verify:
```bash
cargo build
cargo test
./target/debug/agtrace watch
```

Expected: Display looks identical to before.

**Phase 3: Inline reactors**

1. Copy reactor logic from `reactors/*.rs` into `runtime.rs` as functions
2. Delete `agtrace-cli/src/reactors/` directory
3. Delete `agtrace-runtime/src/reactor.rs`
4. Update imports

Verify:
```bash
cargo build
cargo test
```

**Phase 4: Clean up**

```bash
cargo fmt
cargo clippy --all-targets
```

Fix any warnings.

**Phase 5: Validation**

Test each mode:
```bash
# Auto-detect mode
./target/debug/agtrace watch

# Explicit session
./target/debug/agtrace watch --session <some-id>

# Verify in separate terminal: start new Claude session
# Observe rotation message
```

Run full test suite:
```bash
cargo test
```


## Validation and Acceptance

**Acceptance criteria:**

1. All existing tests pass: `cargo test` shows same pass rate as before
2. Watch displays identical output: User sees same event stream, token usage, formatting
3. All watch modes work: auto-detect, explicit session, rotation
4. Line count reduced: From ~2500 to ~1500 lines (40% reduction)
5. No new clippy warnings: `cargo clippy --all-targets` clean

**Manual verification:**

Start watch in terminal:
```bash
./target/debug/agtrace watch
```

Expected output (example):
```
[👀 Watching] /Users/zawakin/.codex/sessions (codex)
✨ Attached to active session: abc123def

📁 Project: /Users/zawakin/go/src/github.com/lanegrid/agtrace
🔖 Hash: a1b2c3d4

14:23:45 👤 User: "help me refactor the watch code"
14:23:47 🧠 Thnk: analyzing current architecture
14:23:50 📖 Read: ("src/handlers/watch.rs")
14:23:52 💬 Msg: I'll help you simplify the watch functionality

⛁ Context Window
Input:  12.3K  (fresh: 12.3K)
Output:  3.2K
Cache:   0 create, 0 read
Total:  15.5K / 200K  (7.8%)
```

In separate terminal, start new session. Watch should show:
```
✨ Session rotated: abc123def → xyz789fed
```


## Idempotence and Recovery

Each phase can be run multiple times safely:

- Phase 1 (rename): Idempotent renames
- Phase 2 (merge UI): Create new file, delete old only after verification
- Phase 3 (inline reactors): Copy before delete
- Phase 4 (cleanup): Formatting is idempotent
- Phase 5 (validation): Read-only testing

**If something breaks:**

1. Check compilation: `cargo build`
2. Check tests: `cargo test`
3. If tests fail, revert specific phase: `git checkout HEAD -- <files>`
4. Review diff: `git diff`

**Recovery strategy:**

- Each phase should be committed separately
- Keep watch_test.rs passing throughout
- If runtime changes break CLI, fix in same commit


## Artifacts and Notes

**Current line counts (baseline):**
```
   489 crates/agtrace-runtime/src/streaming/watcher.rs
   326 crates/agtrace-runtime/src/runtime.rs
   259 crates/agtrace-runtime/src/reactor.rs
   653 crates/agtrace-cli/src/ui/refresh.rs
   675 crates/agtrace-cli/src/ui/console.rs (partially watch-related)
   303 crates/agtrace-cli/src/reactors/token_usage_monitor.rs
   284 crates/agtrace-cli/src/reactors/safety_guard.rs
    72 crates/agtrace-cli/src/reactors/stall_detector.rs
   104 crates/agtrace-cli/src/handlers/watch.rs
  ----
  3165 total (watch-related code)
```

**Target line counts:**
```
   450 crates/agtrace-runtime/src/streaming/watcher.rs (minimal change)
   400 crates/agtrace-runtime/src/runtime.rs (added inline reactor functions)
   400 crates/agtrace-cli/src/ui/watch_ui.rs (merged from refresh.rs)
   100 crates/agtrace-cli/src/handlers/watch.rs (simplified)
  ----
  1350 total (57% reduction)
```


## Interfaces and Dependencies

**After Phase 1: Unified WatchEvent**

In `crates/agtrace-runtime/src/streaming/watcher.rs`:
```rust
#[derive(Debug, Clone)]
pub enum WatchEvent {
    Attached {
        path: PathBuf,
        session_id: Option<String>,
    },
    Update {
        state: SessionState,
        new_events: Vec<AgentEvent>,
        warnings: Vec<String>, // From inlined reactors
    },
    SessionRotated {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    Waiting {
        message: String,
    },
    Error(String),
}
```

In `crates/agtrace-runtime/src/runtime.rs`:
```rust
pub struct Runtime {
    rx: Receiver<WatchEvent>,
    _handle: JoinHandle<()>,
}

impl Runtime {
    pub fn start(config: RuntimeConfig) -> Result<Self>;
    pub fn receiver(&self) -> &Receiver<WatchEvent>;
}
```

**After Phase 2: Simplified UI**

In `crates/agtrace-cli/src/ui/watch_ui.rs`:
```rust
pub struct WatchDisplay {
    events: VecDeque<AgentEvent>,
    state: SessionState,
    terminal: Box<dyn TerminalWriter>,
}

impl WatchDisplay {
    pub fn new(terminal: Box<dyn TerminalWriter>) -> Self;
    pub fn push_event(&mut self, event: AgentEvent);
    pub fn update_state(&mut self, state: SessionState);
    pub fn render(&mut self);
}
```

**After Phase 3: Inline Reactors**

In `crates/agtrace-runtime/src/runtime.rs`:
```rust
// Inline reactor functions (private)
fn check_token_usage(state: &SessionState, limits: &TokenLimits) -> Option<String>;
fn check_stall(state: &SessionState, threshold_secs: u64) -> Option<String>;
fn check_safety(event: &AgentEvent, state: &SessionState) -> Option<String>;
```

No public reactor trait. These are implementation details of runtime.


## Migration Strategy

This is a refactoring with no user-visible changes. Migration strategy:

1. **Backward compatibility:** Not applicable (internal refactor)
2. **Testing:** Existing `watch_test.rs` must continue passing
3. **Rollback:** If issues found post-merge, revert entire PR as one unit
4. **Documentation:** Update internal architecture docs if any exist

**Risk mitigation:**

- Keep tests passing after each phase
- Commit each phase separately for easy bisection
- Manual testing of all watch modes before final commit
- Code review focusing on event flow correctness
