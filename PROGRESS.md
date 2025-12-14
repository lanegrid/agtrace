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

### Phase 4: Validation & Switch ⏳ IN PROGRESS
**Goal**: Verify v2 produces correct results, then switch CLI to v2

**Tasks**:
- [ ] Create dual-pipeline tests:
  - [ ] Run same input through v1 and v2 pipelines
  - [ ] Compare SessionSummary outputs
  - [ ] Compare Span counts and token calculations
  - [ ] Document where v2 is more accurate
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
- [ ] Switch CLI commands to use v2 pipeline:
  - [ ] Update session show command
  - [ ] Update analysis commands
  - [ ] Update export commands
- [ ] Update documentation

**Success Criteria**: All tests pass, CLI uses v2 by default

**Commits**:
- `1d41f45` - feat: add normalize_*_file_v2 helpers and integration tests for Codex and Claude

### Phase 5: Cleanup ⏳ NOT STARTED
**Goal**: Remove v1 code once v2 is stable

**Tasks**:
- [ ] Remove v1 types:
  - [ ] Delete/deprecate AgentEventV1
  - [ ] Remove v1 mapper code from providers
- [ ] Remove v1 engine:
  - [ ] Delete span.rs, turn.rs (old versions)
  - [ ] Rename span_v2.rs -> span.rs
- [ ] Remove guessing logic:
  - [ ] Delete JSON parsing hacks in extract_input_summary
  - [ ] Remove event order assumptions
- [ ] Update all documentation and comments

**Success Criteria**: Codebase only contains v2 code, all tests pass

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

Phase 4 is in progress! Integration tests for all three providers are now complete.

**Immediate Next Steps**:
1. **Phase 4: Validation & Switch** (Remaining tasks)
   - Create dual-pipeline tests comparing v1 and v2 outputs
   - Validate token calculations and span accuracy across all providers
   - Document where v2 is more accurate than v1
   - Switch CLI commands to use v2 pipeline

2. **Future Considerations**:
   - Decide if turn_v2.rs is needed (can defer to Phase 5)
   - Consider adding summary_v2.rs for SessionSummary generation
   - Plan full CLI migration rollout

## References

- `docs/schema_v1/schema_v1.md` - V1 specification
- `docs/schema_v2/schema_v2.md` - V2 Rust schema definition
- `docs/schema_v2/migration_v1_to_v2.md` - Migration strategy
- `docs/schema_v2/schema_goal.md` - Design goals and non-goals
