# MCP Tools Redesign: Approach B (4 Specialized Tools)

## Design Decision

Based on MCP best practices research and LLM tool selection studies, we're migrating from:

**Before (Approach A):**
```
get_session_by_id(session_id, detail_level: summary|turns|steps|full)
```

**After (Approach B):**
```
get_session_summary(session_id)
get_session_turns(session_id, cursor?, limit?)
get_session_steps(session_id, cursor?, limit?)
get_session_full(session_id, cursor?, limit?)
```

## Rationale

### 1. LLM Selection Accuracy
- Research shows **1.3-1.6x higher error rate** with parameterized tools
- Two-stage decision (tool + parameter) vs single-stage (tool name encodes intent)
- Enum validation errors eliminated

### 2. Response Size Control
Current measurements:
- `summary`: 5-10 KB ✅
- `turns`: 50-100 KB ⚠️ (exceeds recommended 32 KB)
- `steps`: 100-300 KB ❌ (too large)
- `full`: unbounded ❌ (unusable)

With specialized tools:
- Each tool can have **different pagination strategies**
- Size hints embedded in tool descriptions
- Rate limiting per tool (e.g., restrict `full` access)

### 3. Tool Discoverability
- Short, focused descriptions (2-3 lines each)
- Clear size expectations: "Returns ≤5 KB session metadata"
- No complex enum documentation

## Tool Specifications

### 1. `get_session_summary`

**Purpose:** Lightweight session overview (always single-page)

**Request:**
```rust
struct GetSessionSummaryArgs {
    session_id: String,  // 8-char prefix or full UUID
}
```

**Response:**
```rust
struct SessionSummaryResponse {
    session_id: String,
    project_hash: String,
    provider: Provider,
    start_time: String,
    end_time: Option<String>,
    stats: SessionStats {
        total_turns: usize,
        total_tokens: u64,
        duration_seconds: u64,
    },
    snippet: String,  // First user message, max 200 chars
    _meta: ResponseMeta,
}
```

**Size:** ~5 KB guaranteed

---

### 2. `get_session_turns`

**Purpose:** Turn-level summaries with pagination

**Request:**
```rust
struct GetSessionTurnsArgs {
    session_id: String,
    cursor: Option<String>,
    limit: Option<usize>,  // default: 10, max: 50
}
```

**Response:**
```rust
struct SessionTurnsResponse {
    session_id: String,
    turns: Vec<TurnSummary>,  // Paginated
    next_cursor: Option<String>,
    _meta: ResponseMeta {
        bytes: usize,
        estimated_tokens: usize,
        has_more: bool,
        total_turns: usize,
        returned_count: usize,
    },
}
```

**Size:** ~10-30 KB per page (10 turns)

**Pagination:**
- Default limit: 10 turns
- Each turn ~1-3 KB (key_actions, outcome, token stats)

---

### 3. `get_session_steps`

**Purpose:** Detailed step-by-step execution (paginated)

**Decision Point:** Should this be turn-scoped or session-scoped?

**Option A: Session-scoped (all steps across all turns)**
```rust
struct GetSessionStepsArgs {
    session_id: String,
    cursor: Option<String>,
    limit: Option<usize>,  // default: 20, max: 100
}
```

**Option B: Turn-scoped (steps for a specific turn)**
```rust
struct GetTurnStepsArgs {
    session_id: String,
    turn_index: usize,  // Specific turn
    cursor: Option<String>,
    limit: Option<usize>,
}
```

**Recommendation:** Start with **Option B (turn-scoped)** renamed to `get_turn_steps`
- More focused, smaller responses
- Natural workflow: `get_session_turns` → pick turn → `get_turn_steps`
- Easier to stay under 32 KB limit

---

### 4. `get_session_full`

**Purpose:** Complete session data with mandatory pagination

**Request:**
```rust
struct GetSessionFullArgs {
    session_id: String,
    cursor: String,  // REQUIRED (not optional)
    limit: Option<usize>,  // default: 5, max: 10 (small chunks)
}
```

**Response:**
```rust
struct SessionFullResponse {
    session_id: String,
    chunk_data: SessionChunk,  // 5-10 turns with full payloads
    next_cursor: Option<String>,
    _meta: ResponseMeta {
        bytes: usize,
        estimated_tokens: usize,
        has_more: bool,
        chunk_index: usize,
        total_chunks: Option<usize>,
    },
}
```

**Size:** Target 50-100 KB per chunk

**Special handling:**
- First call: `cursor: "start"` or `cursor: null` (auto-initialized)
- Server enforces small limits (5-10 turns/chunk)
- Payloads **not truncated** (unlike `steps` level)

---

## Response Metadata (`_meta`)

All responses include:

```rust
struct ResponseMeta {
    bytes: usize,             // Actual JSON byte size
    estimated_tokens: usize,  // ~bytes / 4
    has_more: bool,           // More data available?
    next_cursor: Option<String>,

    // Context-specific fields
    total_items: Option<usize>,      // For lists
    returned_count: Option<usize>,   // Items in this response
}
```

## Tool Descriptions (for MCP registration)

```rust
{
    name: "get_session_summary",
    description: "Get lightweight session overview (≤5 KB). Returns session metadata, turn count, token stats, and first message snippet. Always single-page, no pagination needed.",
    inputSchema: schema_for!(GetSessionSummaryArgs)
}

{
    name: "get_session_turns",
    description: "Get turn-level summaries with tool usage (10-30 KB per page). Paginated. Each turn includes key actions, outcome, and token stats. Start here after reviewing summary.",
    inputSchema: schema_for!(GetSessionTurnsArgs)
}

{
    name: "get_turn_steps",
    description: "Get detailed steps for a specific turn (20-50 KB). Shows tool calls, results, and truncated payloads. Use after identifying interesting turns.",
    inputSchema: schema_for!(GetTurnStepsArgs)
}

{
    name: "get_session_full",
    description: "Get complete session data with full payloads (50-100 KB per chunk). REQUIRES pagination cursor. Use sparingly—only when you need untruncated tool inputs/outputs.",
    inputSchema: schema_for!(GetSessionFullArgs)
}
```

## Migration Plan

### Phase 1: Add new tools (keep old)
- ✅ Implement 4 new request DTOs
- ✅ Implement pagination for turns/steps/full
- ✅ Add `_meta` field to all responses
- ✅ Register new tools alongside `get_session_by_id`
- ✅ Mark `get_session_by_id` as deprecated in description

### Phase 2: Update documentation
- ✅ Update MCP server README
- ✅ Add migration guide
- ✅ Update tool descriptions with size hints

### Phase 3: Remove old tool (Breaking Change)
- ⬜ Remove `get_session_by_id`
- ⬜ Remove `GetSessionDetailsArgs`
- ⬜ Update CHANGELOG.md

## Open Questions

1. **`get_session_steps` vs `get_turn_steps`?**
   - Leaning toward `get_turn_steps` (turn-scoped) for better size control

2. **Should `get_session_full` require cursor even for first call?**
   - Recommendation: Accept `null`/`"start"` as initial cursor
   - Forces explicit pagination mindset

3. **Rate limiting per tool?**
   - Consider adding `max_requests_per_minute` metadata
   - Example: `get_session_full` → 10/min, `get_session_summary` → 100/min

## Success Metrics

- [ ] All responses stay under 100 KB (preferably 32 KB)
- [ ] `estimated_tokens` within 10% of actual token count
- [ ] LLM tool selection error rate decreases (measure via logs)
- [ ] Users primarily use `summary` → `turns` workflow (track usage stats)
