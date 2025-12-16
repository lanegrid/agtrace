# Event-Driven Reactor Architecture Implementation

## Goal
Transform `watch` from passive log viewer to active circuit breaker with pluggable reactors.

## Architecture

```
Event Source (SessionWatcher)
    ↓
Main Loop (Coordinator)
    ↓
Reactors (TuiRenderer, StallDetector, SafetyGuard)
    ↓
Reactions (Continue, Warn, Intervene)
```

## Implementation Plan

### Phase 1: Core Reactor Infrastructure (v0.1.0 foundation) ✅

- [x] Create `crates/agtrace-cli/src/reactor.rs` with core traits:
  - `Reactor` trait with `handle(&mut self, ctx: ReactorContext) -> Result<Reaction>`
  - `Reaction` enum: `Continue`, `Warn(String)`, `Intervene { reason, severity }`
  - `ReactorContext` struct: contains `&AgentEvent` + `&SessionState` (Copy-able)
  - `SessionState` struct: lightweight metadata (tokens, error_count, start_time, etc.)

- [x] Extract TuiRenderer reactor:
  - Move display logic from `handlers/watch.rs` to `reactors/tui_renderer.rs`
  - Implement `Reactor` trait
  - Keep existing formatting (icons, colors, truncation)

- [x] Refactor `handlers/watch.rs` main loop:
  - Initialize `SessionState` on first event
  - Register reactors: `Vec<Box<dyn Reactor>>`
  - Update state on each event (`update_session_state`)
  - Call `reactor.handle(ctx)` for each registered reactor
  - Handle `Reaction` responses (`handle_reaction`)

### Phase 2: Monitoring Reactors (v0.1.0 complete) ✅

- [x] Implement `reactors/stall_detector.rs`:
  - Track `last_activity` timestamp
  - Emit `Intervene { severity: Notification }` after 60s idle threshold
  - 5-minute cooldown between notifications

- [x] Implement `reactors/safety_guard.rs`:
  - Check tool arguments for dangerous patterns:
    - Path traversal (`..`)
    - Absolute paths outside user directories
    - System directory access (`/`, `/etc/`, `/sys/`)
  - Emit `Intervene { severity: Notification }` for v0.1.0 (monitoring only)

### Phase 3: Testing & Polish ✅

- [x] Add tests for reactor isolation
  - Unit tests for `Reactor` trait (5 tests in `reactor.rs`)
  - Unit tests for `SafetyGuard` (7 tests in `safety_guard.rs`)
  - All tests passing (16 total in lib)
- [x] Document reactor interface
  - Created `docs/reactor_architecture.md` with full documentation
  - Includes examples, design principles, and future extensions
- [ ] Add CLI flag: `--disable-reactor <name>` for debugging (deferred to next iteration)

## Future (v0.2.0): Intervention Mode

- [ ] Add `agtrace run -- <command>` to spawn child process
- [ ] Upgrade `Intervene { severity: Kill }` to terminate child
- [ ] Add pattern detection (loop detection, token budget)
- [ ] Add AI-powered anomaly detection reactor

## Implementation Summary (Completed)

### What Was Built

**Phase 1: Core Infrastructure**
- Reactor trait system with `Reaction` enum
- `SessionState` for lightweight session metadata
- `ReactorContext` (Copy-able) for passing event + state
- Main loop refactoring with reactor coordination

**Phase 2: Monitoring Reactors**
- `TuiRenderer`: Display logic (extracted from watch.rs)
- `StallDetector`: Idle detection with cooldown
- `SafetyGuard`: Path traversal and system directory detection

**Phase 3: Testing & Documentation**
- 12 unit tests (5 for core, 7 for SafetyGuard)
- All tests passing (16 total including config tests)
- Comprehensive documentation in `docs/reactor_architecture.md`

### Metrics

- **Code Quality**: 0 errors, 1 warning (unused field)
- **Line Count**: 563 insertions, 152 deletions (net +411 lines)
- **Commits**: 2 commits with atomic changes
- **Test Coverage**: Core reactor logic fully tested

### Key Design Decisions

1. **Context Pattern**: Event + State separation for smart decisions
2. **Copy-able Context**: Enables multiple reactors without borrowing issues
3. **Check Ordering**: System paths before general absolute paths (SafetyGuard)
4. **v0.1.0 Safety**: All interventions are Notification-level (no kills yet)

## Notes

- Keep v0.1.0 focused on monitoring (no kills, only warnings)
- Reactor interface supports sync operations (async deferred to v0.2.0)
- SessionState is lightweight summary, not full AgentSession
- `--disable-reactor` CLI flag deferred to next iteration (not critical for MVP)
