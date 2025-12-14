# Refactoring Progress: CLI-Engine Separation

## Goal
Separate responsibilities between `agtrace-cli` (Presentation Layer) and `agtrace-engine` (Domain Layer) to improve maintainability, testability, and extensibility.

## Architecture Principles
- **Engine (Domain Layer)**: Pure business logic - metrics, analysis, selection, diagnostics
- **CLI (Interface Layer)**: User input handling, Engine API calls, output formatting

## Phase 1: Pack (Analysis) Logic Migration ✅

### Status: Completed
**Priority: HIGH** - Successfully separated pack.rs analysis logic into engine layer.

### Summary of Changes
- **Engine**: Created `agtrace-engine/src/analysis/` module with:
  - `metrics.rs`: SessionMetrics computation (pure business logic)
  - `digest.rs`: SessionDigest with text cleaning helpers
  - `lenses.rs`: Selection lenses (Failures, Bottlenecks, Toolchains, Loops)
  - `packing.rs`: High-level `analyze_and_select_sessions()` API
- **CLI**: Created `agtrace-cli/src/output/pack_view.rs` for presentation
- **Result**: pack.rs reduced from 523 lines to ~95 lines (82% reduction)

### Tasks

#### 1.1 Move SessionMetrics ✅
- [x] Create `crates/agtrace-engine/src/analysis/mod.rs`
- [x] Create `crates/agtrace-engine/src/analysis/metrics.rs`
- [x] Move `SessionMetrics` struct
- [x] Move `compute_metrics()` function

#### 1.2 Move Lens Logic ✅
- [x] Create `crates/agtrace-engine/src/analysis/lenses.rs`
- [x] Move `LensType` enum
- [x] Move `Lens` struct and implementations
- [x] Move `Thresholds` calculation logic
- [x] Move `select_sessions_by_lenses()` function

#### 1.3 Move SessionDigest ✅
- [x] Create `crates/agtrace-engine/src/analysis/digest.rs`
- [x] Move `SessionDigest` struct
- [x] Move `clean_snippet()` helper
- [x] Move `find_activation()` helper
- [x] Move `truncate_string()` helper

#### 1.4 Create High-Level API ✅
- [x] Create `crates/agtrace-engine/src/analysis/packing.rs`
- [x] Create public API: `analyze_and_select_sessions()`
- [x] Keep `balance_sessions_by_provider()` in CLI (uses DB types)

#### 1.5 Create Presentation Layer ✅
- [x] Create `crates/agtrace-cli/src/output/pack_view.rs`
- [x] Move `output_compact()` formatter
- [x] Move `output_diagnose()` formatter
- [x] Move `output_tools()` formatter
- [x] Move `print_digest_summary()` helper

#### 1.6 Refactor CLI Handler ✅
- [x] Refactor `handlers/pack.rs` to use engine API
- [x] Reduce from 523 lines to 95 lines
- [x] CLI now only: parse args → call engine → format output

#### 1.7 Quality Assurance ✅
- [x] Run `cargo test` - All 27 tests passed
- [x] Run `cargo clippy` - No warnings
- [x] Run `cargo fmt` - Formatted
- [x] Manually test `agtrace pack --help` - Works correctly

## Phase 2: Diagnostics (Doctor) Logic Migration ✅

### Status: Completed
**Priority: MEDIUM** - Successfully separated doctor validation logic into engine layer.

### Summary of Changes
- **Engine**: Created `agtrace-engine/src/diagnostics/` module with:
  - `validator.rs`: Parse error categorization and validation types
  - `DiagnoseResult`, `FailureType`, `FailureExample` structures
  - `categorize_parse_error()` function with unit tests
- **CLI**: Created `agtrace-cli/src/output/doctor_view.rs` for presentation
- **Result**: doctor_run.rs reduced from 257 lines to 117 lines (54% reduction)

### Tasks

#### 2.1 Move Validation Logic ✅
- [x] Create `crates/agtrace-engine/src/diagnostics/mod.rs`
- [x] Create `crates/agtrace-engine/src/diagnostics/validator.rs`
- [x] Move `DiagnoseResult`, `FailureType`, `FailureExample` to engine
- [x] Move `categorize_parse_error` logic from `doctor_run.rs`
- [x] Add unit tests for validation logic

#### 2.2 Create Presentation Layer ✅
- [x] Create `crates/agtrace-cli/src/output/doctor_view.rs`
- [x] Move `print_results()` output formatting logic

#### 2.3 Refactor CLI Handlers ✅
- [x] Refactor `handlers/doctor_run.rs` to use engine API
- [x] CLI now only: walk directories → call validator → format errors
- [x] doctor_check.rs kept as-is (primarily presentation logic)

#### 2.4 Quality Assurance ✅
- [x] Run `cargo test` - All 27 tests passed
- [x] Run `cargo clippy` - No warnings
- [x] Run `cargo fmt` - Formatted
- [x] Test `agtrace doctor run --help` - Works correctly
- [x] Test `agtrace doctor check --help` - Works correctly

## Phase 3: Filtering & Loading Unification 📋

### Status: Not Started
**Priority: LOW**

### Tasks

#### 3.1 Unify Event Filtering
- [ ] Review filtering logic in `session_show` and `lab_export`
- [ ] Create `crates/agtrace-engine/src/query.rs` (or similar)
- [ ] Define `EventFilter` struct
- [ ] Move `filter_events_v2()` logic to engine
- [ ] Create common API for event filtering

#### 3.2 Refactor CLI Usages
- [ ] Update `session_show` to use common filter API
- [ ] Update `lab_export` to use common filter API

#### 3.3 Quality Assurance
- [ ] Run tests
- [ ] Test filtering with --hide, --only flags
- [ ] Commit

## Post-Refactoring Improvements 💡

Potential improvements after main refactoring:

- [ ] Add comprehensive unit tests for engine analysis module
- [ ] Consider internationalization for selection reasons
- [ ] Add benchmarks for large session corpus analysis
- [ ] Document engine API with examples
- [ ] Consider adding tracing/logging to engine for debugging

## Notes

- Keep commits small and focused
- One-line commit messages only
- Run lint and fmt before each commit
- For snapshot tests: accept → diff → verify → commit with implementation
- Prioritize quality over speed
