# AgTrace MCP Tools Interface Design

**Design Philosophy**: Build MCP tools optimized for LLM context efficiency and progressive disclosure. Follow [MCP 2024-11-05 specification](https://modelcontextprotocol.io/specification/2024-11-05/server/utilities/pagination) and industry best practices for large-payload handling.

**Target Models**: Optimized for advanced reasoning models (o3, Claude 3.5 Sonnet) with 100k+ context windows but limited tolerance for noise.

---

## Core Design Principles

### 1. Response Size Guidelines (MCP Best Practices 2024)

| Category | Target | Max | Rationale |
|----------|--------|-----|-----------|
| **Single Response** | 10-20 KB | 30 KB | Safe for all LLMs (~4-6k tokens) |
| **Detail Response** | 20-30 KB | 50 KB | Acceptable for focused queries |
| **Full Data** | N/A | 100 KB | Hard cap; prefer pagination beyond this |
| **Total Investigation** | 40-100 KB | 200 KB | Across 3-8 tool calls |

**Key Insight**: Keep each tool response < 30 KB to avoid overwhelming LLM working memory. Use cursor-based pagination for larger datasets.

### 2. Progressive Disclosure Pattern

```
Breadth (10 items × 1 KB each)
  ↓
Overview (1 item × 10 KB)
  ↓
Detail (1 turn × 30 KB)
  ↓
Full Event (1 event × 5 KB)
```

**Never**: Dump 500 KB of data in a single response. **Always**: Let the LLM drill down conversationally.

### 3. Cursor-Based Pagination (MCP Specification)

All list operations MUST support opaque cursor tokens:

```typescript
{
  items: [...],
  next_cursor: "eyJsYXN0X2lkIjo0Mjd9" | null  // null = final page
}
```

**Why cursors over offsets**:
- Stable across concurrent modifications
- Opaque implementation details
- MCP 2024-11-05 specification compliance

---

## Tool Hierarchy

```
Projects
  └─ Sessions (paginated list)
      ├─ Session Overview (summary)
      ├─ Session Analysis (diagnostics)
      └─ Turns (indexed access)
          └─ Events (by ID)
              └─ Full Payload
```

---

## 1. Discovery & Navigation Tools

### `list_sessions`

**Purpose**: Browse recent sessions with pagination
**Response Size**: ~10 KB per page (10 sessions)
**Pagination**: Cursor-based (MCP spec compliant)

```typescript
// Request
{
  limit?: number,           // default: 10, max: 50
  cursor?: string,          // opaque pagination token
  provider?: string,        // "claude_code" | "codex" | "gemini"
  project_hash?: string,
  since?: string,           // ISO 8601
  until?: string
}

// Response
{
  sessions: [
    {
      id: "fb3cff44",
      project_hash: "427e6b3f...",
      provider: "claude_code",
      start_time: "2026-01-01T21:40:05Z",
      snippet: "過去コミット2個くらいでmcpを作った...", // max 200 chars
      turn_count: 6,
      duration_seconds: 3520,
      total_tokens: 3074273
    }
  ],
  total_in_page: 10,
  next_cursor: "eyJvZmZzZXQiOjEwfQ==" | null,  // null = last page
  hint: "Use get_session_details(id, detail_level='summary') for turn breakdown"
}
```

**Implementation Notes**:
- Server determines page size; clients MUST NOT assume fixed size
- Cursors are opaque and MAY change format without notice
- Always check `next_cursor == null` to detect final page

---

### `get_project_info`

**Purpose**: List all indexed projects
**Response Size**: ~10 KB

```typescript
// Request
{}

// Response
{
  projects: [
    {
      hash: "427e6b3f...",
      root_path: "/Users/zawakin/go/src/github.com/lanegrid/agtrace",
      session_count: 47,
      last_activity: "2026-01-02T06:38:45Z"
    }
  ]
}
```

---

## 2. Session-Level Tools

### `get_session_details`

**Purpose**: Retrieve session data with configurable detail level
**Response Size**: 5 KB (summary) to 100 KB (full)
**Detail Levels**: Progressive disclosure hierarchy

```typescript
// Request
{
  session_id: string,        // "fb3cff44" or full UUID
  detail_level?: "summary" | "turns" | "steps" | "full",  // default: "summary"
  include_reasoning?: boolean  // default: false (only for 'turns' level)
}

// Response varies by detail_level
```

#### Detail Level: `summary` (Target: 5-10 KB)

```typescript
{
  session_id: "fb3cff44-13ae-41a6-a6df-f287e0552835",
  start_time: "2026-01-01T21:40:05Z",
  end_time: "2026-01-01T22:38:45Z",
  stats: {
    total_turns: 6,
    total_tokens: 3074273,
    duration_seconds: 3520
  },
  turns: [
    {
      id: "turn-0",
      timestamp: "2026-01-01T21:40:05Z",
      user_message: "過去コミット2個くらいでmcpを作った...", // max 200 chars
      step_count: 14,
      stats: { input_tokens: 50000, output_tokens: 8000 }
    }
  ],
  hint: "Use detail_level='turns' to see tool execution summaries"
}
```

#### Detail Level: `turns` (Target: 15-30 KB)

```typescript
{
  session_id: "...",
  turns: [
    {
      turn_index: 0,
      user_message: "...",
      steps: [
        {
          step_index: 0,
          summary: "Read /path/to/file.rs (ok), Write /path/to/output.rs (ok)",
          tool_calls: 2,
          failed_tools: 0,
          tokens: { input: 5000, output: 800 }
        }
      ],
      outcome: "success" | "partial" | "failed",
      key_actions: ["Used tools: Read, Write"]
    }
  ],
  hint: "Use detail_level='steps' for detailed payloads"
}
```

#### Detail Level: `steps` (Target: 50-100 KB)

Returns full `AgentSession` structure with **truncated payloads** (500 chars):

```typescript
{
  // Full session structure
  turns: [
    {
      steps: [
        {
          reasoning: { text: "思考内容..." },  // truncated to 500 chars
          tools: [
            {
              call: { name: "Read", arguments: {...} },
              result: { content: "1→use agtrace..." }  // truncated to 500 chars
            }
          ]
        }
      ]
    }
  ],
  hint: "Payloads are truncated to 500 chars. Use search_events(pattern) to find specific content, or detail_level='full' for complete data"
}
```

#### Detail Level: `full` (Target: Unbounded, use with caution)

Returns complete `AgentSession` without any truncation:

```typescript
{
  // Complete session with all payloads
  hint: "Response may be large. Use search_events(pattern) to find specific events, or detail_level='steps' for truncated payloads"
}
```

**Design Rationale**:
- **summary**: Quick scan of session structure
- **turns**: Tool usage patterns and outcomes
- **steps**: Detailed debugging with size control
- **full**: Last resort when complete context needed

---

### `analyze_session`

**Purpose**: Diagnostic health check (failures, loops, anomalies)
**Response Size**: ~1-5 KB

```typescript
// Request
{
  session_id: string,
  include_failures?: boolean,  // default: true
  include_loops?: boolean      // default: false
}

// Response
{
  score: 80,  // 0-100 health score
  insights: [
    {
      turn_index: 0,
      severity: "Critical" | "Warning" | "Info",
      lens: "Failures" | "Loops" | "TokenUsage",
      message: "1 tool execution(s) failed"
    }
  ],
  summary: {
    event_counts: {
      assistant_messages: 14,
      tool_calls: 67,
      tool_results: 67,
      user_messages: 6
    }
  }
}
```

---

## 3. Search Tools

### `search_events`

**Purpose**: Find events across sessions
**Response Size**: ~5-10 KB per page (5 matches)
**Pagination**: Cursor-based

```typescript
// Request
{
  pattern: string,        // substring match
  limit?: number,         // default: 5, max: 20
  cursor?: string,        // pagination token
  provider?: string,
  event_type?: string,    // "ToolCall" | "ToolResult" | etc
  include_full_payload?: boolean  // default: false
}

// Response
{
  matches: [
    {
      session_id: "fb3cff44",
      timestamp: "2026-01-01T21:44:41Z",
      type: "ToolCall",
      preview: {  // When include_full_payload=false (default)
        tool: "Read",
        arguments: { file_path: "..." }  // truncated to 100 chars
      }
      // OR
      payload: { ... }  // When include_full_payload=true
    }
  ],
  total: 5,
  next_cursor: "eyJ..." | null,
  hint: "Use get_session_details(session_id, detail_level='steps') to see all events in a session"
}
```

**Key Design Decision**: Return **previews** by default to stay within 30 KB limit. LLM can request full payloads explicitly.

---

## Tool Flow Examples

### Example 1: Debug Failed Tools in o3 Session

```
1. User: "Show me recent o3 sessions"
   → list_sessions(provider="claude_code", limit=10)
   → 10 KB response with session summaries

2. AI: "I see session fb3cff44 with 273k tokens. Let me check health."
   → analyze_session("fb3cff44")
   → 2 KB response: Score 65, "Turn 2 and 5 had failures"

3. AI: "Turn 2 looks critical. Let me examine it."
   → get_session_details("fb3cff44", detail_level="turns")
   → 25 KB response with turn summaries

4. AI: "Turn 2 step 3 failed. Let me see full details."
   → get_session_details("fb3cff44", detail_level="steps")
   → 80 KB response with truncated payloads

5. User: "What was the exact error message?"
   → search_events(pattern="error", session_id="fb3cff44", include_full_payload=true)
   → 8 KB response with full error details
```

**Total Response Size**: 10 + 2 + 25 + 80 + 8 = **125 KB** across 5 calls
**Context Efficiency**: ✅ Within 200 KB budget

---

### Example 2: Investigate Token Usage Pattern

```
1. AI: "Let me check token distribution"
   → get_session_details("abc123", detail_level="summary")
   → 8 KB: See that Turn 3 consumed 150k tokens

2. AI: "That's unusually high. What tools were used?"
   → get_session_details("abc123", detail_level="turns")
   → 20 KB: Turn 3 had 47 tool calls (mostly Read)

3. User: "Show me what files were read"
   → search_events(pattern="Read", limit=20)
   → 15 KB: List of all Read tool calls with file paths
```

**Total Response Size**: 8 + 20 + 15 = **43 KB**
**Context Efficiency**: ✅ Excellent

---

## Best Practices Summary

### ✅ Recommended Patterns

1. **Cursor-based pagination**: Use opaque tokens for all list operations
2. **Progressive disclosure**: Start with summaries, drill down on demand
3. **Size awareness**: Include response size estimates in hints
4. **Idempotent operations**: Same request = same response (for retry safety)
5. **Defensive truncation**: Always truncate by default; provide opt-in for full data

### ❌ Anti-Patterns to Avoid

1. ~~Offset-based pagination~~ (not stable, not MCP-compliant)
2. ~~Single "dump everything" tool~~ (overwhelms context)
3. ~~No way to get full details~~ (blocks deep debugging)
4. ~~Assuming fixed page sizes~~ (violates MCP spec)
5. ~~Returning 500 KB tool results~~ (exceeds best practice limits)

### 🔒 Security & Privacy Safeguards

1. **Server-side size limits**: Enforce max 100 KB per response regardless of client request
2. **Redact secrets**: Never return API keys, tokens, or credentials in event payloads
3. **Audit logging**: Track request/response sizes for monitoring
4. **Timeout protection**: Cap expensive searches at 30 seconds

---

## Implementation Roadmap

### Phase 1: MCP Spec Compliance (Priority: HIGH)

- [x] ~~`detail_level` hierarchy~~ (implemented)
- [x] ~~Response size hints~~ (implemented)
- [x] ~~Error handling improvements~~ (implemented)
- [ ] **Cursor pagination for `list_sessions`** (REQUIRED)
- [ ] **Cursor pagination for `search_events`** (RECOMMENDED)

### Phase 2: Advanced Features (Priority: MEDIUM)

- [ ] Response size warnings (> 30 KB alert)
- [ ] Field selection (`fields=["id", "timestamp"]`)
- [ ] Streaming support for very large sessions (> 1 MB)

### Phase 3: Optional Enhancements (Priority: LOW)

- [ ] `compare_sessions(id1, id2)` - Diff tool
- [ ] `get_reasoning_chain()` - Extract only thinking steps
- [ ] Advanced filters (regex, date ranges, token thresholds)

---

## Response Size Targets (Updated for Best Practices)

| Tool | Target | Max | Status |
|------|--------|-----|--------|
| `list_sessions` (10 items) | 10 KB | 20 KB | ✅ Compliant |
| `get_session_details` (summary) | 8 KB | 15 KB | ✅ Compliant |
| `get_session_details` (turns) | 20 KB | 30 KB | ✅ Compliant |
| `get_session_details` (steps) | 60 KB | **100 KB** | ⚠️ Review needed |
| `get_session_details` (full) | N/A | **Unbounded** | ❌ Needs pagination |
| `analyze_session` | 2 KB | 5 KB | ✅ Compliant |
| `search_events` (5 items) | 8 KB | 15 KB | ✅ Compliant |
| `get_project_info` | 10 KB | 20 KB | ✅ Compliant |

**Compliance Rate**: 6/8 tools within best practices (75%)

---

## References

- [MCP 2024-11-05 Specification - Pagination](https://modelcontextprotocol.io/specification/2024-11-05/server/utilities/pagination)
- [MCP Best Practices Guide](https://modelcontextprotocol.info/docs/best-practices/)
- [15 Best Practices for Production MCP Servers](https://thenewstack.io/15-best-practices-for-building-mcp-servers-in-production/)
- Industry guidelines: Single response < 30 KB, total investigation < 200 KB

---

**Last Updated**: 2026-01-02
**Specification Version**: MCP 2024-11-05
**Implementation Status**: Phase 1 (75% complete)
