# AgTrace MCP Tools Interface Design

**Design Philosophy**: Build MCP tools optimized for LLM context efficiency and progressive disclosure. Follow [MCP 2024-11-05 specification](https://modelcontextprotocol.io/specification/2024-11-05/server/utilities/pagination) and industry best practices for large-payload handling.

**Target Models**: Optimized for advanced reasoning models (o3, Claude 3.5 Sonnet) with 100k+ context windows but limited tolerance for noise.

---

## What's New in v0.4.0

### ✨ New Tools

**`search_event_previews`** - Lightweight event search returning ~300 char previews
- Replaces `search_events` with better progressive disclosure
- Returns `event_index` for precise retrieval
- Supports `session_id` filter for scoped searches
- Uses `McpResponse` wrapper with structured pagination

**`get_event_details`** - Retrieve full event payload by index
- Complements `search_event_previews` for drill-down workflow
- Single-event responses (~1-5 KB)
- No truncation or size limits

### 🔧 Improvements

- **Type Safety**: `EventType` and `Provider` are now proper enums (not strings)
- **Structured Errors**: `McpError` with error codes, details, and retry flags
- **Standardized Responses**: `McpResponse<T>` wrapper with consistent pagination
- **Better Filtering**: Native `session_id` parameter for scoped searches

### ⚠️ Deprecations

- `search_events` - Deprecated in favor of `search_event_previews` + `get_event_details`
  - See [Migration Guide](#migration-guide) below

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

### `search_event_previews` ✨ NEW

**Purpose**: Find events across sessions (returns lightweight previews)
**Response Size**: ~10-15 KB per page (10 matches)
**Pagination**: Cursor-based

```typescript
// Request
{
  query: string,          // substring match in event JSON payloads
  limit?: number,         // default: 10, max: 50
  cursor?: string,        // pagination token
  provider?: "claude_code" | "codex" | "gemini",
  event_type?: "ToolCall" | "ToolResult" | "Message" | "User" | "Reasoning" | "TokenUsage" | "Notification",
  session_id?: string     // optional: search within specific session
}

// Response (McpResponse wrapper)
{
  data: {
    matches: [
      {
        session_id: "fb3cff44",
        event_index: 42,       // use this with get_event_details
        timestamp: "2026-01-01T21:44:41Z",
        event_type: "ToolCall",
        preview: {             // type-specific preview (~300 chars)
          tool: "Read",
          arguments: { file_path: "/path/to/file.rs" }  // truncated
        }
      }
    ]
  },
  pagination: {
    total_in_page: 10,
    next_cursor: "eyJ..." | null,
    has_more: false
  },
  hint: "Use get_event_details(session_id, event_index) to retrieve full event payload"
}
```

**Preview Content Types**:
- **ToolCall**: `{ tool: string, arguments: Value }` (args truncated to 100 chars)
- **ToolResult**: `{ preview: string }` (truncated to 300 chars)
- **Text events** (User/Message/Reasoning): `{ text: string }` (truncated to 300 chars)
- **TokenUsage**: `{ input: number, output: number }`

---

### `get_event_details` ✨ NEW

**Purpose**: Retrieve full event payload by session and index
**Response Size**: ~1-5 KB (single event)

```typescript
// Request
{
  session_id: string,     // 8-char prefix or full UUID
  event_index: number     // from search_event_previews response
}

// Response
{
  session_id: "fb3cff44-13ae-41a6-a6df-f287e0552835",
  event_index: 42,
  timestamp: "2026-01-01T21:44:41Z",
  event_type: "ToolCall",
  payload: {              // full, untruncated EventPayload
    name: "Read",
    arguments: {
      file_path: "/Users/zawakin/go/src/github.com/lanegrid/agtrace/src/main.rs"
    }
  }
}
```

**Usage Pattern**:
1. Use `search_event_previews` to find relevant events
2. Use `get_event_details` to retrieve full payload for specific events

---

### `search_events` ⚠️ DEPRECATED

**Status**: Deprecated since v0.4.0
**Migration**: Use `search_event_previews` + `get_event_details` instead

```typescript
// OLD (deprecated)
search_events(pattern="Read", include_full_payload=true)

// NEW (recommended)
1. search_event_previews(query="Read")      // get previews with event_index
2. get_event_details(session_id, event_index)  // get full payload
```

**Why deprecated**:
- `include_full_payload` toggle created inconsistent response sizes
- New split provides better progressive disclosure
- `event_index` enables precise event retrieval

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
   → search_event_previews(query="error", session_id="fb3cff44")
   → 5 KB response with event previews
   → get_event_details("fb3cff44", event_index=87)
   → 3 KB response with full error payload
```

**Total Response Size**: 10 + 2 + 25 + 80 + 5 + 3 = **125 KB** across 6 calls
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
   → search_event_previews(query="Read", event_type="ToolCall", limit=20)
   → 15 KB: List of Read tool calls with file paths (previews)
```

**Total Response Size**: 8 + 20 + 15 = **43 KB**
**Context Efficiency**: ✅ Excellent

---

### Example 3: Search and Retrieve Specific Events

```
1. User: "Find all tool failures in recent sessions"
   → search_event_previews(query="error", event_type="ToolResult", limit=10)
   → 12 KB: 10 failed tool results with previews

2. AI: "I found 10 failures. Event at index 42 in session fb3cff44 looks relevant."
   User: "Show me the full details"
   → get_event_details("fb3cff44", event_index=42)
   → 3 KB: Complete ToolResult payload with full error stack trace
```

**Total Response Size**: 12 + 3 = **15 KB**
**Context Efficiency**: ✅ Excellent - Targeted retrieval

---

## Error Handling

### Structured Error Responses

All MCP tools return structured errors following a consistent schema:

```typescript
{
  error: {
    code: "session_not_found" | "ambiguous_session_prefix" | "invalid_event_index" |
          "invalid_cursor" | "invalid_parameter" | "search_timeout" | "internal_error",
    message: "Human-readable error message",
    details: {
      // Error-specific context
    },
    retryable: boolean
  }
}
```

### Common Error Examples

#### Session Not Found
```json
{
  "error": {
    "code": "session_not_found",
    "message": "Session not found: fb3c",
    "details": {
      "session_id": "fb3c"
    },
    "retryable": false
  }
}
```

#### Ambiguous Session Prefix
```json
{
  "error": {
    "code": "ambiguous_session_prefix",
    "message": "Session ID prefix 'fb3' matches 3 sessions. Provide more characters.",
    "details": {
      "prefix": "fb3",
      "matches": ["fb3cff44", "fb3a1b2c", "fb3d9e8f"]
    },
    "retryable": false
  }
}
```

#### Invalid Event Index
```json
{
  "error": {
    "code": "invalid_event_index",
    "message": "Event index 999 out of bounds for session fb3cff44 (max: 127)",
    "details": {
      "session_id": "fb3cff44",
      "requested_index": 999,
      "max_index": 127
    },
    "retryable": false
  }
}
```

---

## Response Structure Standards

### McpResponse Wrapper

Tools that return lists or paginated data use a standardized wrapper:

```typescript
{
  data: T,                    // Tool-specific response data
  pagination?: {              // Present for list operations
    total_in_page: number,    // Items in current page
    next_cursor: string | null,  // Opaque token for next page
    has_more: boolean         // Quick check without parsing cursor
  },
  hint?: string              // Guidance for next steps
}
```

**Tools using McpResponse**:
- ✅ `search_event_previews`
- ⚠️ `list_sessions` (partial - lacks wrapper, has flat fields)
- ⚠️ `search_events` (deprecated)

**Tools with custom responses** (justified by use case):
- `get_session_details` - Response shape varies by detail_level
- `get_event_details` - Single event, no pagination needed
- `analyze_session` - Diagnostic report format

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

### ✅ Phase 1: MCP Spec Compliance (COMPLETED)

- [x] ~~`detail_level` hierarchy~~ (implemented)
- [x] ~~Response size hints~~ (implemented)
- [x] ~~Structured error handling with `McpError`~~ (implemented)
- [x] ~~Cursor pagination for `list_sessions`~~ (implemented)
- [x] ~~Split `search_events` → `search_event_previews` + `get_event_details`~~ (implemented v0.4.0)
- [x] ~~`EventType` and `Provider` enums~~ (implemented)
- [x] ~~`McpResponse<T>` wrapper for standardized responses~~ (implemented)

### Phase 2: MCP Integration Testing (Priority: HIGH - IN PROGRESS)

- [ ] Test with actual Claude Code client
- [ ] Test with o3 client
- [ ] Verify provider filter works correctly in MCP context
- [ ] Add integration tests for new tools
- [ ] Measure actual response sizes vs. targets

### Phase 3: Advanced Features (Priority: MEDIUM)

- [ ] Response size warnings in hints (> 30 KB alert)
- [ ] Migrate `list_sessions` to `McpResponse` wrapper (consistency)
- [ ] Field selection (`fields=["id", "timestamp"]`)
- [ ] Streaming support for very large sessions (> 1 MB)
- [ ] Rate limiting metadata in responses

### Phase 4: Optional Enhancements (Priority: LOW)

- [ ] `compare_sessions(id1, id2)` - Diff tool
- [ ] `get_reasoning_chain()` - Extract only thinking steps
- [ ] Advanced filters (regex, date ranges, token thresholds)
- [ ] Result caching for `search_event_previews`
- [ ] Telemetry for tool usage patterns

---

## Response Size Targets (Updated for Best Practices)

| Tool | Target | Max | Status |
|------|--------|-----|--------|
| `list_sessions` (10 items) | 10 KB | 20 KB | ✅ Compliant |
| `get_session_details` (summary) | 8 KB | 15 KB | ✅ Compliant |
| `get_session_details` (turns) | 20 KB | 30 KB | ✅ Compliant |
| `get_session_details` (steps) | 60 KB | **100 KB** | ⚠️ Review needed |
| `get_session_details` (full) | N/A | **Unbounded** | ⚠️ Use with caution |
| `analyze_session` | 2 KB | 5 KB | ✅ Compliant |
| `search_event_previews` (10 items) | 10 KB | 15 KB | ✅ Compliant |
| `get_event_details` (single) | 1-5 KB | 10 KB | ✅ Compliant |
| `get_project_info` | 10 KB | 20 KB | ✅ Compliant |
| ~~`search_events`~~ (deprecated) | 8 KB | 15 KB | ⚠️ Deprecated |

**Compliance Rate**: 7/9 tools within best practices (78%)
**New Tools**: `search_event_previews` and `get_event_details` both meet size targets

---

## References

- [MCP 2024-11-05 Specification - Pagination](https://modelcontextprotocol.io/specification/2024-11-05/server/utilities/pagination)
- [MCP Best Practices Guide](https://modelcontextprotocol.info/docs/best-practices/)
- [15 Best Practices for Production MCP Servers](https://thenewstack.io/15-best-practices-for-building-mcp-servers-in-production/)
- Industry guidelines: Single response < 30 KB, total investigation < 200 KB

---

**Last Updated**: 2026-01-02
**Specification Version**: MCP 2024-11-05
**Implementation Status**: Phase 1 Complete, Phase 2 In Progress

## Migration Guide

### Migrating from `search_events` to New Tools

The `search_events` tool has been split into two specialized tools for better progressive disclosure:

#### Pattern 1: Search with Previews (Default)
```typescript
// OLD
search_events({ pattern: "Read", limit: 10 })

// NEW
search_event_previews({ query: "Read", limit: 10 })
```

#### Pattern 2: Search with Full Payloads
```typescript
// OLD
search_events({ pattern: "error", include_full_payload: true })

// NEW (two-step)
const result = search_event_previews({ query: "error" })
const events = result.data.matches.map(m =>
  get_event_details({ session_id: m.session_id, event_index: m.event_index })
)
```

#### Pattern 3: Filter by Event Type
```typescript
// OLD
search_events({ pattern: "Write", event_type: "ToolCall" })

// NEW (with proper enum)
search_event_previews({ query: "Write", event_type: "ToolCall" })
```

#### Pattern 4: Search Within Session
```typescript
// OLD (not supported)
// Had to filter results manually

// NEW (native support)
search_event_previews({
  query: "error",
  session_id: "fb3cff44"
})
```

### Key Improvements

1. **Type Safety**: `event_type` is now an enum instead of string
2. **Better Indexing**: `event_index` enables precise retrieval
3. **Consistent Sizing**: Previews are always ~300 chars, full payloads always complete
4. **Structured Errors**: Rich error details instead of free-text messages
5. **McpResponse Wrapper**: Consistent pagination metadata
