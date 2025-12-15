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
- [ ] Define `SessionLiveness` enum (Hot/Warm/Cold/None)
- [ ] Replace `find_latest_log_file` with `find_active_target`
- [ ] Implement 5-minute window for "Hot Active" sessions
- [ ] Add waiting mode for "Cold Dead" sessions
- [ ] Multi-session warning when multiple hot sessions exist

### Phase 2: CLI Extension
- [ ] Add `--id <SESSION_ID>` option to watch command
- [ ] Support explicit file path specification
- [ ] Update handler to respect explicit session selection

### Phase 3: Testing & Polish
- [ ] Build and fix compilation errors
- [ ] Run fmt and clippy
- [ ] Manual testing scenarios:
  - No sessions (waiting mode)
  - One hot session (auto-attach)
  - Multiple hot sessions (warning + latest)
  - Only cold sessions (waiting mode)
  - Explicit --id override
- [ ] Commit with one-line message

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
