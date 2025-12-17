# TraceView Abstraction for CLI Handlers

This ExecPlan is a living document and must be maintained per `.agent/PLANS.md`.

## Purpose / Big Picture

The CLI handlers in `crates/agtrace-cli/src/handlers` currently print directly to stdout/stderr and assemble formatted strings, which couples business logic with presentation. After this change, handlers will emit intent/data objects to a `TraceView` trait, allowing the console implementation to render the same output while enabling alternative UIs (TUI/JSON) without touching handler logic. Users should see identical CLI output, but the rendering will be centralized under the view abstraction.

## Progress

- [x] (2025-12-17 14:05Z) Drafted ExecPlan capturing goals, scope, and migration path.
- [x] (2025-12-17 14:41Z) Added TraceView traits/models/console implementation, refactored handlers to delegate all output to views, and ran `cargo test -p agtrace-cli`.

## Surprises & Discoveries

- Observation: Handler warnings surfaced after moving view logic; resolved by tightening imports and scoping view-model usage locally. Evidence: `cargo test -p agtrace-cli` initially warned about unused imports in `handlers/watch.rs`.

## Decision Log

- Decision: Added a generic `render_warning` to `SystemView` to centralize warning output needed by multiple handlers (session list auto-refresh, doctor run, watch stream endings). Rationale: Avoid proliferating bespoke warning events for trivial strings while still routing side effects through the view layer. Date/Author: 2025-12-17 / Assistant.
- Decision: Kept `render_stream_update` hook in `WatchView` with a no-op console implementation to preserve a place for future streaming UIs without altering handler logic. Rationale: Minimizes future churn while maintaining current output parity. Date/Author: 2025-12-17 / Assistant.

## Outcomes & Retrospective

- Handler layer no longer performs stdout/stderr writes; all output now routes through `TraceView` with console implementation mirroring previous formatting. Test suite `cargo test -p agtrace-cli` passes, indicating behavior parity for existing commands. Further UIs can plug into the new trait surface without touching command logic.

## Context and Orientation

`crates/agtrace-cli` hosts the CLI logic. Handlers under `crates/agtrace-cli/src/handlers/*.rs` orchestrate command behavior and currently own printing (e.g., `session_list.rs`, `watch.rs`, `index.rs`). Presentation helpers live in `crates/agtrace-cli/src/views/**` and data shapers in `crates/agtrace-cli/src/display_model/**`. The entrypoint `crates/agtrace-cli/src/commands.rs` wires CLI args to handlers. There is no existing `TraceView` abstraction; we will add a new `ui` module (e.g., `crates/agtrace-cli/src/ui/{mod,traits,console}.rs`) to define the interface and console implementation that wraps current view logic. Handlers must call the trait methods instead of `println!/eprintln!`, passing structured data (existing display models where possible).

## Plan of Work

Describe the new UI surface and then refactor handlers incrementally.

1. Introduce a `ui` module in `crates/agtrace-cli/src/ui` with:
   - `traits.rs` defining `TraceView` composed of subtraits (SystemView, SessionView, DiagnosticView, WatchView) matching handler responsibilities. Methods should accept structured data (e.g., `SessionDisplay`, `DoctorCheckDisplay`, new simple structs for provider/project summaries, corpus stats, init steps, index progress, scan stats, provider config updates). Include a simple `ProgressHandle` trait if progress reporting is needed.
   - `console.rs` implementing `TraceView` for the current stdout/stderr behavior by delegating to existing view helpers (`views::init`, `views::doctor`, `views::provider`, `views::session`, `views::pack`) or by moving the current handler print logic verbatim. Keep output parity (color, wording, ordering).
   - `mod.rs` exporting the traits and console implementation.
2. Add/adjust lightweight view-model structs/enums (e.g., provider/project summaries, corpus overview stats, init steps, doctor inspect payload, watch stream notifications, index progress) either in a new `ui/models.rs` or reusing `display_model` types. Prefer reusing `display_model` when it already matches the data needed; otherwise, create small structs in `ui` and use them in trait signatures.
3. Update `commands.rs` (and `main` wiring if needed) to construct a console view (`ConsoleTraceView`) once and pass `&dyn TraceView` into each handler entry point, adjusting function signatures accordingly.
4. Refactor handlers to remove direct `println!/eprintln!` calls. Each handler should compute data as before, then invoke the relevant `TraceView` methods. For watch/index flows with multiple messages, translate each display action into a view call while keeping logic intact. Preserve existing view helper usage inside the console implementation rather than inside handlers.
5. Ensure existing view helpers compile with the new console implementation; move any handler-specific formatting into `console.rs` while keeping the helpers for complex layouts (timeline, pack report, init steps, schemas). Remove unused imports from handlers after migration.
6. Run tests for `agtrace-cli` (and targeted components) to confirm behavior; adjust console output if tests expect specific formatting. If snapshot tests exist, follow CLAUDE.md guidance.

## Concrete Steps

- From repo root, create `crates/agtrace-cli/src/ui/{mod.rs,traits.rs,console.rs}` and any needed model file using `apply_patch`.
- Update Cargo module wiring in `crates/agtrace-cli/src/lib.rs` and any `use` paths to expose the new `ui` module.
- Change handler signatures in `crates/agtrace-cli/src/handlers/*.rs` to accept `&dyn TraceView` (or relevant subtraits) and replace print statements with trait calls.
- Adjust `crates/agtrace-cli/src/commands.rs` to instantiate the console view once and pass it to handlers. Handle the default guidance path similarly.
- Run `cargo test -p agtrace-cli` (or more targeted suites if runtime is high) from repo root to verify. If tests are slow, at least run unit tests touching modified modules (`cargo test -p agtrace-cli handlers::watch` etc.).
- Keep output parity by comparing representative command flows manually if necessary (e.g., ensure session list, provider list, watch attach messages still match prior wording/colors).

## Validation and Acceptance

The change is accepted when:
- `cargo test -p agtrace-cli` passes.
- All handlers compile and no `println!/eprintln!` remain in `crates/agtrace-cli/src/handlers`.
- CLI output for key commands (session list/show, provider list/detect/set, index update, watch attach/update paths, doctor check/run/inspect, init flow, corpus overview, pack) matches prior text/color/ordering when run against existing data.
- The console view fully implements the `TraceView` methods used by handlers without panics or unimplemented stubs.

## Idempotence and Recovery

The plan is additive and refactors only handler/view wiring. Re-running steps is safe: module additions are deterministic, and handler conversions are mechanical. If compilation fails midway, re-open the plan, fix trait signatures, and rerun `cargo test`. No data migrations occur.

## Artifacts and Notes

- Keep notable diffs or command outputs here during execution, especially if adjusting formatting for parity.

## Interfaces and Dependencies

- Introduce `TraceView` and subtraits in `crates/agtrace-cli/src/ui/traits.rs` with methods such as `render_session_list(&[SessionSummary])`, `render_session_detail(&SessionDisplay, ViewStyle)`, `render_provider_list(&[ProviderConfigSummary])`, `render_project_list(&[ProjectSummary])`, `render_corpus_overview(&CorpusStats)`, `render_diagnose_results(&[DiagnoseResult], bool)`, `render_file_check(&DoctorCheckDisplay)`, `render_raw_inspection(RawContent)`, `render_provider_schema(&ProviderSchemaContent)`, `render_pack_report(&[SessionDigest], ReportTemplate)`, `on_watch_attached(...)`, `render_watch_update(&SessionState, &[AgentEvent])`, `on_watch_rotated(...)`, `on_watch_waiting(&str)`, `render_watch_error(&str, fatal: bool)`, and `render_index_progress(...)`. Exact names may adapt to match handler needs but must cover all existing outputs. Define supporting structs (e.g., provider/project summaries, corpus stats, init step enum, index progress summary) under `ui`.
