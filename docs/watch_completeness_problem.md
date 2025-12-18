# Watch Command Completeness Problem

## Context

agtrace is a CLI tool that normalizes and analyzes agent behavior logs from multiple providers (Claude Code, Codex, Gemini). Sessions can consist of multiple log files:

- **Main file**: Primary conversation (e.g., `session1.jsonl`)
- **Sidechain files**: Background agent logs (e.g., `agent-0c4c3cf8.jsonl`)
- Both share the same `session_id` and belong to one logical session

**Design Principle**: We recently adopted a "completeness-first" approach where sessions always include ALL files (main + sidechain) to prevent information loss.

## Current Implementation Status

### ✅ Working Commands (Complete)
All commands using `SessionLoader` properly handle multi-file sessions:
- `session show` - displays all events from main + sidechain
- `session list` - indexes all files per session
- `pack` - analyzes across all session files
- `lab export` - exports unified session data

**Implementation**: Uses `Database.get_session_files(session_id)` which returns all files belonging to a session.

### ❌ Broken Command (Incomplete)
`watch` command only monitors **a single file**, violating completeness:

```rust
// crates/agtrace-runtime/src/streaming/watcher.rs:64
let mut current_file: Option<PathBuf> = None;  // Only ONE file

// watcher.rs:305-320
fn load_and_detect_changes(path: &Path, ...) -> Result<...> {
    let all_events = provider.normalize_file(path, &context)?;  // Single file only
    //...
}
```

**Impact**: When watching a session, sidechain file updates are invisible. User sees incomplete live data.

## Technical Architecture

### Current Runtime Design
`agtrace-runtime` (watch infrastructure) is intentionally **database-independent**:

```
┌─────────────────┐
│ agtrace-cli     │  (knows Database)
│   handlers/     │
└────────┬────────┘
         │
         v
┌─────────────────┐
│ agtrace-runtime │  (NO database dependency)
│   SessionWatcher│
└────────┬────────┘
         │
         v
┌─────────────────┐
│ agtrace-        │
│   providers     │  (file normalization only)
└─────────────────┘
```

**Current watch flow**:
1. User: `agtrace watch`
2. Runtime: Pick ONE file via `find_active_target()` (heuristic: most recently modified)
3. Watch filesystem events on that ONE file
4. Normalize & display updates from that ONE file only

## Problem Statement

**How do we make `watch` command session-aware (monitor ALL files) while respecting the architecture?**

## Solution Options

### Option A: Database Integration
**Approach**: Make Runtime depend on Database to resolve session files.

```rust
// Pseudo-code
pub struct SessionWatcher {
    db: Arc<Database>,  // NEW dependency
    current_session_id: Option<String>,
    watched_files: HashMap<PathBuf, usize>,  // Track multiple files
}

fn handle_fs_event(...) {
    // On file change:
    if let Some(session_id) = extract_session_id(path) {
        // Reload ALL files for this session from DB
        let files = db.get_session_files(&session_id)?;
        for file in files {
            let events = normalize_file(&file.path)?;
            // Merge and display...
        }
    }
}
```

**Pros**:
- ✅ Leverages existing indexed metadata
- ✅ Reliable (database already tracks session → files mapping)
- ✅ Consistent with `SessionLoader` approach

**Cons**:
- ❌ Breaks architecture: Runtime gains database dependency
- ❌ Requires index to be up-to-date (user must run `index update` first)
- ❌ Won't detect new sidechain files until re-indexed

**Dependency Impact**:
```
agtrace-runtime
  ├─ agtrace-providers (already exists)
  └─ agtrace-index (NEW - introduces circular-ish dependency concerns)
```

### Option B: Provider-Level Scanning
**Approach**: Add `fn get_session_files(log_root, session_id) -> Vec<PathBuf>` to provider trait.

```rust
// agtrace-providers/src/lib.rs
pub trait LogProvider {
    fn can_handle(&self, path: &Path) -> bool;
    fn normalize_file(&self, path: &Path) -> Result<Vec<AgentEvent>>;

    // NEW method
    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>>;
}

// Runtime uses this
fn handle_fs_event(...) {
    if let Some(session_id) = extract_session_id(path) {
        // Dynamically scan for all files with this session_id
        let files = provider.find_session_files(&log_root, &session_id)?;
        for file in files {
            // Normalize and merge...
        }
    }
}
```

**Pros**:
- ✅ No database dependency (preserves architecture)
- ✅ Works without indexing (live discovery)
- ✅ Detects new sidechain files immediately

**Cons**:
- ❌ Repeated filesystem scans (performance cost)
- ❌ Duplicates logic already in `provider.scan()` for indexing
- ❌ Adds complexity to provider trait

**Performance**:
- Claude provider: Scan `~/.claude/projects/-encoded-dir/**/*.jsonl` on every file change
- Typical: 10-100 files per project directory
- Cost: ~10ms per scan (acceptable for 500ms poll interval)

### Option C: Hybrid (Fallback Strategy)
**Approach**: Try database first, fall back to scanning if unavailable.

```rust
fn resolve_session_files(session_id: &str) -> Vec<PathBuf> {
    if let Some(db) = optional_db {
        db.get_session_files(session_id)  // Fast path
    } else {
        provider.find_session_files(log_root, session_id)  // Slow path
    }
}
```

**Pros**:
- ✅ Best of both worlds (fast when indexed, works without index)

**Cons**:
- ❌ Most complex implementation
- ❌ Still requires database dependency OR new provider method
- ❌ Two code paths to maintain

## Decision Criteria

| Criterion | Option A (Database) | Option B (Scan) | Option C (Hybrid) |
|-----------|---------------------|-----------------|-------------------|
| **Architectural purity** | ⚠️ Violates separation | ✅ Clean | ⚠️ Complex |
| **Performance** | ✅ O(1) lookup | ⚠️ O(n) scan | ✅/⚠️ Depends |
| **Reliability** | ⚠️ Needs up-to-date index | ✅ Always live | ✅ Redundant |
| **Works without index** | ❌ No | ✅ Yes | ✅ Yes |
| **Implementation complexity** | 🟡 Medium | 🟡 Medium | 🔴 High |
| **Detects new files immediately** | ❌ No | ✅ Yes | ✅ Yes |

## Questions for Discussion

1. **Architecture philosophy**: Is it acceptable for `agtrace-runtime` to depend on `agtrace-index`? Or should runtime remain a pure "file watcher + normalizer" layer?

2. **Performance priority**: Is repeated scanning (Option B) acceptable for watch mode? Typical projects have <100 files in log directories.

3. **Index dependency**: Should `watch` require users to run `index update` first (Option A)? This breaks the "works out of the box" property.

4. **Future extensibility**: If we support more multi-file session patterns (e.g., Codex subagents), which option scales better?

5. **Edge case**: What happens when a sidechain file appears AFTER initial attach?
   - Option A: User must re-run `index update` + restart watch
   - Option B: Detected automatically on next file event
   - Option C: Graceful degradation

## Recommendation Needed

**Which option should we implement, and why?**

Please consider:
- Long-term maintainability
- User experience (what would a developer expect from `agtrace watch`?)
- Consistency with other commands
- Performance characteristics for typical usage (1-10 active sessions, 5-50 files per session)

---

## Additional Context

- **Scan implementation already exists**: `ClaudeProvider::scan()` in `agtrace-providers` already walks directories and groups files by `session_id`. We could extract this logic into `find_session_files()`.

- **Database schema supports this**: The `log_files` table already has `session_id` foreign key. Query is trivial: `SELECT path FROM log_files WHERE session_id = ?`.

- **Watch is real-time critical**: Poll interval is 500ms. Any solution must complete session file resolution + normalization within ~100ms to avoid lag.

- **Typical Claude session**: 1 main file + 0-3 sidechain files. Most sessions have NO sidechain files, so optimization for the common case matters.
