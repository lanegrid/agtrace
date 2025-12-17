# Runtime/Engine Layering and Interventions

This ExecPlan is a living document and must be maintained per `.agent/PLANS.md`.

## Purpose / Big Picture

Move `agtrace` from a CLI-centric watcher into a layered runtime where pure parsing lives in `agtrace-engine`, stateful orchestration and reactors live in a new `agtrace-runtime`, and `agtrace-cli` is only responsible for presentation plus OS-specific intervention execution. Users should be able to run `agtrace watch` and see the same stream output while the system now supports structured runtime events, pluggable executors for process control, and testable parsing of state updates with no I/O.

## Progress

- [x] (2025-12-17 17:14Z) Drafted ExecPlan after reviewing watcher/reactor architecture, user requirements, and PLANS.md/CLAUDE.md.
- [x] (2025-12-17 17:27Z) Phase 1: Added `agtrace-engine` `StateUpdates`/`ContextWindowUsage`, implemented `extract_state_updates`, refactored CLI watcher to consume it, and ran `cargo test -p agtrace-engine state_updates` plus `cargo test -p agtrace-cli handlers::watch`.
- [x] (2025-12-17 17:50Z) Phase 2: Added `agtrace-runtime` crate with watcher/reactor/runtime loop, re-exported SessionState/Reaction/streaming to CLI, refactored `watch` handler to subscribe to `RuntimeEvent`, and ran `cargo test -p agtrace-runtime` plus `cargo test -p agtrace-cli handlers::watch`.
- [x] (2025-12-17 18:05Z) Phase 3: Added runtime intervention plumbing and `RuntimeEvent::InterventionExecuted`, implemented CLI `CliInterventionExecutor` (pgrep + prompt + signals), wired into `watch` runtime config, and surfaced executor results via `TraceView` warnings. Tests: `cargo test -p agtrace-runtime`, `cargo test -p agtrace-cli handlers::watch`.
- [ ] Phase 4: Refactor TUI rendering into a runtime observer consuming `RuntimeEvent`; shrink `watch` handler to runtime setup/subscription.
- [ ] Validation: Run targeted tests (`cargo test -p agtrace-engine`, new `agtrace-runtime` tests, `cargo test -p agtrace-cli`).

## Surprises & Discoveries

- None yet. Record unexpected behaviors or data quirks as they appear.

## Decision Log

- Decision: Move token usage types (`ContextWindowUsage` and newtypes) into `agtrace-engine` and re-export them to keep CLI imports stable while enabling runtime to depend on engine-owned parsing. Rationale: Keeps token math co-located with pure parsing (`StateUpdates`) and supports the required dependency direction (`types` ← `engine` ← `runtime` ← `cli`). Date/Author: 2025-12-17 / Assistant.
- Decision: Introduced `agtrace-runtime` as a dedicated crate owning watcher/reactor/runtime loop while CLI re-exports the traits/types and subscribes to `RuntimeEvent` for rendering. Rationale: Enforces the one-way dependency chain and prepares for executor injection without embedding orchestration in the CLI. Date/Author: 2025-12-17 / Assistant.
- Decision: Added `Intervention`/`ProcessTarget`/`InterventionExecutor` surfaces to runtime and a CLI `CliInterventionExecutor` using `pgrep` + user confirmation + POSIX signals. Rationale: Provide an initial executor path so `SafetyGuard`/reactor interventions propagate through runtime events without blocking the main stream (execution on a worker thread). Date/Author: 2025-12-17 / Assistant.

## Outcomes & Retrospective

To be completed after milestones. Summarize behavioral changes and remaining gaps.

## Context and Orientation

Current layout after Phase 2: `crates/agtrace-runtime` hosts `SessionState`/`Reaction`/`Reactor` plus `Runtime` and the streaming watcher; CLI re-exports those surfaces and keeps concrete reactors (TuiRenderer, StallDetector, SafetyGuard, TokenUsageMonitor) under `crates/agtrace-cli/src/reactors/*.rs`. The `watch` handler now constructs `RuntimeConfig`, starts `Runtime`, and listens for `RuntimeEvent` to drive `TraceView`. Engine crate `crates/agtrace-engine` exposes `ContextWindowUsage` and `StateUpdates` parsing; token limits remain in CLI for now. Types for events live in `crates/agtrace-types/v2.rs`. Dependency direction now follows `agtrace-types` ← `agtrace-engine` ← `agtrace-runtime` ← `agtrace-cli`.

## Plan of Work

Establish a pure parsing surface in the engine, then introduce a dedicated runtime crate that owns watchers, session state, and reactor execution, and finally adapt CLI to observe runtime events and perform OS interactions.

1. **Engine state extraction**: Add `StateUpdates` and `ContextWindowUsage` (or rehome the existing type) under `crates/agtrace-engine`, plus a pure `extract_state_updates(event: &AgentEvent) -> StateUpdates` that captures model/context window metadata, token snapshots, reasoning tokens, error/turn flags. Provide helper methods to merge/apply updates to a runtime-managed `SessionState`. Cover with unit tests against representative JSON/metadata snippets.
2. **Create `agtrace-runtime` crate**: Register the new crate in the workspace and depend on `agtrace-engine` and `agtrace-types`. Move `SessionState`, `Reaction`, `Severity`, `Reactor` trait, and `run_reactors`-style coordination here. Relocate `streaming/watcher.rs` logic (filesystem polling and `SessionUpdate`) and create a `Runtime` struct that owns a worker thread consuming `Receiver<StreamEvent>` and yielding a typed `RuntimeEvent` iterator/stream. Define `RuntimeEvent`, `RuntimeConfig`, `Intervention`, `ProcessTarget`, and `InterventionExecutor` traits/enums per the user contract, keeping runtime free of CLI/OS specifics. Ensure runtime emits `StateUpdated`, `ReactionTriggered`, `InterventionExecuted`, `SessionRotated`, `SessionAttached`, `FatalError`, and similar events without direct UI calls.
3. **Wire interventions**: In runtime, translate reactor `Reaction::Intervene` into `Intervention` commands issued via an injected `InterventionExecutor` without blocking the stream (spawn task/thread or queue). Keep reactors pure decision-makers returning `Reaction`. In CLI, implement `InterventionExecutor` with `pgrep`-based `ProcessTarget::Name` resolution, prompting the user before sending signals, and handling failure cases gracefully to send back `Result<(), String>`.
4. **Observer-driven UI**: Remove `TuiRenderer` from the reactor set; instead, adapt it (or a new renderer) to consume `RuntimeEvent` alongside the existing `TraceView` wiring. Adjust `watch` handler to build `RuntimeConfig` (provider, reactors, executor, paths, poll interval), start the runtime, and subscribe to its events, forwarding updates to `TraceView` methods. Keep handler code around 100 LOC by delegating state updates and reactions to runtime.
5. **Migration and cleanup**: Update imports and module wiring so CLI relies on runtime APIs instead of direct engine parsing or watcher internals. Ensure dependency direction matches the required chain; adjust `agtrace-cli` to no longer access moved types directly except via runtime exports. Add targeted tests in `agtrace-runtime` for event propagation and reaction/intervention routing, and update CLI tests to cover executor prompting stubs and runtime subscription hooks.

## Concrete Steps

- Add `crates/agtrace-runtime` to the workspace; scaffold `Cargo.toml` with dependencies on `agtrace-engine`, `agtrace-types`, `agtrace-providers` (if runtime continues to own file normalization), and shared utilities used by the watcher.
- In `agtrace-engine`, introduce `state_updates` (or similar) module defining `ContextWindowUsage`, `StateUpdates`, and `extract_state_updates`; move or re-export the current token usage types from CLI to keep consumers consistent. Write unit tests in `crates/agtrace-engine/tests` for metadata parsing (model names, context window limits, reasoning tokens, error/turn flags).
- Port `streaming/watcher.rs`, `reactor.rs`, and reactor structs to `agtrace-runtime`, adjusting namespaces and dependencies. Implement `Runtime`, `RuntimeEvent`, and `RuntimeConfig` surfaces, ensuring the runtime owns the event loop and emits events via a channel/iterator.
- Define `Intervention`, `ProcessTarget`, `InterventionExecutor`, and runtime handling that transforms `Reaction::Intervene` into executor calls while continuing to process new stream events.
- Update `agtrace-cli` to construct `RuntimeConfig`, implement `InterventionExecutor` (pgrep + prompt + signal), and refactor `handlers/watch.rs` to subscribe to runtime events and render via `TraceView`, with `TuiRenderer` moved to a pure observer.
- Run `cargo test -p agtrace-engine`, `cargo test -p agtrace-runtime` (new), and `cargo test -p agtrace-cli`; add focused unit tests for the executor prompt flow and runtime event emissions if feasible.

## Validation and Acceptance

Acceptance requires: engine tests demonstrating `extract_state_updates` correctly updates session state from JSON metadata and token usage snapshots without I/O; `agtrace-runtime` tests showing `RuntimeEvent` emission for updates, reactions, and interventions; `agtrace-cli` `watch` handler simplified to ~100 LOC that primarily wires runtime config and subscribes to events; and manual or test verification that a `SafetyGuard` `Kill` reaction triggers an executor call with the expected signal. `cargo test` commands listed above must pass.

## Idempotence and Recovery

Changes are additive and relocations are controlled; re-running steps reuses the same modules and tests. If runtime wiring fails mid-migration, fallback by keeping old CLI paths until the runtime API is ready, then remove deprecated code once tests pass. File moves should preserve history where possible; if a move fails, reapply with `git mv` to avoid duplicates.

## Artifacts and Notes

Populate with notable command outputs (e.g., failing tests before fixes, runtime event traces) during execution as needed.

## Interfaces and Dependencies

By the end of this plan, the following interfaces should exist:

- `agtrace-engine::state_updates`: `pub struct ContextWindowUsage { ... }`, `pub struct StateUpdates { model: Option<String>, context_window_limit: Option<u64>, usage: Option<ContextWindowUsage>, reasoning_tokens: Option<i32>, is_error: bool, is_new_turn: bool }`, and `pub fn extract_state_updates(event: &agtrace_types::v2::AgentEvent) -> StateUpdates`.
- `agtrace-runtime`: `pub struct SessionState { ... }`, `pub enum Reaction { Continue, Warn(String), Intervene { reason: String, severity: Severity } }`, `pub trait Reactor { fn name(&self) -> &str; fn handle(&mut self, ctx: ReactorContext) -> anyhow::Result<Reaction>; }`, `pub enum Intervention { Notify { title: String, message: String }, KillProcess { target: ProcessTarget, signal: Signal } }`, `pub enum ProcessTarget { Pid(u32), Name(String), LogFileWriter { path: PathBuf } }`, `pub enum RuntimeEvent { SessionAttached { display_name: String }, StateUpdated { state: Box<SessionState>, new_events: Vec<AgentEvent> }, ReactionTriggered { reactor_name: String, reaction: Reaction }, InterventionExecuted { intervention: Intervention, result: Result<(), String> }, SessionRotated { old_path: PathBuf, new_path: PathBuf }, FatalError(String) }`, `pub struct RuntimeConfig { provider: Box<dyn LogProvider>, reactors: Vec<Box<dyn Reactor>>, executor: Box<dyn InterventionExecutor>, watch_path: PathBuf, poll_interval: Duration }`, and `pub struct Runtime` that consumes `StreamEvent` and yields `RuntimeEvent`.
- `agtrace-cli`: `InterventionExecutor` implementation using `pgrep` for `ProcessTarget::Name`, user prompt before signals, and integration into `watch` command setup; `TuiRenderer` adapted to an observer that reacts to `RuntimeEvent` instead of being a reactor.
