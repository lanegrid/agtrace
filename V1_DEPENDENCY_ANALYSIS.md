# V1 Schema Dependency Analysis

Generated: 2025-12-14

## Summary

This document maps all remaining dependencies on `AgentEventV1` (v1 schema) and provides migration paths to v2 schema.

## Build Error Summary

Total files with v1 dependencies: **11 files**

- **agtrace-types**: 1 file (definition location)
- **agtrace-engine**: 2 files
- **agtrace-index**: 1 file
- **agtrace-providers**: 6 files
- **agtrace-cli**: 4 files (from earlier grep, not yet compiled)

## Detailed Dependency Map

### 1. agtrace-engine (2 files)

#### `crates/agtrace-engine/src/analysis.rs`
- **Import**: `use agtrace_types::{AgentEventV1, EventType};`
- **Usage**: `pub fn analyze(session_id: String, events: &[AgentEventV1], detectors: Vec<Detector>)`
- **Purpose**: Pattern detection for agent behavior analysis
- **Functions using AgentEventV1**:
  - `analyze()` - main analysis entry point
  - `detect_loops()` - loop detection
  - `detect_apologies()` - apology pattern detection
  - `detect_lazy_tools()` - lazy tool usage detection
  - `detect_zombie_chains()` - zombie chain detection
  - `detect_lint_ping_pong()` - lint ping-pong detection
  - `analyze_tool_usage()` - tool usage analysis

**V2 Replacement Strategy**:
- Create `convert_v2_to_v1()` adapter function temporarily
- OR: Rewrite analysis logic to work directly with v2 events
- V2 has better tool tracking with `tool_call_id`, should improve accuracy

#### `crates/agtrace-engine/src/export.rs`
- **Import**: `use agtrace_types::{AgentEventV1, EventType};`
- **Usage**: `pub fn transform(events: &[AgentEventV1], strategy: ExportStrategy) -> Vec<AgentEventV1>`
- **Purpose**: Transform events for export (raw, clean, reasoning)
- **Functions using AgentEventV1**:
  - `transform()` - main export entry point
  - `apply_clean_strategy()` - remove failed attempts
  - `apply_reasoning_strategy()` - extract reasoning pairs

**V2 Replacement Strategy**:
- Create `convert_v2_to_v1()` adapter function temporarily
- OR: Rewrite export logic to work directly with v2 events
- V2 has cleaner event types, should be easier to filter

---

### 2. agtrace-index (1 file - DEPRECATED)

#### `crates/agtrace-index/src/storage.rs`
- **Status**: ALREADY DEPRECATED (marked since 2.0.0)
- **Import**: `use agtrace_types::*;` (includes AgentEventV1)
- **Purpose**: Old file-based storage (replaced by SQLite)
- **8 error locations**:
  1. `save_events(&self, events: &[AgentEventV1])` - line 43
  2. `HashMap<(String, String), Vec<&AgentEventV1>>` - line 45
  3. `Vec<AgentEventV1>` - line 70
  4. `load_session_events() -> Result<Vec<AgentEventV1>>` - line 108
  5. `load_all_events() -> Result<Vec<AgentEventV1>>` - line 210
  6. `read_jsonl_file() -> Result<Vec<AgentEventV1>>` - line 266
  7. `let event: AgentEventV1` - line 276
  8. `write_jsonl_file(events: &[AgentEventV1])` - line 284

**V2 Replacement Strategy**:
- **DELETE THIS FILE** - it's already deprecated
- SQLite index should handle v2 natively
- No migration needed, just removal

---

### 3. agtrace-providers (6 files)

#### `crates/agtrace-providers/src/claude/io.rs`
- **Import**: `use agtrace_types::AgentEventV1;`
- **Function**: `pub fn normalize_claude_file(...) -> Result<Vec<AgentEventV1>>`
- **Purpose**: Entry point for Claude file normalization (v1 version)
- **Note**: `normalize_claude_file_v2()` already exists!

**V2 Replacement Strategy**:
- **DELETE** `normalize_claude_file()` function
- Already replaced by `normalize_claude_file_v2()`
- Update callers to use v2 version

#### `crates/agtrace-providers/src/claude/mapper.rs`
- **Function**: `pub(crate) fn normalize_claude_stream(...) -> Vec<AgentEventV1>`
- **6 error locations**: All `AgentEventV1::new()` calls
  - line 116: user message event
  - line 181: assistant message event
  - line 227: reasoning event
  - line 252: tool_call event
  - line 305: tool_result event
  - line 342: meta event

**V2 Replacement Strategy**:
- **DELETE** this entire function (replaced by v2 normalization)
- Already replaced by functions in `crates/agtrace-providers/src/v2/claude.rs`

#### `crates/agtrace-providers/src/codex/io.rs`
- **Import**: `use agtrace_types::AgentEventV1;`
- **Function**: `pub fn normalize_codex_file(...) -> Result<Vec<AgentEventV1>>`
- **Note**: `normalize_codex_file_v2()` already exists!

**V2 Replacement Strategy**:
- **DELETE** `normalize_codex_file()` function
- Already replaced by `normalize_codex_file_v2()`

#### `crates/agtrace-providers/src/codex/mapper.rs`
- **Function**: `pub(crate) fn normalize_codex_stream(...) -> Vec<AgentEventV1>`
- **1 error location**: `AgentEventV1::new()` at line 57

**V2 Replacement Strategy**:
- **DELETE** this entire function (replaced by v2 normalization)
- Already replaced by functions in `crates/agtrace-providers/src/v2/codex.rs`

#### `crates/agtrace-providers/src/gemini/io.rs`
- **Import**: `use agtrace_types::AgentEventV1;`
- **Function**: `pub fn normalize_gemini_file(...) -> Result<Vec<AgentEventV1>>`
- **Note**: `normalize_gemini_file_v2()` already exists!

**V2 Replacement Strategy**:
- **DELETE** `normalize_gemini_file()` function
- Already replaced by `normalize_gemini_file_v2()`

#### `crates/agtrace-providers/src/gemini/mapper.rs`
- **Function**: `pub(crate) fn normalize_gemini_session(...) -> Vec<AgentEventV1>`
- **5 error locations**: All `AgentEventV1::new()` calls
  - line 43: user message
  - line 65: assistant message
  - line 91: reasoning event
  - line 109: tool event
  - line 181: meta event

**V2 Replacement Strategy**:
- **DELETE** this entire function (replaced by v2 normalization)
- Already replaced by functions in `crates/agtrace-providers/src/v2/gemini.rs`

---

### 4. agtrace-cli (4 files - from earlier grep)

#### `crates/agtrace-cli/src/output/timeline.rs`
- **Import**: `use agtrace_types::{AgentEventV1, EventType, FileOp};`
- **Functions**:
  - `summarize_v1_events(events: &[AgentEventV1])`
  - `print_events_timeline(events: &[AgentEventV1], ...)`
  - `print_session_summary(events: &[AgentEventV1], ...)`
- **Purpose**: Timeline output display

**V2 Replacement Strategy**:
- **Option 1**: Create `convert_v2_to_v1()` adapter (temporary)
- **Option 2**: Rewrite timeline to use v2 events directly
- Prefer Option 2 for cleaner code

#### `crates/agtrace-cli/src/handlers/session_show.rs`
- **Usage**: Likely calls timeline.rs functions
- **Purpose**: Session display handler

**V2 Replacement Strategy**:
- Update to call v2-compatible timeline functions
- Should be straightforward once timeline.rs is migrated

#### `crates/agtrace-cli/src/handlers/lab_export.rs`
- **Usage**: Likely uses engine/export.rs
- **Purpose**: Lab export handler

**V2 Replacement Strategy**:
- Update to work with v2 events
- May need v2-compatible export transforms

#### `crates/agtrace-cli/src/handlers/lab_analyze.rs`
- **Usage**: Likely uses engine/analysis.rs
- **Purpose**: Lab analyze handler

**V2 Replacement Strategy**:
- Update to work with v2 events
- May need v2-compatible analysis functions

---

## Migration Priority

### Phase 1: Delete Dead Code (Easy Wins)
1. ✅ **Delete** `crates/agtrace-index/src/storage.rs` - already deprecated
2. ✅ **Delete** `crates/agtrace-providers/src/*/mapper.rs` - replaced by v2
3. ✅ **Delete** v1 normalize functions from `io.rs` files - replaced by v2

### Phase 2: Create V2→V1 Adapter (Temporary Bridge)
Create `crates/agtrace-engine/src/convert.rs`:
```rust
pub fn convert_v2_to_v1(events: &[v2::AgentEvent]) -> Vec<AgentEventV1> {
    // Convert v2 events back to v1 for legacy code
}
```

### Phase 3: Migrate Analysis & Export (Medium Effort)
1. Update `analysis.rs` to accept `&[v2::AgentEvent]` OR use adapter
2. Update `export.rs` to accept `&[v2::AgentEvent]` OR use adapter

### Phase 4: Migrate Timeline Display (Low Effort)
1. Update `timeline.rs` to work with v2 events directly
2. Remove v1 conversion layer

### Phase 5: Remove AgentEventV1 (Final Cleanup)
1. Uncomment the deletion in `lib.rs`
2. Remove all adapter code
3. Full v2 migration complete

---

## V2 Advantages for Each Use Case

### Analysis (engine/analysis.rs)
- **Better tool tracking**: `tool_call_id` as UUID for exact matching
- **Cleaner event types**: EventPayload enum vs flat structure
- **More accurate**: No ambiguous fields, explicit relationships

### Export (engine/export.rs)
- **Simpler filtering**: Event types are explicit (User, Message, ToolCall, etc.)
- **Better reasoning extraction**: Separate Reasoning event type
- **Token handling**: TokenUsage as sidecar, easy to filter

### Timeline (cli/output/timeline.rs)
- **Cleaner display logic**: Payload variants map directly to display types
- **Better time tracking**: parent_id chain for chronology
- **Token attribution**: Clear separation via TokenUsage events

---

## Recommended Next Steps

1. **Immediate**: Delete mapper.rs files and deprecated storage.rs
2. **Short-term**: Create v2→v1 adapter for analysis/export/timeline
3. **Medium-term**: Rewrite analysis/export to use v2 natively
4. **Long-term**: Remove AgentEventV1 entirely

---

## Files to Delete

```bash
# Deprecated storage (already marked deprecated)
rm crates/agtrace-index/src/storage.rs

# V1 mappers (replaced by v2 normalization)
rm crates/agtrace-providers/src/claude/mapper.rs
rm crates/agtrace-providers/src/codex/mapper.rs
rm crates/agtrace-providers/src/gemini/mapper.rs
```

## Functions to Delete from io.rs files

- `crates/agtrace-providers/src/claude/io.rs::normalize_claude_file()`
- `crates/agtrace-providers/src/codex/io.rs::normalize_codex_file()`
- `crates/agtrace-providers/src/gemini/io.rs::normalize_gemini_file()`

---

## Summary Statistics

- **Total v1 references**: ~30 locations across 11 files
- **Can be deleted immediately**: ~20 locations (mappers + storage)
- **Need migration/adapter**: ~10 locations (analysis, export, timeline)
- **Estimated effort**:
  - Delete phase: 30 minutes
  - Adapter creation: 1-2 hours
  - Native v2 migration: 3-4 hours
  - **Total**: ~5-6 hours for complete v1 removal
