# AgTrace MCP Tools Interface Design

**Design Principle**: Each tool should return 5-50 KB responses. Enable conversational drill-down rather than dumping everything at once.

---

## Tool Hierarchy

```
Projects
  └─ Sessions (list)
      └─ Session (overview)
          └─ Turn (details)
              └─ Event (full payload)
```

---

## 1. Discovery & Navigation Tools

### `list_sessions`
**Purpose**: Browse recent sessions
**Response Size**: ~10 KB (10 sessions)

```typescript
// Request
{
  limit?: number,           // default: 10, max: 50
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
  total: 25,
  hint: "Use get_session_overview(id) to see turn-by-turn breakdown"
}
```

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

### `get_session_overview`
**Purpose**: High-level session stats + turn summaries
**Response Size**: ~5-15 KB (6 turns)

```typescript
// Request
{
  session_id: string  // "fb3cff44" or full UUID
}

// Response
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
      turn_index: 0,
      timestamp: "2026-01-01T21:40:05Z",
      user_message: "過去コミット2個くらいでmcpを作った...", // max 200 chars
      step_count: 14,
      tool_call_count: 12,
      failed_tools: 1,
      tokens: { input: 50000, output: 8000 },
      duration_ms: 86445
    },
    // ... 5 more turns
  ],
  hint: "Use get_turn_details(session_id, turn_index) to see tool calls and results"
}
```

---

### `analyze_session`
**Purpose**: Diagnostic health check
**Response Size**: ~1 KB

```typescript
// Request
{
  session_id: string,
  include_failures?: boolean,  // default: true
  include_loops?: boolean      // default: false
}

// Response
{
  score: 80,  // 0-100
  insights: [
    {
      turn_index: 0,
      severity: "Critical",
      lens: "Failures",
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

## 3. Turn-Level Tools (Deep Dive)

### `get_turn_details`
**Purpose**: Complete details for a specific turn
**Response Size**: ~20-50 KB per turn

```typescript
// Request
{
  session_id: string,
  turn_index: number  // 0-based
}

// Response
{
  turn: {
    turn_index: 0,
    user_message: "過去コミット2個くらいでmcpを作った...",  // full text
    timestamp: "2026-01-01T21:40:05Z",
    steps: [
      {
        step_index: 0,
        reasoning: [
          { text: "MCPサーバーの実装を確認..." }  // truncated to 500 chars
        ],
        tool_calls: [
          {
            event_id: "evt_abc123",  // NEW: for get_event_details()
            tool: "Read",
            arguments: { file_path: "/path/to/file.rs" },
            result_preview: "1→use agtrace_sdk::Client;...",  // 200 chars
            is_error: false,
            duration_ms: 45
          }
        ],
        message: { text: "コードを確認しました..." },
        tokens: { input: 5000, output: 800 }
      }
      // ... more steps
    ],
    stats: {
      step_count: 14,
      tool_call_count: 12,
      failed_tools: 1,
      tokens: { input: 50000, output: 8000 }
    }
  },
  hint: "Use get_event_details(session_id, event_id) for full tool result content"
}
```

**Key Design Decision**:
- Tool results are **previewed** (200 chars)
- Full content requires `get_event_details()`

---

## 4. Event-Level Tools (Maximum Detail)

### `get_event_details`
**Purpose**: Get complete payload for a specific event
**Response Size**: ~1-10 KB per event

```typescript
// Request
{
  session_id: string,
  event_id: string  // from get_turn_details()
}

// Response
{
  event: {
    id: "evt_abc123",
    timestamp: "2026-01-01T21:40:05Z",
    type: "ToolResult",
    payload: {
      content: "<FULL BASH OUTPUT - 5000 chars>",  // NO TRUNCATION
      is_error: false
    }
  }
}
```

---

## 5. Search Tools

### `search_events`
**Purpose**: Find events across sessions
**Response Size**: ~5-10 KB (5 matches)

```typescript
// Request
{
  pattern: string,        // substring match
  limit?: number,         // default: 5, max: 20
  provider?: string,
  event_type?: string     // "ToolCall" | "ToolResult" | etc
}

// Response
{
  matches: [
    {
      session_id: "fb3cff44",
      event_id: "evt_abc123",     // NEW: for drill-down
      timestamp: "2026-01-01T21:44:41Z",
      type: "ToolCall",
      summary: "Read /path/to/file.rs"  // 1-line description
    }
  ],
  total_matches: 25,
  hint: "Use get_event_details(session_id, event_id) for full payload"
}
```

**Key Change**: Returns **summaries** by default, not full payloads

---

## Tool Flow Examples

### Example 1: Debug a Failed Session

```
1. User: "Show me recent sessions"
   → list_sessions()
   → AI sees session "fb3cff44" with 6 turns

2. User: "What went wrong in fb3cff44?"
   → analyze_session("fb3cff44")
   → Score: 80, "Turn 0 and 1 had failed tools"

3. User: "Show me turn 0"
   → get_turn_details("fb3cff44", 0)
   → AI sees 12 tool calls, 1 failed

4. User: "What was the failed tool's output?"
   → get_event_details("fb3cff44", "evt_xyz")
   → Full error message: "File not found: /path/to/file.rs"
```

**Response sizes**: 10 KB → 1 KB → 30 KB → 2 KB = **43 KB total**

---

### Example 2: Investigate Tool Usage

```
1. User: "Find all times I used the Read tool"
   → search_events(pattern="Read", limit=10)
   → 10 summaries: "Read /path/to/file.rs"

2. User: "Show me the third one"
   → get_event_details("abc123", "evt_def456")
   → Full ToolCall payload with arguments

3. User: "What was the result?"
   → (AI already has event_id from search)
   → get_event_details("abc123", "evt_result_789")
   → Full file content
```

**Response sizes**: 8 KB → 2 KB → 5 KB = **15 KB total**

---

## Design Decisions Summary

### ✅ Good Patterns
1. **Layered disclosure**: summary → overview → details → full payload
2. **Event IDs**: Enable precise drill-down without re-searching
3. **Hints**: Guide AI to next action
4. **Size limits**: Each tool < 50 KB
5. **Defaults**: Conservative (limit=5-10)

### ❌ Avoided Patterns
1. ~~Single "get everything" tool~~
2. ~~Untruncated large payloads by default~~
3. ~~No way to get full details~~
4. ~~Pagination (complex for AI)~~

### 🤔 Optional Future Extensions
- `compare_sessions(id1, id2)` - Diff two sessions
- `get_reasoning_chain(session_id, turn_index)` - Just reasoning, no tools
- Field selection (`fields=["id", "timestamp"]`)

---

## Implementation Priority

### Phase 1 (MVP - Must Have)
1. ✅ `list_sessions` (already exists, needs truncation)
2. ✅ `get_project_info` (already exists)
3. ✅ `analyze_session` (already exists)
4. **NEW**: `get_session_overview` (replaces current get_session_details default)
5. **FIX**: `search_events` (return summaries, not payloads)

### Phase 2 (Deep Dive)
6. **NEW**: `get_turn_details`
7. **NEW**: `get_event_details`

### Phase 3 (Optional)
8. `compare_sessions`
9. Field selection
10. Advanced filters

---

## Response Size Targets

| Tool | Target Size | Max Size |
|------|-------------|----------|
| list_sessions (10) | 10 KB | 20 KB |
| get_session_overview | 10 KB | 30 KB |
| analyze_session | 1 KB | 5 KB |
| search_events (5) | 5 KB | 15 KB |
| get_turn_details | 30 KB | 80 KB |
| get_event_details | 2 KB | 20 KB |
| get_project_info | 10 KB | 20 KB |

**Total for typical investigation**: 40-100 KB across 3-5 tool calls
