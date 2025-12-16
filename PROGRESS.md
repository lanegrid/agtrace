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

### Phase 3: Testing & Polish

- [ ] Add tests for reactor isolation
- [ ] Document reactor interface
- [ ] Add CLI flag: `--disable-reactor <name>` for debugging

## Future (v0.2.0): Intervention Mode

- [ ] Add `agtrace run -- <command>` to spawn child process
- [ ] Upgrade `Intervene { severity: Kill }` to terminate child
- [ ] Add pattern detection (loop detection, token budget)
- [ ] Add AI-powered anomaly detection reactor

## Notes

- Keep v0.1.0 focused on monitoring (no kills, only warnings)
- Reactor interface must support both sync display and async operations
- SessionState is lightweight summary, not full AgentSession
