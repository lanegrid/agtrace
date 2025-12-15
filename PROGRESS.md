# Progress: `agtrace watch` Architecture Refactoring

## Overview
Refactoring the `watch` command to separate concerns between file watching logic and UI presentation, following Producer-Consumer pattern for better testability and extensibility.

## Goals
1. **Separation of Concerns**: Decouple file watching from UI rendering
2. **Testability**: Make `SessionWatcher` unit-testable
3. **Extensibility**: Enable future TUI, JSON output, or WebSocket modes
4. **Reusability**: Centralize session detection and streaming logic

## Architecture Design

```
┌─────────────────────────────────────────────────────────────┐
│ handlers/watch.rs (UI Layer)                                │
│ - Parse CLI args                                             │
│ - Format and print events                                    │
│ - Handle StreamEvent display                                 │
└───────────────────┬─────────────────────────────────────────┘
                    │ StreamEvent (channel)
                    ↓
┌─────────────────────────────────────────────────────────────┐
│ streaming/watcher.rs (Business Logic Layer)                 │
│ - SessionWatcher: File system monitoring                     │
│ - Latest session detection                                   │
│ - File offset tracking                                       │
│ - Event emission (Attached, NewEvents, SessionRotated)      │
└───────────────────┬─────────────────────────────────────────┘
                    │ parse_line()
                    ↓
┌─────────────────────────────────────────────────────────────┐
│ agtrace-providers (Provider Layer)                          │
│ - LogProvider::parse_line() - Stream-friendly API           │
│ - Per-line parsing for real-time processing                 │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Foundation (streaming module) ✅
- [x] Rewrite PROGRESS.md with new plan
- [x] Create `streaming/mod.rs` module
- [x] Define `StreamEvent` enum
  - `Attached { path, session_id }`
  - `NewEvents(Vec<AgentEvent>)`
  - `Error(String)`
  - `SessionRotated { old_path, new_path }`
- [x] Create `SessionWatcher` struct skeleton

### Phase 2: Provider Extension ✅
- [x] Add `parse_line(&self, line: &str) -> Result<Option<AgentEvent>>` to LogProvider trait
- [x] Implement default for v2 schema (direct JSONL parsing)
- [x] Handle malformed lines gracefully (return None)

### Phase 3: SessionWatcher Implementation ✅
- [x] Move file watching logic from handlers/watch.rs
- [x] Implement initial session detection (in `new()`)
- [x] Implement event loop with notify integration
- [x] Handle Create/Modify events with session rotation logic
- [x] File offset management for incremental reads
- [x] Thread-based event emission

### Phase 4: Handler Refactoring ✅
- [x] Simplify `handlers/watch.rs` to only handle display
- [x] Replace direct notify usage with SessionWatcher
- [x] Keep existing `print_event()` formatting logic
- [x] Update to receive StreamEvent from channel

### Phase 5: Testing & Polish ✅
- [x] Build and fix compilation errors
- [x] Manual testing with real sessions - verified auto-attach
- [x] Verify auto-attach and session rotation
- [x] Run fmt and clippy - all warnings fixed
- [x] Commit refactoring

## Key Design Decisions

### 1. Session Detection Timing
- **Initialization**: `SessionWatcher::new()` scans for latest file
- **Runtime**: On `EventKind::Create`, check if newer than current

### 2. Thread Model
- Main thread: UI rendering (blocks on channel receive)
- Worker thread: File watching and parsing (sends to channel)

### 3. Error Handling
- Parse errors: Silent skip (incomplete writes), emit Error event for serious issues
- File access errors: Emit Error event, continue watching

### 4. Provider Integration
- V2 schema: Direct `serde_json::from_str()` in parse_line
- Future: Provider-specific normalization for Claude/Codex raw formats

## Benefits of This Architecture

1. **UI Swappable**: handlers/watch.rs can be replaced with TUI or JSON formatter
2. **Unit Testable**: SessionWatcher can be tested with mock file systems
3. **Provider Agnostic**: Works with any LogProvider implementation
4. **Performance**: Thread-based design prevents UI blocking
5. **Maintainable**: Clear boundaries between concerns

## Known Limitations (Post-Refactor)
- Still expects v2 schema JSONL format
- Provider parse_line() implementation needed for raw formats
  - Requires passing provider instance to SessionWatcher (architectural change)
- No automatic reconnection if watcher fails

## Post-Refactor Improvements (Review Feedback)

### Error Handling Enhancement
- ✅ Worker thread panic handling with `std::panic::catch_unwind`
- ✅ Fatal error detection and graceful termination
- ✅ Channel disconnection detection with user-friendly messages
- ✅ Named worker threads for better debugging

### Provider Documentation
- ✅ Comprehensive parse_line documentation
- ✅ Clear distinction between v2 JSONL and raw format support
- ✅ Future extension path documented

## Future Enhancements
- TUI mode with ratatui
- JSON output mode for scripting
- Multi-session parallel watching
- Session filtering by project
- Provider-specific raw format support in watch mode

---

# Progress: Smart Session Detection with Liveness Window

## Overview
Improving `agtrace watch` session detection to avoid UX issues:
- **Problem 1**: Attaching to "dead" sessions (last updated days ago)
- **Problem 2**: Confusing behavior with multiple concurrent sessions
- **Solution**: Implement Liveness Window detection with smart fallback

## Implementation Plan

### Phase 1: Core Liveness Logic ✅
- [x] Define `SessionLiveness` enum (Hot/Warm/Cold/None)
- [x] Replace `find_latest_log_file` with `find_active_target`
- [x] Implement 5-minute window for "Hot Active" sessions
- [x] Add waiting mode for "Cold Dead" sessions
- [x] Multi-session warning when multiple hot sessions exist

### Phase 2: CLI Extension ✅
- [x] Add `--id <SESSION_ID>` option to watch command
- [x] Support explicit file path specification
- [x] Update handler to respect explicit session selection

### Phase 3: Testing & Polish ✅
- [x] Build and fix compilation errors
- [x] Run fmt and clippy
- [x] Manual testing scenarios:
  - One hot session (auto-attach) ✅
  - Explicit --id override ✅
  - Nonexistent session error handling ✅
- [x] Commit with one-line message

## Implementation Summary

Successfully implemented smart session detection with liveness window:
- **Liveness Detection**: 5-minute threshold separates "hot" active sessions from "cold" dead ones
- **Waiting Mode**: Gracefully handles no active sessions with informative message
- **Multi-Session Warning**: Alerts when multiple sessions are active simultaneously
- **Explicit Override**: `--id` option allows bypassing auto-detection
- **Flexible Resolution**: Supports both session ID prefix matching and direct file paths

Commit: `08913e5` - feat: add liveness window detection and explicit session selection to watch command

## Design Details

### Liveness Thresholds
- **Hot Active**: Last modified within 5 minutes
- **Warm Idle**: Last modified within 1 hour (future enhancement)
- **Cold Dead**: Older than 1 hour

### Behavior Matrix
| Scenario | Behavior |
|----------|----------|
| No files | Wait mode: "Waiting for new session..." |
| Only cold files | Wait mode: "No active sessions (last: 2 days ago)" |
| One hot file | Auto-attach |
| Multiple hot files | Auto-attach to latest + warning |
| --id specified | Force attach (skip liveness check) |

### Return Type
```rust
enum WatchTarget {
    File { path: PathBuf, offset: u64 },
    Waiting { reason: String },
}
```

---

# Progress: Context-Aware Watch (Project Isolation)

## Overview
Enhancing `agtrace watch` to understand project context - only monitoring and displaying logs related to the current working directory's project, avoiding noise from other concurrent projects.

## Problem Statement
Current `watch` command monitors all sessions in the log directory without filtering:
- **Issue 1**: Switching to unrelated sessions when working in different projects
- **Issue 2**: Noise from concurrent agent sessions in other projects
- **Issue 3**: No intuitive "watch this project" behavior

## Solution: Content-Aware Filtering
Implement project context detection that reads log file headers to check if they belong to the current project:
- **Claude Code**: Check `cwd` field in log files
- **Gemini**: Check `project_hash` field in logs.json
- **Codex**: Check `cwd` field in session files

## Implementation Plan

### Phase 1: Project Context Detection ✅
- [x] Add project context detection to `handlers/watch.rs`
- [x] Use existing `discover_project_root()` from agtrace-types
- [x] Get LogProvider instance via `create_provider()`
- [x] Pass provider and project_root to SessionWatcher

### Phase 2: Content-Aware File Filtering ✅
- [x] Refactor to use `LogProvider::belongs_to_project()` (no custom implementation)
- [x] Update `SessionWatcher::new()` to accept `Arc<dyn LogProvider>` and `Option<PathBuf>`
- [x] Update `find_active_target()` to call `provider.belongs_to_project()`
- [x] Update `handle_fs_event()` to filter new files via provider
- [x] Remove duplicate/custom project filtering logic

### Phase 3: Testing & Validation ⏳
- [ ] Test Case 1: Single project auto-attach
- [ ] Test Case 2: Multi-project isolation (Case 6 from requirements)
- [ ] Test Case 3: Explicit --id override still works
- [x] Build, fmt, clippy
- [ ] Commit with one-line message

## Implementation Summary

Successfully implemented context-aware watch using existing infrastructure:

### Architecture
```
handlers/watch.rs
  ├─ discover_project_root() → PathBuf
  ├─ create_provider(name) → Arc<dyn LogProvider>
  └─ SessionWatcher::new(log_root, provider, target, project_root)
       │
       ├─ find_active_target() → filters via provider.belongs_to_project()
       └─ handle_fs_event() → filters new files via provider.belongs_to_project()
```

### Key Implementation Details

1. **Leverages Existing Code**:
   - Uses `LogProvider::belongs_to_project()` from agtrace-providers
   - No duplicate implementation of header parsing
   - Claude: Uses `extract_cwd_from_claude_file()` internally
   - Gemini: Uses `extract_project_hash_from_gemini_file()` internally

2. **Provider Instance Management**:
   - Provider inferred from log_root path (`~/.claude`, `~/.codex`, `~/.gemini`)
   - Converted to `Arc<dyn LogProvider>` for thread-safe sharing
   - Passed to worker thread for session rotation filtering

3. **Project Filtering Behavior**:
   - With `--id`: No filtering (explicit override)
   - Without `--id` + cwd determinable: Filter to current project
   - Without `--id` + cwd not determinable: Watch all (fallback)

## Key Design Decisions

### 1. Reuse Existing Provider Infrastructure
- **No Custom Implementation**: Removed custom `is_file_for_project()` in favor of `LogProvider::belongs_to_project()`
- **Provider-Agnostic**: Works with all providers (Claude, Codex, Gemini) without special casing
- **Maintainability**: Single source of truth for project membership logic

### 2. Provider Detection
Infer provider from log root path:
- `~/.claude/projects/`: Claude Code
- `~/.gemini/sessions/`: Gemini
- `~/.codex/sessions/`: Codex

### 3. Backwards Compatibility
- Explicit `--id` flag bypasses project filtering (existing behavior)
- `--all-projects` flag (future) could disable filtering
- No changes to v2 schema or provider normalization

## Expected Behavior Changes

| Scenario | Before | After |
|----------|--------|-------|
| `watch` in Proj-A | Attaches to any latest session | Only attaches to Proj-A sessions |
| New session in Proj-B while watching Proj-A | Switches to Proj-B | Ignores Proj-B, stays on Proj-A |
| `watch --id <session>` | Attaches to session | Same (bypasses filter) |

## Out of Scope (Future Work)
- `--all-projects` flag to disable filtering
- Cross-project session comparison view

---

# Issue: Watch Mode Event Display Not Working

## Problem Discovery (2025-12-16)

Watch command detects file modifications but doesn't display parsed events:
```
[DEBUG] FS Event: Modify(Data(Content)) | Paths: [".../242c8d90.jsonl"]
[DEBUG] FS Event: Modify(Data(Content)) | Paths: [".../242c8d90.jsonl"]
# ... but no actual AgentEvent output
```

## Root Cause Analysis

### Current Implementation
```rust
process_new_events(path, offset, provider) {
    // Reads lines from offset
    provider.parse_line(line)  // ← This is the problem
}
```

### The Issue
1. **`LogProvider::parse_line()` default implementation assumes v2 JSONL**:
   ```rust
   fn parse_line(&self, line: &str) -> Result<Option<AgentEvent>> {
       serde_json::from_str::<AgentEvent>(line)  // Direct v2 parse
   }
   ```

2. **Claude Code logs are raw `ClaudeRecord` format**, not v2:
   ```json
   {"type":"user","uuid":"...","sessionId":"...","message":{...}}
   {"type":"assistant","uuid":"...","sessionId":"...","message":{...}}
   ```

3. **Normalization requires full session context**:
   - `EventBuilder` needs to track tool_call mappings across records
   - ToolResult events need to resolve `tool_use_id` → `tool_call_uuid`
   - Parent-child relationships require ordered traversal

### Why Line-by-Line Streaming is Hard
- **Stateful normalization**: `EventBuilder` maintains session-wide state
- **Cross-record dependencies**: ToolResult needs previous ToolCall's UUID
- **Multiple events per record**: One AssistantRecord → [Reasoning, ToolCall, Message, TokenUsage]

## Proposed Solution: File Re-Normalization Approach

### Design Decision
**Re-normalize the entire file on each modification** instead of incremental line parsing.

**Rationale**:
- Event frequency: ~10 seconds per event (low volume)
- File size: Typical sessions are < 1MB
- Simplicity: Reuse existing `normalize_file()` logic
- Correctness: Guaranteed consistency with batch import

### Implementation Plan

#### Phase 1: Offset-Based Duplicate Detection ✅ (Already Implemented)
Current code tracks file offset to skip already-processed content.

#### Phase 2: Event Deduplication Strategy
Two approaches:

**Option A: Event Count Tracking** (Simpler)
```rust
struct FileState {
    path: PathBuf,
    event_count: usize,  // Number of events already displayed
}

on_modify(path) {
    let all_events = provider.normalize_file(path)?;
    let state = file_states.get_mut(path);
    let new_events = &all_events[state.event_count..];

    display(new_events);
    state.event_count = all_events.len();
}
```

**Option B: Event ID Tracking** (More Robust)
```rust
struct FileState {
    path: PathBuf,
    seen_event_ids: HashSet<Uuid>,
}

on_modify(path) {
    let all_events = provider.normalize_file(path)?;
    let new_events: Vec<_> = all_events
        .into_iter()
        .filter(|e| !state.seen_event_ids.contains(&e.id))
        .collect();

    display(new_events);
    state.seen_event_ids.extend(new_events.iter().map(|e| e.id));
}
```

**Recommendation**: Start with **Option A** (event count), fallback to **Option B** if issues arise.

#### Phase 3: Implementation Changes

1. **Rename `process_new_events()` → `detect_new_events()`**
2. **Replace line-by-line parsing with full normalization**:
   ```rust
   fn detect_new_events(
       path: &Path,
       last_event_count: usize,
       provider: &Arc<dyn LogProvider>,
   ) -> Result<Vec<AgentEvent>> {
       let all_events = provider.normalize_file(path, &ImportContext::default())?;
       Ok(all_events.into_iter().skip(last_event_count).collect())
   }
   ```

3. **Update state tracking**:
   ```rust
   // Replace HashMap<PathBuf, u64> with HashMap<PathBuf, usize>
   let mut file_event_counts: HashMap<PathBuf, usize> = HashMap::new();
   ```

#### Phase 4: Testing & Validation
- [ ] Test with Claude Code sessions (primary use case)
- [ ] Test with rapid file updates (buffering behavior)
- [ ] Verify no duplicate events displayed
- [ ] Performance test with large sessions (> 100 events)

## Alternative Considered: Provider-Specific `parse_line()` Implementation

### Why We're NOT Doing This (For Now)

**Complexity**:
- Requires maintaining stateful parsers for each provider
- Claude: Need `EventBuilder` state for UUID mapping
- Gemini: Array format makes line-by-line parsing impossible
- Codex: Similar challenges

**Limited Benefit**:
- Event frequency is low (~10s intervals)
- File sizes are manageable (< 1MB typically)
- Full re-normalization is fast enough

**Future Work**:
If performance becomes an issue with very large sessions (> 1000 events):
- Implement incremental normalization with snapshot/checkpoint system
- Add `parse_line_stateful()` method with provider state management
- Consider separate streaming schema (v3) optimized for real-time

## Current Status
- [x] Project filtering implemented and working
- [x] File modification detection working
- [x] Debug logging added
- [ ] Event normalization in watch mode (blocked by this issue)
- [ ] Implement file re-normalization approach
