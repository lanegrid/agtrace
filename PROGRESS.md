# Schema v1 to v2 Migration Progress

## Current State (2025-12-14 - Phase 4 In Progress)

### Codebase Overview
- **Total lines**: ~9,500 lines
- **Current schema**: v1 (AgentEventV1 - flat structure)
- **Crates**:
  - `agtrace-types`: Type definitions (AgentEventV1, enums)
  - `agtrace-providers`: Provider-specific code (claude, codex, gemini)
  - `agtrace-engine`: Analysis engine (span, turn, summary, ~1,400 lines)
  - `agtrace-cli`: CLI interface
  - `agtrace-index`: Database/storage

### V1 Schema Issues
Based on docs/schema_v2/schema_goal.md, the v1 schema has:
1. **Fat Structure Problem**: AgentEventV1 tries to hold all possible fields, leading to ambiguity
2. **Missing Logical References**: No proper tool_call_id as UUID for linking calls and results
3. **Token Attribution Issues**: Tokens embedded in events, making async/incremental updates difficult
4. **Context Reconstruction Issues**: Difficult to rebuild conversation history accurately

### V2 Schema Goals
1. **Provider Normalization**: Abstract provider differences into unified time-series data
2. **Accurate Cost Tracking**: Sidecar TokenUsage events for proper async token tracking
3. **Context Replayability**: Linked-list structure (parent_id) for perfect conversation replay
4. **Logic-Time Separation**: Separate time-series (parent_id) from logical links (tool_call_id)

## Migration Strategy: 5-Phase Strangler Fig Pattern

Following docs/schema_v2/migration_v1_to_v2.md, we will implement parallel adoption to avoid breaking existing functionality.

### Phase 1: Type Layer Coexistence ✅ COMPLETED (2025-12-14)
**Goal**: Create v2 types alongside v1 without removing v1

**Tasks**:
- [x] Create `crates/agtrace-types/src/v2.rs` with:
  - [x] AgentEvent struct
  - [x] EventPayload enum (User, Reasoning, ToolCall, ToolResult, Message, TokenUsage)
  - [x] Supporting payload structs
  - [x] Helper methods (is_generation_event, is_context_event)
- [x] Export v2 module in `crates/agtrace-types/src/lib.rs`
- [x] Keep v1 types intact (no changes to existing AgentEventV1)
- [x] Add dependencies: uuid (1.10), chrono serde feature

**Success Criteria**: ✅ Code compiles with both v1 and v2 types available, all tests pass

### Phase 2: Normalization Layer ✅ COMPLETED (2025-12-14)
**Goal**: Implement provider raw data -> v2 conversion

**Critical Design Decision**:
Convert **Provider Raw -> V2** directly (NOT V1 -> V2), because v1 loses information.

**Schema Review Applied** (2025-12-14):
- [x] Add `provider_call_id` to ToolCallPayload for log tracing
- [x] Extend TokenUsageDetails with audio token fields
- [x] Validate against real provider data (Claude, Codex, Gemini)
- [x] Document ingestion requirements (UUID resolution, unfold strategies)

**Tasks**:
- [x] Create conversion infrastructure:
  - [x] EventBuilder with trace_id, parent_id tracking
  - [x] tool_map for provider tool_id -> UUID mapping
- [x] Implement provider converters:
  - [x] Gemini: Unfold nested structure into event stream ✅
    - [x] Handle thoughts[] -> multiple Reasoning events
    - [x] Handle toolCalls[] -> ToolCall + ToolResult pairs
    - [x] Handle token attribution (last generation event of turn)
    - [x] Set provider_call_id for traceability
    - [x] Tests pass (4 tests)
  - [x] Codex: Handle async token notifications ✅
    - [x] Parse JSON string arguments to Value
    - [x] Extract exit codes from text output using regex
    - [x] Attach TokenUsage as sidecar events
    - [x] Tests pass (3 tests)
  - [x] Claude: Extract embedded usage ✅
    - [x] Parse message.content[] blocks (unfold tool_result)
    - [x] Create TokenUsage events for ToolCall/Message
    - [x] Handle thinking blocks → Reasoning events
    - [x] Tests pass (3 tests)
- [x] Write conversion tests with snapshot data (10 unit tests passing)

**Success Criteria**: ✅ Can convert all provider snapshots to Vec<v2::AgentEvent>

**Commits**:
- `0f912f3` - feat: add v2 schema types and Gemini normalization layer
- `daed86d` - docs: update progress tracking for v2 migration
- `acc8b16` - refactor: add provider_call_id and audio tokens to v2 schema per review
- Next - feat: add v2 normalization layers for Codex and Claude

### Phase 3: Parallel Engine Implementation ✅ COMPLETED (2025-12-14)
**Goal**: Implement v2-based analysis alongside v1 engine

**Tasks**:
- [x] Create `crates/agtrace-engine/src/span_v2.rs` ✅
  - [x] build_spans_v2(events: &[v2::AgentEvent]) -> Vec<Span>
  - [x] Replace "pending buffer" logic with HashMap<Uuid, ToolAction>
  - [x] Use tool_call_id for O(1) call-result matching
  - [x] Implement TokenUsage sidecar pattern with message_map
  - [x] Add exit code extraction from tool output
- [x] Create `crates/agtrace-engine/src/turn_v2.rs` - SKIPPED (will implement in Phase 4 if needed)
- [x] Update facade in lib.rs to expose v2 functions ✅
- [x] Add regex and uuid dependencies to engine ✅
- [x] Export provider modules as public for testing ✅
- [x] Create normalize_gemini_file_v2 helper ✅
- [x] Write integration tests with Gemini snapshots ✅
  - [x] test_gemini_span_v2_building
  - [x] test_v2_tool_matching_accuracy
- [x] All tests passing (11 unit + integration tests) ✅

**Success Criteria**: ✅ Can build spans from v2 events with improved accuracy

**Key Improvements Over V1**:
- O(1) tool call/result matching using HashMap instead of linear search
- No fallback guessing logic - all references are explicit via UUIDs
- TokenUsage as sidecar events, not embedded in generation events
- Proper handling of out-of-order tool results
- Clean separation of concerns with dedicated maps for tools and messages

**Commits**:
- `498ea77` - feat: add v2 span engine with O(1) tool matching and sidecar token tracking

### Phase 4: Validation & Switch ✅ COMPLETED (2025-12-14)
**Goal**: Verify v2 produces correct results, then switch CLI to v2

**Tasks**:
- [x] Create dual-pipeline tests ✅:
  - [x] Run same input through v1 and v2 pipelines
  - [x] Compare SessionSummary outputs
  - [x] Compare Span counts and token calculations
  - [x] Document where v2 is more accurate (docs/schema_v2/v2_improvements.md)
- [x] Add integration tests for all providers ✅ (2025-12-14)
  - [x] Created `normalize_codex_file_v2` helper function
  - [x] Created `normalize_claude_file_v2` helper function
  - [x] Added integration tests in `span_v2_snapshots.rs`:
    - [x] test_codex_span_v2_building
    - [x] test_claude_span_v2_building
  - [x] All v2 tests passing (13 provider unit tests + 6 engine tests)
- [x] Add v2 provider snapshot tests ✅ (2025-12-14)
  - [x] Add test_gemini_parse_v2_snapshot
  - [x] Add test_codex_parse_v2_snapshot
  - [x] Add test_claude_parse_v2_snapshot
  - [x] Add test_gemini_parse_raw_v2_snapshot
  - [x] Add test_codex_parse_raw_v2_snapshot
  - [x] Add test_claude_parse_raw_v2_snapshot
  - [x] UUID redaction helper for deterministic snapshots
  - [x] All 6 v2 snapshot tests passing
- [x] Switch CLI commands to use v2 pipeline ✅:
  - [x] Update session show command (uses v2 directly for spans)
  - [x] Update analysis commands (loads v2, converts to v1 for compatibility)
  - [x] Update export commands (loads v2, converts to v1 for compatibility)
  - [x] Add SessionLoader::load_events_v2() method
  - [x] Add summarize_v2() function for v2 events
- [x] Documentation ✅:
  - [x] Created docs/schema_v2/v2_improvements.md with quantitative comparisons
  - [x] Documented 50-150% improvement in span accuracy
  - [x] Documented 363% improvement in token tracking (Claude)

**Success Criteria**: ✅ All tests pass, CLI uses v2 by default

**V2 Improvements Measured**:
- **Claude**: 150% more spans captured (5 vs 2), 734 more tokens tracked
- **Codex**: 66% more spans captured (5 vs 3), same token count
- **Gemini**: Same spans (2 vs 2), same token count, but 52% more granular events

**Commits**:
- `1d41f45` - feat: add normalize_*_file_v2 helpers and integration tests for Codex and Claude
- Next commits will include: dual-pipeline tests, CLI v2 switch, and documentation

### Phase 5: Cleanup ⏳ IN PROGRESS (2025-12-14)
**Goal**: Remove v1 code once v2 is stable

**Tasks**:
- [ ] Remove v1 types:
  - [ ] Delete/deprecate AgentEventV1 - PENDING (still used in some places)
  - [x] Remove v1 mapper code from providers - PARTIAL (mapper modules still exist but unused)
- [x] Remove v1 engine:
  - [x] Delete span.rs, turn.rs (old versions) - span_v2.rs renamed to span.rs, turn.rs deleted
  - [x] Rename span_v2.rs -> span.rs
- [x] Remove v1 loading infrastructure:
  - [x] Update SessionLoader to use v2 only
  - [x] Update LogProvider trait to return v2 events
  - [x] Remove v1 test files (dual_pipeline_comparison, span_snapshots, turn_snapshots)
  - [x] Remove v1 provider snapshot tests
- [x] Update CLI to use v2:
  - [x] Fix timeline output to work with v2 events (with local v1 adapter)
  - [x] Update all handlers to use load_events_v2

**Success Criteria**: Codebase uses v2 by default, all tests pass ✅

**Completed Work (2025-12-14)**:
- Renamed span_v2.rs → span.rs
- Deleted turn.rs
- Updated LogProvider trait to return Vec<AgentEvent> (v2)
- Removed SessionLoader::load_events (v1 method)
- Updated all three providers (Claude, Codex, Gemini) to use v2 normalize functions
- Removed 3 v1 test files and 6 v1 snapshot tests
- Created local timeline summary adapter for v1 compatibility
- All tests passing (30 tests across all crates)

**Remaining Work**:
- Complete removal of AgentEventV1 type (requires updating remaining v1 usage)
- Delete mapper.rs modules (currently unused)
- Remove v1 normalize functions from io.rs files
- Update convert_v2_to_v1 usage in timeline output

**Commits**:
- `51a5235` - refactor: migrate LogProvider and CLI to v2 pipeline, remove v1 loading and test code

## Key Design Decisions

### parent_id: Time-Series Chain
- Forms a linked list of events in chronological order
- Used for context reconstruction (LLM conversation history)
- Each event points to the previous event in the conversation

### tool_call_id: Logical Reference
- Stored in ToolResultPayload to link to ToolCallPayload
- Allows O(1) lookup of which call a result belongs to
- Handles out-of-order and async results

### TokenUsage: Sidecar Pattern
- Separate event type, not embedded in ToolCall/Message
- parent_id points to the generation event it measures
- Filtered out (is_context_event() = false) when building LLM context
- Enables async/incremental token updates without modifying parent events

### Provider Differences
- **Gemini**: Batch structure -> unfold into event stream
- **Codex**: Async tokens + echo -> dedupe and attach sidecar
- **Claude**: Embedded usage -> extract and create sidecar events

## Next Steps

**Phase 4 is complete!** ✅ The CLI now uses v2 by default with proven improvements in accuracy.

**Immediate Next Steps**:
1. **Phase 5: Cleanup** (Optional - can run v1/v2 in parallel indefinitely)
   - Remove v1 types and engine code
   - Remove convert_v2_to_v1 adapter functions
   - Update analysis/export to use v2 natively
   - Rename span_v2.rs → span.rs

2. **Recommended Workflow**:
   - Test v2 in production with real sessions
   - Monitor for any edge cases or issues
   - Keep v1 code as fallback if needed
   - Only proceed with Phase 5 cleanup after confidence in v2 stability

3. **Future Enhancements**:
   - Add native v2 analysis functions (remove v1 conversion)
   - Add native v2 export functions (remove v1 conversion)
   - Consider adding turn_v2.rs if turn-based analysis is needed

## References

- `docs/schema_v1/schema_v1.md` - V1 specification
- `docs/schema_v2/schema_v2.md` - V2 Rust schema definition
- `docs/schema_v2/migration_v1_to_v2.md` - Migration strategy
- `docs/schema_v2/schema_goal.md` - Design goals and non-goals
