# Schema v1 to v2 Migration Progress

## Current Status (2025-12-14)

✅ **100% Complete** - Full native v2 implementation, all v1 code removed

**What's Working**:
- All 3 providers (Claude, Codex, Gemini) normalize to v2
- CLI uses v2 pipeline exclusively
- Span engine uses v2 with O(1) tool matching
- Analysis works natively with v2 events (no adapter)
- Export works natively with v2 events (no adapter)
- Timeline display works directly with v2 events
- All v1 code deleted (mappers, converters, AgentEventV1 struct)
- 28 tests passing across all crates
- Zero clippy warnings
- **Proven improvements**: 50-150% better span accuracy, 363% better token tracking

---

## Remaining Tasks to Complete Migration

See `V1_DEPENDENCY_ANALYSIS.md` for detailed dependency map (11 files, 30 locations).

### Task 1: Delete Dead V1 Code ⏱️ 15 minutes

**What**: Remove v1 mapper files and deprecated storage (already unused, zero risk)

**Files to delete**:
```bash
# V1 mappers (replaced by v2 normalization in crates/agtrace-providers/src/v2/)
rm crates/agtrace-providers/src/claude/mapper.rs
rm crates/agtrace-providers/src/codex/mapper.rs
rm crates/agtrace-providers/src/gemini/mapper.rs

# Deprecated storage (marked deprecated since 2.0.0, replaced by SQLite)
rm crates/agtrace-index/src/storage.rs
```

**Functions to delete** from io.rs files:
- `crates/agtrace-providers/src/claude/io.rs::normalize_claude_file()` - replaced by `normalize_claude_file_v2()`
- `crates/agtrace-providers/src/codex/io.rs::normalize_codex_file()` - replaced by `normalize_codex_file_v2()`
- `crates/agtrace-providers/src/gemini/io.rs::normalize_gemini_file()` - replaced by `normalize_gemini_file_v2()`

**Verification**:
```bash
cargo build  # Should compile
cargo test   # All tests should pass
```

---

### Task 2: Migrate Analysis & Export ⏱️ 3-4 hours

**Option A: Quick Bridge (recommended for now)**

Create temporary v2→v1 adapter in `crates/agtrace-engine/src/convert.rs`:

```rust
use agtrace_types::{v2, AgentEventV1, EventType, Role, Channel, ToolStatus, Source};

/// Convert v2 events to v1 format for legacy analysis/export code
pub fn convert_v2_to_v1(events: &[v2::AgentEvent]) -> Vec<AgentEventV1> {
    let mut v1_events = Vec::new();

    for event in events {
        // Skip TokenUsage events (sidecar, not in v1)
        if matches!(event.payload, v2::EventPayload::TokenUsage(_)) {
            continue;
        }

        // Extract common fields
        let ts = event.timestamp.to_rfc3339();
        let session_id = Some(event.trace_id.to_string());
        let event_id = Some(event.id.to_string());
        let parent_event_id = event.parent_id.map(|id| id.to_string());

        // Convert based on payload type
        match &event.payload {
            v2::EventPayload::User(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(), // project_hash
                    ts,
                    EventType::UserMessage,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::User);
                ev.channel = Some(Channel::Chat);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::Reasoning(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::Reasoning,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::ToolCall(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::ToolCall,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.tool_name = Some(p.name.clone());
                ev.tool_call_id = Some(event.id.to_string());
                ev.text = Some(p.arguments.to_string());
                v1_events.push(ev);
            }
            v2::EventPayload::ToolResult(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::ToolResult,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Tool);
                ev.tool_call_id = Some(p.tool_call_id.to_string());
                ev.tool_status = Some(if p.is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Success
                });
                ev.text = Some(p.output.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::Message(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::AssistantMessage,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.channel = Some(Channel::Chat);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::TokenUsage(_) => {
                // Already filtered above, but handle explicitly
            }
        }
    }

    v1_events
}
```

Then update analysis.rs and export.rs to use adapter:

```rust
// In analysis.rs
pub fn analyze_v2(
    session_id: String,
    events: &[v2::AgentEvent],
    detectors: Vec<Detector>,
) -> AnalysisReport {
    let v1_events = crate::convert::convert_v2_to_v1(events);
    analyze(session_id, &v1_events, detectors)
}

// In export.rs
pub fn transform_v2(
    events: &[v2::AgentEvent],
    strategy: ExportStrategy,
) -> Vec<v2::AgentEvent> {
    let v1_events = crate::convert::convert_v2_to_v1(events);
    let transformed_v1 = transform(&v1_events, strategy);
    // Would need reverse conversion or just return original v2 events
    events.to_vec() // Placeholder
}
```

**Option B: Native V2 Rewrite (better long-term)**

Rewrite analysis.rs and export.rs to work directly with `&[v2::AgentEvent]`:
- Better tool tracking with UUID-based tool_call_id
- Cleaner event filtering with EventPayload enum
- More accurate analysis

See `V1_DEPENDENCY_ANALYSIS.md` section 1 for detailed migration strategies.

**Verification**:
```bash
cargo test --package agtrace-engine
```

---

### Task 3: Migrate Timeline Display ⏱️ 1-2 hours

**What**: Update `crates/agtrace-cli/src/output/timeline.rs` to work with v2 events

**Current**: Has local `convert_v2_to_v1()` adapter and works with AgentEventV1

**Target**: Work directly with v2::AgentEvent

**Steps**:
1. Update function signatures to accept `&[v2::AgentEvent]`
2. Update event type matching to use `EventPayload` enum
3. Update token collection to handle TokenUsage sidecar events
4. Update file operation extraction

**Example**:
```rust
pub fn print_events_timeline(events: &[v2::AgentEvent], truncate: bool, enable_color: bool) {
    for event in events {
        match &event.payload {
            v2::EventPayload::User(p) => {
                // Display user message
            }
            v2::EventPayload::ToolCall(p) => {
                // Display tool call
            }
            // ... etc
        }
    }
}
```

**Verification**:
```bash
cargo build --bin agtrace
./target/debug/agtrace show <session-id>  # Should display correctly
```

---

### Task 4: Final V1 Removal ⏱️ 30 minutes

**What**: Delete AgentEventV1 struct definition after all usage is removed

**Prerequisites**: Tasks 1-3 complete, no remaining references to AgentEventV1

**Steps**:
1. Verify no references:
```bash
rg "AgentEventV1" crates/ --type rust
# Should only show the definition in lib.rs
```

2. Delete the struct:
```rust
// In crates/agtrace-types/src/lib.rs
// DELETE lines 158-255 (AgentEventV1 struct and impl)
```

3. Clean up imports in all files

**Verification**:
```bash
cargo build
cargo test
cargo clippy
```

---

## Completion Criteria

**Phase 5 Status: ✅ 100% Complete**

- [x] All v1 mapper files deleted (Task 1) ✅
- [x] All v1 normalize functions deleted (Task 1) ✅
- [x] Deprecated storage.rs deleted (Task 1) ✅
- [x] Analysis works natively with v2 (Task 2 - Option B) ✅
- [x] Export works natively with v2 (Task 2 - Option B) ✅
- [x] Timeline display works with v2 natively (Task 3) ✅
- [x] v2→v1 adapter (convert.rs) deleted ✅
- [x] AgentEventV1 struct deleted (Task 4) ✅
- [x] `cargo build` succeeds ✅
- [x] All tests pass: `cargo test` ✅
- [x] No clippy warnings: `cargo clippy` ✅
- [x] `rg "AgentEventV1" crates/` returns no results ✅

**Total Time**: ~5 hours (complete native v2 migration)

---

## Quick Start for New Contributors

1. Read `V1_DEPENDENCY_ANALYSIS.md` for detailed dependency map
2. Start with Task 1 (safe deletions, 15 minutes)
3. Run tests after each task
4. Choose Option A (adapter) or Option B (native rewrite) for Task 2
5. Complete Task 3 and Task 4 sequentially

---

## References

- `V1_DEPENDENCY_ANALYSIS.md` - Complete dependency map (11 files, 30 locations)
- `docs/schema_v2/schema_v2.md` - V2 schema specification
- `docs/schema_v2/schema_goal.md` - Design goals and rationale
- `docs/schema_v2/v2_improvements.md` - Measured improvements (50-150% better)
- `docs/schema_v2/migration_v1_to_v2.md` - Migration strategy

---

## Migration History Summary

**Phases 1-4 (COMPLETED)**:
- Phase 1: Created v2 types alongside v1
- Phase 2: Implemented all 3 provider normalizations (Claude, Codex, Gemini)
- Phase 3: Built v2 span engine with O(1) tool matching
- Phase 4: Validated v2 accuracy, switched CLI to v2 pipeline

**Phase 5 (IN PROGRESS)**:
- See tasks above for remaining work

**Key Commits**:
- `51a5235` - refactor: migrate LogProvider and CLI to v2 pipeline
- `89dc427` - feat: complete Phase 4 - switch CLI to v2 with validation
- `498ea77` - feat: add v2 span engine with O(1) tool matching

---

## Key Design Decisions

**parent_id vs tool_call_id**:
- `parent_id`: Time-series linked list (chronological order)
- `tool_call_id`: Logical reference (which result belongs to which call)

**TokenUsage Sidecar Pattern**:
- Separate event type, not embedded in ToolCall/Message
- Enables async token updates without modifying parent events
- Filtered out when building LLM context (is_context_event() = false)

**Provider Normalization**:
- Gemini: Unfold nested structure into event stream
- Codex: Attach async token notifications as sidecars
- Claude: Extract embedded usage into separate TokenUsage events
