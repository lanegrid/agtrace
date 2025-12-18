# Session Completeness Implementation Progress

## ✅ Completed (2025-12-19)

### Commit: `36c00ab`
**feat: implement session completeness with sidechain support and find_session_files trait method**

#### What was done:
1. **Provider Layer - find_session_files trait method**
   - Added `LogProvider::find_session_files(log_root, session_id) -> Vec<PathBuf>`
   - Implemented for ClaudeProvider, CodexProvider, GeminiProvider
   - Performance target: <10ms for ~100 files (uses walkdir + header extraction)

2. **Sidechain Completeness Across All Commands**
   - Removed `include_sidechain` option from `SessionLoader::LoadOptions`
   - Removed `agent-*.jsonl` filtering from `ClaudeProvider::can_handle()`
   - Removed sidechain filtering from `session show --raw` mode
   - All commands (show/list/pack/lab export) now include sidechain events

3. **Testing & Documentation**
   - Added `session_completeness_test.rs` - verifies 29 main + 3 sidechain events
   - Added test helper `copy_sample_file_to_project()` for Claude-encoded directories
   - Created `docs/watch_completeness_problem.md` with architectural analysis

#### Design Decision:
**Option B (Provider-Level Scanning)** was chosen over Database Integration because:
- Live sidechain discovery (detects files appearing during execution)
- No database dependency for runtime layer
- Acceptable performance cost (~5-10ms per poll)

---

## 🚧 Next Task: Runtime Layer - SessionWatcher Refactoring

### Goal
Make `watch` command session-aware by monitoring ALL files (main + sidechain) belonging to a session.

### Current Problem
**File:** `crates/agtrace-runtime/src/streaming/watcher.rs`

```rust
// Current: Single file tracking
let mut current_file: Option<PathBuf> = None;  // ❌ Only ONE file

// Current: Single file loading
fn load_and_detect_changes(path: &Path, ...) {
    provider.normalize_file(path, ...)  // ❌ Single file only
}
```

**Impact:** When watching a session, sidechain file updates are invisible.

### Implementation Plan

#### Step 1: Modify SessionWatcher State (watcher.rs:64)
**Before:**
```rust
let mut current_file: Option<PathBuf> = None;
let mut file_event_counts: HashMap<PathBuf, usize> = HashMap::new();
```

**After:**
```rust
let mut current_session_id: Option<String> = None;
let mut file_states: HashMap<PathBuf, FileState> = HashMap::new();

struct FileState {
    last_event_count: usize,
    last_size: u64,
}
```

#### Step 2: Update resolve_explicit_target() (watcher.rs:323)
**Extract session_id from the target file:**
```rust
fn resolve_explicit_target(...) -> Result<WatchTarget> {
    // ... existing file resolution ...

    // NEW: Extract session_id from the first file
    let session_id = extract_session_id_from_file(&path, &provider)?;

    Ok(WatchTarget::Session {
        session_id,
        initial_path: path
    })
}
```

#### Step 3: Implement Multi-File Discovery (watcher.rs:171-303)
**In handle_fs_event():**
```rust
fn handle_fs_event(...) -> Result<()> {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                if !provider.can_handle(path) { continue; }

                // Extract session_id from changed file
                let session_id = match extract_session_id_from_file(path, provider) {
                    Ok(id) => id,
                    Err(_) => continue,
                };

                // If it's the current session (or first session), discover all files
                if current_session_id.is_none() || current_session_id.as_ref() == Some(&session_id) {
                    current_session_id = Some(session_id.clone());

                    // DISCOVERY: Find ALL files for this session
                    let session_files = provider.find_session_files(&log_root, &session_id)?;

                    // LOAD: Process each file for new events
                    let mut all_new_events = Vec::new();
                    for file_path in session_files {
                        if let Ok((events, new_events)) = load_and_detect_changes(
                            &file_path,
                            file_states.get(&file_path).map(|s| s.last_event_count).unwrap_or(0),
                            provider
                        ) {
                            file_states.insert(file_path.clone(), FileState {
                                last_event_count: events.len(),
                                last_size: std::fs::metadata(&file_path)?.len(),
                            });
                            all_new_events.extend(new_events);
                        }
                    }

                    // SORT: Merge events by timestamp
                    all_new_events.sort_by_key(|e| e.timestamp.clone());

                    // SEND: Emit merged update
                    if !all_new_events.is_empty() {
                        let session = assemble_session_from_events(&all_new_events);
                        tx.send(StreamEvent::Update(SessionUpdate {
                            session,
                            new_events: all_new_events,
                            orphaned_events: vec![],
                            total_events: file_states.values().map(|s| s.last_event_count).sum(),
                        }))?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
```

#### Step 4: Add Helper Function
```rust
fn extract_session_id_from_file(path: &Path, provider: &Arc<dyn LogProvider>) -> Result<String> {
    // Provider-specific extraction (already exists for Claude/Codex/Gemini)
    if let Ok(header) = extract_claude_header(path) {
        return Ok(header.session_id.ok_or_else(|| anyhow!("No session_id"))?);
    }
    // ... similar for other providers
    Err(anyhow!("Could not extract session_id"))
}
```

### Testing Plan

1. **Unit Test:** Multi-file session discovery
   ```rust
   #[test]
   fn test_watcher_discovers_sidechain_files() {
       // Setup: Create main + sidechain files with same session_id
       // Start watcher on main file
       // Create sidechain file during watch
       // Assert: Both files are being monitored
   }
   ```

2. **Integration Test:** Use existing `watch_test.rs`
   - Modify `test_session_rotation_emits_attached_event` to include sidechain
   - Verify events from both main + sidechain are emitted

3. **Manual Test:**
   ```bash
   cargo build --release
   ./target/release/agtrace watch
   # In Claude Code: Trigger a task that spawns a sidechain agent
   # Verify: Sidechain events appear in watch output
   ```

### Files to Modify

- `crates/agtrace-runtime/src/streaming/watcher.rs` (~200 lines changed)
- `crates/agtrace-runtime/tests/watch_test.rs` (add multi-file test)

### Success Criteria

- ✅ Watch command displays events from all session files (main + sidechain)
- ✅ Sidechain files appearing mid-session are detected within 500ms
- ✅ Events from different files are correctly interleaved by timestamp
- ✅ All existing watch tests still pass
- ✅ Performance: <100ms total per poll cycle for session with 5 files

### Reference Documents

- **Architecture Analysis:** `docs/watch_completeness_problem.md`
- **Provider Implementation:** See `ClaudeProvider::find_session_files()` in `crates/agtrace-providers/src/claude/mod.rs:181`
- **Completeness Test:** `crates/agtrace-cli/tests/session_completeness_test.rs`

### Notes

- **Polling Interval:** 500ms (defined in `watcher.rs:72`)
- **Performance Budget:** Session file discovery should complete in <10ms to avoid blocking the poll loop
- **Edge Case:** Handle gracefully when a sidechain file is deleted mid-session (skip it, don't crash)
- **Sorting:** Events must be sorted by `timestamp` field, not file modification time
