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

## Phase 3: Filtering & Loading Unification ⊘

### Status: Skipped (Not Needed)
**Priority: LOW** - Investigation revealed no duplication to refactor.

### Analysis
After examining the codebase:
- `session_show.rs` has `filter_events_v2()` function for --hide/--only filtering
- `lab_export.rs` does NOT use hide/only filtering - it exports all events
- **No duplication exists** - filtering logic only in one place
- **Conclusion**: Refactoring not needed, would be over-engineering

### Tasks

#### 3.1 Investigation ✅
- [x] Review filtering logic in `session_show` and `lab_export`
- [x] Determined: No shared filtering logic to extract
- [x] Decision: Skip Phase 3 as unnecessary

## Summary

### Completed Refactoring ✅

This refactoring successfully separated domain logic from presentation concerns:

**Phase 1: Pack Analysis (HIGH)** ✅
- Moved 300+ lines of analysis logic to `agtrace-engine/src/analysis/`
- Created metrics, lenses, digest, and packing modules
- Reduced pack.rs from 523 → 95 lines (82% reduction)

**Phase 2: Doctor Diagnostics (MEDIUM)** ✅
- Moved validation logic to `agtrace-engine/src/diagnostics/`
- Created validator module with unit tests
- Reduced doctor_run.rs from 257 → 117 lines (54% reduction)

**Phase 3: Filtering Unification (LOW)** ⊘
- Investigated and found no duplication
- Correctly skipped unnecessary refactoring

### Architecture Improvements

**Before:**
```
CLI Handlers (pack.rs, doctor_run.rs)
├── Business Logic (metrics, analysis, validation)
├── Data Structures (SessionDigest, DiagnoseResult)
└── Presentation (formatting, colors)
```

**After:**
```
agtrace-engine (Domain Layer)
├── analysis/
│   ├── metrics.rs (SessionMetrics, compute_metrics)
│   ├── digest.rs (SessionDigest, text cleaning)
│   ├── lenses.rs (selection algorithms)
│   └── packing.rs (high-level API)
└── diagnostics/
    └── validator.rs (error categorization)

agtrace-cli (Presentation Layer)
├── handlers/ (thin controllers, ~100 lines each)
└── output/ (formatting and display)
```

### Metrics

- **Total lines moved to engine:** ~600 lines
- **Total lines reduced from handlers:** ~570 lines
- **Code reduction:** 68% average reduction in handler complexity
- **Test coverage:** Added unit tests for core logic
- **Build quality:** All tests passing, no clippy warnings

### Benefits

1. **Testability:** Pure business logic can be unit tested independently
2. **Reusability:** Engine modules can be used by other interfaces (API, TUI, etc.)
3. **Maintainability:** Clear separation of concerns
4. **Extensibility:** Easy to add new analysis lenses or diagnostic checks

## Post-Refactoring Improvements 💡

Potential future improvements:

- [ ] Add comprehensive unit tests for all engine analysis functions
- [ ] Add benchmarks for large session corpus analysis
- [ ] Document engine public API with usage examples
- [ ] Consider adding more selection lenses (e.g., token usage patterns)
- [ ] Add integration tests for end-to-end workflows
