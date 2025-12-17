# Retire intervention pipeline

This ExecPlan is a living document and must be maintained in accordance with `.agent/PLANS.md`. Keep all sections current as work proceeds.

## Purpose / Big Picture

The CLI and runtime currently expose an "intervention" pipeline (Intervention enums, Signal handling, executor threads, CLI `pgrep` + `kill`). We do not plan to use this feature, so the goal is to remove intervention-specific types, executors, events, and UI handling while keeping the rest of the reaction system (warnings/continues) intact. After this change, reactors will only warn or continue; the runtime will no longer spawn executor threads; the CLI will no longer prompt to send signals.

## Progress

- [x] (2025-12-17 18:37Z) Drafted plan to delete intervention pipeline and leave warning-only reactions.
- [x] (2025-12-17 18:44Z) Removed intervention types and executor plumbing from runtime crate; Reaction now only Continue/Warn.
- [x] (2025-12-17 18:44Z) Updated CLI reactors, UI, and handler wiring to match the simplified reaction surface and deleted `intervention.rs`.
- [x] (2025-12-17 18:50Z) Cleaned up docs and ran tests (`cargo test -p agtrace-runtime`, `cargo test -p agtrace-cli`) to confirm no intervention code remains.

## Surprises & Discoveries

- None yet.

## Decision Log

- Decision: Remove the intervention feature entirely (types, executors, runtime event variants, CLI wiring) and downgrade any prior intervention paths to warnings or no-ops as appropriate. Rationale: The feature is unused and should not incur complexity or prompting in the watch flow. Date/Author: 2025-12-17 / Assistant.

## Outcomes & Retrospective

- Pending completion.

## Context and Orientation

Before removal, intervention concepts lived in `crates/agtrace-runtime/src/runtime.rs` (ProcessTarget, Intervention, Signal, InterventionExecutor, RuntimeEvent::InterventionExecuted, RuntimeConfig::executor/allow_kill, to_intervention helper and executor thread) and were re-exported in `crates/agtrace-runtime/src/lib.rs`. Reactions are now defined in `crates/agtrace-runtime/src/reactor.rs` with only Continue/Warn variants. The CLI no longer has `crates/agtrace-cli/src/intervention.rs`; `crates/agtrace-cli/src/handlers/watch.rs` constructs a `RuntimeConfig` without an executor, and UI rendering in `crates/agtrace-cli/src/ui/*` only handles warnings. CLI reactors emitting interventions previously included `reactors/safety_guard.rs`, `reactors/stall_detector.rs`, and `reactors/token_usage_monitor.rs`; these now emit warnings. Documentation references to interventions remain in `docs/execplan-runtime-refactor.md` and must be cleaned.

## Plan of Work

First, simplify the runtime reaction surface by removing the Intervene variant and Severity enum from `crates/agtrace-runtime/src/reactor.rs`, leaving only Continue and Warn. Drop all intervention-specific types and executor plumbing from `crates/agtrace-runtime/src/runtime.rs`, including RuntimeEvent::InterventionExecuted, RuntimeConfig::executor/allow_kill, and the thread that invokes executors; adjust event emission to only cover attach/update/reaction/wait/rotate/fatal. Remove the re-exports in `crates/agtrace-runtime/src/lib.rs`. Next, delete the CLI executor module (`crates/agtrace-cli/src/intervention.rs`) and stop importing it in `crates/agtrace-cli/src/lib.rs` and `handlers/watch.rs`; adjust RuntimeConfig construction accordingly and prune handling of InterventionExecuted events. Update CLI reactors to return Warn or Continue instead of Intervene, revising messages to remain informative but non-executing. Update UI traits and console rendering to drop intervention severity handling and only display warnings. Remove or rewrite tests that expect Intervention variants or executor behavior. Finally, scrub documentation (`docs/execplan-runtime-refactor.md`) of intervention references and run the relevant test suites to ensure the feature is fully removed.

## Concrete Steps

1. In `crates/agtrace-runtime/src/reactor.rs`, remove Severity and the Intervene variant; update imports and any tests in that file accordingly.
2. In `crates/agtrace-runtime/src/runtime.rs`, delete ProcessTarget/Intervention/Signal/InterventionExecutor, RuntimeConfig executor/allow_kill, RuntimeEvent::InterventionExecuted, the to_intervention helper, and executor thread logic; keep the runtime loop emitting reactions as warnings only. Update or remove related tests.
3. Update `crates/agtrace-runtime/src/lib.rs` exports to match the reduced surface.
4. Delete `crates/agtrace-cli/src/intervention.rs` and remove module references from `crates/agtrace-cli/src/lib.rs` and `crates/agtrace-cli/src/handlers/watch.rs`; remove InterventionExecuted handling in the watch loop.
5. Revise CLI reactors (`reactors/safety_guard.rs`, `reactors/stall_detector.rs`, `reactors/token_usage_monitor.rs`) to emit Warn or Continue only; adjust tests to match.
6. Simplify UI reaction handling in `crates/agtrace-cli/src/ui/traits.rs` and `crates/agtrace-cli/src/ui/console.rs` to drop intervention severity branches.
7. Remove intervention references from `docs/execplan-runtime-refactor.md` and any other docs if found.
8. Run `cargo test --workspace` (from repo root); if too slow, run `cargo test -p agtrace-runtime` and `cargo test -p agtrace-cli` at minimum. Ensure failures are addressed and update this plan’s Progress with timestamps.

## Validation and Acceptance

Acceptance: All intervention-related types, modules, and event handling are removed; reactors only produce Continue or Warn; `agtrace watch` no longer references or prompts for interventions. Running `cargo test --workspace` from the repository root succeeds. Manual smoke: launching `agtrace watch` should print reactions as warnings without any prompt to send signals or mention interventions.

## Idempotence and Recovery

Edits are deletions and simplifications; re-running steps is safe as long as files are saved. If tests fail due to missing symbols, revisit the corresponding module to remove or rewrite the reference. No data migrations are involved.

## Artifacts and Notes

- Tests (2025-12-17 18:50Z): `cargo test -p agtrace-runtime`, `cargo test -p agtrace-cli`.

## Interfaces and Dependencies

Post-change, `crates/agtrace-runtime/src/reactor.rs` should expose only `Reaction::{Continue, Warn}` and drop Severity entirely. `RuntimeEvent` should not include intervention variants, and `RuntimeConfig` should not accept an executor. The CLI should have no `intervention` module, no executor wiring in `handlers/watch.rs`, and its reactors should rely on warnings instead of interventions. UI reaction rendering should match the two-variant Reaction enum.
