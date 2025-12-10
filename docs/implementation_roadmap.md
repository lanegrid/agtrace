# agtrace v2.0 Implementation Roadmap

## Overview

This document outlines the implementation plan for migrating agtrace from v1.x (file-based) to v2.0 (Pointer Edition with Schema-on-Read).

**Target:** MVP with core functionality (`scan`, `list`, `view`)

**Timeline Estimate:** 4-7 days for experienced Rust developer

---

## Phase 0: Preparation

**Goal:** Set up development environment and create feature branch.

### Tasks

- [ ] Create feature branch `feat/pointer-edition`
- [ ] Review all spec documents:
  - [ ] `docs/agtrace_cli_spec.md`
  - [ ] `docs/database_schema.md`
  - [ ] `docs/architecture.md`
  - [ ] `docs/agent_event_schema_v1.md` (unchanged)
- [ ] Set up test environment with sample logs from all providers

**Estimated Time:** 0.5 day

---

## Phase 1: Database Foundation

**Goal:** Implement SQLite schema and storage abstraction.

### 1.1 Add Dependencies

**File:** `Cargo.toml`

```toml
[dependencies]
rusqlite = { version = "0.30", features = ["bundled"] }
```

**Tasks:**
- [ ] Add `rusqlite` dependency
- [ ] Run `cargo build` to verify

**Estimated Time:** 0.1 day

### 1.2 Create Database Module

**File:** `src/db/mod.rs` (new)

**Tasks:**
- [ ] Define `Database` struct
- [ ] Implement schema initialization:
  ```rust
  pub fn init_schema(&self) -> Result<()> {
      // CREATE TABLE projects
      // CREATE TABLE sessions
      // CREATE TABLE log_files
      // CREATE INDEX ...
  }
  ```
- [ ] Implement CRUD operations:
  - [ ] `insert_or_update_project()`
  - [ ] `insert_or_update_session()`
  - [ ] `insert_or_update_log_file()`
  - [ ] `list_sessions(project_hash, limit)`
  - [ ] `get_session_files(session_id)`

**Estimated Time:** 1 day

### 1.3 Write Database Tests

**File:** `src/db/mod.rs` (tests)

**Tasks:**
- [ ] Test schema initialization
- [ ] Test project insertion
- [ ] Test session insertion with FK constraint
- [ ] Test file insertion
- [ ] Test list_sessions query
- [ ] Test get_session_files query

**Estimated Time:** 0.5 day

**Milestone:** Database foundation ready

---

## Phase 2: Scanner Implementation

**Goal:** Implement header-only scanning for all providers.

### 2.1 Define Scanner Trait

**File:** `src/scanner/mod.rs` (new)

**Tasks:**
- [ ] Define `SessionMetadata` struct:
  ```rust
  pub struct SessionMetadata {
      pub session_id: String,
      pub project_hash: String,
      pub provider: String,
      pub start_ts: String,
      pub end_ts: Option<String>,
      pub snippet: Option<String>,
      pub log_files: Vec<LogFileMetadata>,
  }
  ```
- [ ] Define `Scanner` trait:
  ```rust
  pub trait Scanner {
      fn scan(&self, log_root: &Path, project_hash: &str) -> Result<Vec<SessionMetadata>>;
  }
  ```

**Estimated Time:** 0.2 day

### 2.2 Implement ClaudeScanner

**File:** `src/scanner/claude.rs` (new)

**Tasks:**
- [ ] Detect project directory by encoding `project_root`
- [ ] Walk directory for `*.jsonl` files
- [ ] Read first few lines per file
- [ ] Extract `sessionId`, `cwd`, `timestamp`
- [ ] Group files by `session_id` (handle `agent-*.jsonl`)
- [ ] Return `SessionMetadata`

**Estimated Time:** 0.5 day

### 2.3 Implement CodexScanner

**File:** `src/scanner/codex.rs` (new)

**Tasks:**
- [ ] Walk `log_root` recursively for `rollout-*.jsonl`
- [ ] Read first 10 lines per file
- [ ] Extract `session_id` from `session_meta` or filename fallback
- [ ] Extract `cwd` from `payload.cwd`
- [ ] Match against `project_hash`
- [ ] Return `SessionMetadata`

**Estimated Time:** 0.5 day

### 2.4 Implement GeminiScanner

**File:** `src/scanner/gemini.rs` (new)

**Tasks:**
- [ ] Detect project directory by matching `project_hash` to 64-hex dirname
- [ ] Read `logs.json` for session metadata
- [ ] Walk `chats/` for `session-*.json` files
- [ ] Extract `sessionId`, `timestamp`
- [ ] Return `SessionMetadata`

**Estimated Time:** 0.5 day

### 2.5 Implement Scan Command Handler

**File:** `src/cli/handlers/scan.rs` (new, replaces `import.rs`)

**Tasks:**
- [ ] Parse CLI args (`--provider`, `--force`, `--verbose`)
- [ ] Resolve `project_root` and calculate `project_hash`
- [ ] For each enabled provider:
  - [ ] Instantiate scanner
  - [ ] Call `scanner.scan()`
  - [ ] Write results to database
- [ ] Update `projects.last_scanned_at`
- [ ] Print summary

**Estimated Time:** 0.5 day

### 2.6 Write Scanner Tests

**File:** `tests/scanner_tests.rs` (new)

**Tasks:**
- [ ] Test ClaudeScanner with sample logs
- [ ] Test CodexScanner with sample logs
- [ ] Test GeminiScanner with sample logs
- [ ] Test scan command end-to-end

**Estimated Time:** 0.5 day

**Milestone:** Scanning functionality complete

---

## Phase 3: List Command

**Goal:** Query SQLite and display session list.

### 3.1 Update List Handler

**File:** `src/cli/handlers/list.rs` (modify)

**Tasks:**
- [ ] Remove file-walking logic
- [ ] Query database via `db.list_sessions()`
- [ ] Format output (plain / JSON)
- [ ] Support `--project`, `--hash`, `--recent`, `--all`

**Estimated Time:** 0.3 day

### 3.2 Write List Tests

**File:** `tests/list_tests.rs` (modify)

**Tasks:**
- [ ] Test list with populated database
- [ ] Test filtering by project
- [ ] Test limit/recent
- [ ] Test JSON output

**Estimated Time:** 0.2 day

**Milestone:** List command complete

---

## Phase 4: View Command (Schema-on-Read)

**Goal:** Dynamically load and normalize raw logs.

### 4.1 Create Viewer Module

**File:** `src/viewer/mod.rs` (new)

**Tasks:**
- [ ] Define `Viewer` struct
- [ ] Implement `view(session_id, format)`:
  1. Query `db.get_session_files(session_id)`
  2. For each file:
     - Open file
     - Detect provider
     - Call mapper (reuse existing `ClaudeMapper`, `CodexMapper`, `GeminiMapper`)
     - Collect events
  3. Merge and sort events by `ts`
  4. Display

**Estimated Time:** 0.8 day

### 4.2 Implement Event Merging

**File:** `src/viewer/merge.rs` (new)

**Tasks:**
- [ ] Implement k-way merge using `BinaryHeap`
- [ ] Sort events by timestamp
- [ ] Handle parse errors gracefully (emit `parse_error` event)

**Estimated Time:** 0.3 day

### 4.3 Update View Handler

**File:** `src/cli/handlers/view.rs` (new, replaces `show.rs`)

**Tasks:**
- [ ] Parse CLI args (`--raw`, `--json`, `--timeline`)
- [ ] Call `viewer.view(session_id, format)`
- [ ] Display output

**Estimated Time:** 0.2 day

### 4.4 Write View Tests

**File:** `tests/view_tests.rs` (new)

**Tasks:**
- [ ] Test view with Claude session
- [ ] Test view with Codex session
- [ ] Test view with Gemini session
- [ ] Test `--raw` mode
- [ ] Test `--json` mode
- [ ] Test parse error handling

**Estimated Time:** 0.5 day

**Milestone:** View command complete

---

## Phase 5: CLI Integration

**Goal:** Wire up new commands and update CLI structure.

### 5.1 Update CLI Args

**File:** `src/cli/args.rs` (modify)

**Tasks:**
- [ ] Rename `Import` to `Scan`
- [ ] Update `Scan` args (remove `--out-jsonl`, `--session-id-prefix`, `--dry-run`)
- [ ] Rename `Show` to `View`
- [ ] Update `View` args (add `--raw`, `--json`, `--timeline`)
- [ ] Remove `Find`, `Stats`, `Export` (comment out for now)
- [ ] Update `List` args (add `--project`, `--hash`, `--recent`, `--all`)

**Estimated Time:** 0.3 day

### 5.2 Update Main Handler

**File:** `src/cli/handlers/mod.rs` (modify)

**Tasks:**
- [ ] Route `Scan` to `scan_handler()`
- [ ] Route `View` to `view_handler()`
- [ ] Update `List` routing
- [ ] Remove/comment out `Find`, `Stats`, `Export`

**Estimated Time:** 0.2 day

### 5.3 Update Global Options

**File:** `src/main.rs` (modify)

**Tasks:**
- [ ] Initialize database on startup (`db.init_schema()`)
- [ ] Pass database connection to handlers

**Estimated Time:** 0.2 day

**Milestone:** CLI integration complete

---

## Phase 6: Project Command

**Goal:** Display project list and metadata.

### 6.1 Implement Project Handler

**File:** `src/cli/handlers/project.rs` (modify)

**Tasks:**
- [ ] Implement `project list` subcommand
- [ ] Query `SELECT * FROM projects`
- [ ] Join with sessions count
- [ ] Format output (table with HASH, ROOT PATH, SESSIONS, LAST SCANNED)

**Estimated Time:** 0.3 day

### 6.2 Write Project Tests

**File:** `tests/project_tests.rs` (modify)

**Tasks:**
- [ ] Test project list with multiple projects
- [ ] Test empty project list

**Estimated Time:** 0.1 day

**Milestone:** Project command complete

---

## Phase 7: Testing & Documentation

**Goal:** Comprehensive testing and usage documentation.

### 7.1 Integration Tests

**File:** `tests/integration_tests.rs` (new)

**Tasks:**
- [ ] Test full workflow:
  1. `agtrace scan --all-projects`
  2. `agtrace list`
  3. `agtrace view <session-id>`
  4. `agtrace project list`
- [ ] Test with real sample logs (Claude, Codex, Gemini)
- [ ] Test edge cases:
  - Empty database
  - Corrupted log files
  - Missing session files

**Estimated Time:** 0.5 day

### 7.2 Update README

**File:** `README.md` (modify)

**Tasks:**
- [ ] Update installation instructions
- [ ] Update usage examples for v2.0
- [ ] Add migration guide from v1.x
- [ ] Update architecture section

**Estimated Time:** 0.3 day

### 7.3 Manual Testing

**Tasks:**
- [ ] Test on macOS with real logs
- [ ] Test on Linux (optional, via CI)
- [ ] Test edge cases manually
- [ ] Verify performance targets

**Estimated Time:** 0.5 day

**Milestone:** Testing complete

---

## Phase 8: Polish & Release

**Goal:** Final touches and release preparation.

### 8.1 Code Cleanup

**Tasks:**
- [ ] Remove unused v1.x code (or mark deprecated)
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt`
- [ ] Update `CHANGELOG.md`

**Estimated Time:** 0.3 day

### 8.2 Performance Benchmarks

**Tasks:**
- [ ] Benchmark scan performance (target: 100 sessions < 1s)
- [ ] Benchmark list performance (target: 1000 sessions < 10ms)
- [ ] Benchmark view performance (target: 1000 events < 100ms)

**Estimated Time:** 0.2 day

### 8.3 Release

**Tasks:**
- [ ] Tag release: `v2.0.0`
- [ ] Update `Cargo.toml` version
- [ ] Merge feature branch to `main`
- [ ] Publish release notes

**Estimated Time:** 0.1 day

**Milestone:** v2.0 released

---

## Summary

### Total Estimated Time

| Phase | Estimated Time |
|-------|----------------|
| Phase 0: Preparation | 0.5 day |
| Phase 1: Database Foundation | 1.6 days |
| Phase 2: Scanner Implementation | 2.7 days |
| Phase 3: List Command | 0.5 day |
| Phase 4: View Command | 1.8 days |
| Phase 5: CLI Integration | 0.7 day |
| Phase 6: Project Command | 0.4 day |
| Phase 7: Testing & Documentation | 1.3 days |
| Phase 8: Polish & Release | 0.6 day |
| **Total** | **10.1 days** |

**Realistic Estimate with Buffer:** 12-15 days (2-3 weeks)

---

## Risk Mitigation

### Known Risks

1. **Provider-specific parsing complexity**
   - Mitigation: Reuse existing mapper logic from v1.x

2. **SQLite schema migration**
   - Mitigation: Use `PRAGMA user_version` for future migrations

3. **Performance bottlenecks in view**
   - Mitigation: Implement caching layer in Phase 9 (post-MVP)

4. **Backward compatibility with v1.x**
   - Mitigation: Keep old code commented out, provide migration script

---

## Post-MVP (Phase 9+)

### Future Features

- [ ] `agtrace find` - Full-text search
- [ ] `agtrace stats` - Statistics aggregation
- [ ] `agtrace export` - JSONL/CSV export
- [ ] Event caching (store normalized events in DB)
- [ ] Incremental scan optimization
- [ ] Watch mode (auto-scan on file changes)
- [ ] Web UI (`agtrace serve`)

---

This roadmap provides a clear path to implementing the Pointer Edition architecture while maintaining code quality and testability.
