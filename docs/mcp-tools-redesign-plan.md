# MCP Tools Redesign Implementation Plan

**Status**: Breaking changes allowed (pre-release)
**Approach**: Option B (Tool Split) + All Phase 1-3 Improvements

---

## New Tool Inventory

### Current (6 tools)
1. `list_sessions` - ✅ Keep (with improvements)
2. `get_session_details` - ✅ Keep (with improvements)
3. `analyze_session` - ✅ Keep (no changes)
4. `search_events` - ❌ **Split into 2 tools**
5. `get_project_info` - ✅ Keep (no changes)

### After Redesign (7 tools)
1. `list_sessions` - Improved parameter names, enum types
2. `get_session_details` - Better documentation, validation
3. `analyze_session` - No changes
4. `search_event_previews` - 🆕 New (breadth search)
5. `get_event_details` - 🆕 New (depth retrieval)
6. `get_project_info` - No changes

---

## New Type System

### 1. Structured Errors
```rust
// crates/agtrace-cli/src/mcp/dto/error.rs
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

pub enum ErrorCode {
    InvalidSessionId,
    AmbiguousSessionPrefix,
    SessionNotFound,
    InvalidEventIndex,
    InvalidCursor,
    InvalidParameter,
    SearchTimeout,
}
```

### 2. Standardized Response Wrapper
```rust
// crates/agtrace-cli/src/mcp/dto/response/common.rs
pub struct McpResponse<T> {
    pub data: T,
    pub pagination: Option<PaginationMeta>,
    pub hint: Option<String>,
}

pub struct PaginationMeta {
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}
```

### 3. Event Type Enum
```rust
// crates/agtrace-cli/src/mcp/dto/common.rs
pub enum EventType {
    ToolCall,
    ToolResult,
    Message,
    User,
    Reasoning,
    TokenUsage,
    Notification,
}
```

---

## Tool Changes Detail

### `search_event_previews` (New)

**Purpose**: Quick search across event payloads (returns previews only)

**Request**:
```rust
pub struct SearchEventPreviewsArgs {
    /// Search query (substring match in event JSON)
    pub query: String,

    /// Maximum results per page (default: 10, max: 50)
    pub limit: Option<usize>,

    /// Pagination cursor from previous response
    pub cursor: Option<String>,

    /// Filter by provider (claude_code, codex, gemini)
    pub provider: Option<String>,

    /// Filter by event type
    pub event_type: Option<EventType>,

    /// Search within specific session only
    pub session_id: Option<String>,
}
```

**Response**:
```rust
pub struct SearchEventPreviewsResponse {
    pub matches: Vec<EventPreview>,
}

pub struct EventPreview {
    pub session_id: String,
    pub event_index: usize,  // For use with get_event_details
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub preview: PreviewContent,  // 300 char snippet
}
```

**Wrapped in**: `McpResponse<SearchEventPreviewsResponse>`

---

### `get_event_details` (New)

**Purpose**: Retrieve full event payload by coordinates

**Request**:
```rust
pub struct GetEventDetailsArgs {
    /// Session ID (8-char prefix or full UUID)
    pub session_id: String,

    /// Zero-based event index within session
    pub event_index: usize,
}
```

**Response**:
```rust
pub struct EventDetailsResponse {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,  // Full, untruncated
}
```

**No pagination** (single event response)

---

### `list_sessions` (Improved)

**Changes**:
```diff
pub struct ListSessionsArgs {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
+   // Rename for clarity
-   pub provider: Option<String>,
+   pub provider: Option<Provider>,  // Enum

    pub project_hash: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

+pub enum Provider {
+    ClaudeCode,
+    Codex,
+    Gemini,
+}
```

**Response**: Wrap in `McpResponse<ListSessionsData>`
```rust
pub struct ListSessionsData {
    pub sessions: Vec<SessionSummaryDto>,
}
```

---

### `get_session_details` (Improved)

**Changes**:
```diff
pub struct GetSessionDetailsArgs {
-   /// Session ID (short or full UUID)
+   /// Session ID: 8-character prefix (e.g., "fb3cff44") or full UUID.
+   /// Ambiguous prefixes matching multiple sessions will return an error.
    pub session_id: String,

-   /// Detail level: 'summary' (5-10KB), ...
+   /// Response size control:
+   /// - 'summary': 5-10 KB (turn count, no payloads)
+   /// - 'turns': 15-30 KB (tool usage per turn)
+   /// - 'steps': 50-100 KB (truncated payloads, 500 chars)
+   /// - 'full': Unbounded (complete session, use with caution)
+   /// Default: 'summary'
    pub detail_level: Option<DetailLevel>,

-   /// Include reasoning/thinking content in summaries (only applies to 'turns' level)
+   /// Include <thinking> blocks in turn summaries.
+   /// Only applies when detail_level='turns'. Ignored for other levels.
+   /// Adds ~5-10 KB per turn with reasoning content.
+   /// Default: false
    pub include_reasoning: Option<bool>,
}
```

**Response**: No wrapper (varies by detail_level, keep as-is)

---

## Implementation Checklist

### Phase 1: Foundation Types
- [ ] Create `crates/agtrace-cli/src/mcp/dto/error.rs`
  - [ ] `McpError` struct
  - [ ] `ErrorCode` enum
  - [ ] Conversion from `crate::Error`
- [ ] Update `crates/agtrace-cli/src/mcp/dto/common.rs`
  - [ ] `EventType` enum
  - [ ] `Provider` enum
  - [ ] `PaginationMeta` struct
  - [ ] `McpResponse<T>` wrapper

### Phase 2: Request/Response DTOs
- [ ] Create `search_event_previews` types in `request.rs`
- [ ] Create `get_event_details` types in `request.rs`
- [ ] Update `list_sessions` to use `Provider` enum
- [ ] Improve `get_session_details` documentation
- [ ] Create new response types in `response/` folder

### Phase 3: Tool Handlers
- [ ] Implement `handle_search_event_previews` in `tools.rs`
- [ ] Implement `handle_get_event_details` in `tools.rs`
- [ ] Update `handle_list_sessions` to use new wrapper
- [ ] Remove old `handle_search_events`

### Phase 4: Server Integration
- [ ] Update `server.rs` tool registration
- [ ] Remove `search_events` from tool list
- [ ] Add `search_event_previews` and `get_event_details`

### Phase 5: Testing
- [ ] Unit tests for error serialization
- [ ] Schema tests for new enums
- [ ] Integration test: search → get_event_details flow
- [ ] Verify pagination with cursors

### Phase 6: Documentation
- [ ] Update `docs/mcp-tools-interface.md`
- [ ] Add migration guide (old → new tools)
- [ ] Update README examples

---

## Breaking Changes Summary

### Removed
- ❌ `search_events` tool (replaced by 2 tools)

### Added
- ✅ `search_event_previews` tool
- ✅ `get_event_details` tool
- ✅ `EventType` enum
- ✅ `Provider` enum
- ✅ Structured error responses
- ✅ `McpResponse<T>` wrapper for list operations

### Modified
- 🔄 `list_sessions`: `provider` now enum instead of string
- 🔄 `list_sessions`: Response wrapped in `McpResponse<T>`
- 🔄 `get_session_details`: Improved parameter descriptions
- 🔄 All tools: Return `McpError` on failure instead of string

---

## Migration Example

### Before (Old API)
```typescript
// Search with full payload
search_events({
  pattern: "Read",
  include_full_payload: true,
  limit: 5
})
```

### After (New API)
```typescript
// Step 1: Search for previews
search_event_previews({
  query: "Read",
  limit: 10
})
// Returns: { data: { matches: [{session_id, event_index, preview}, ...] }, pagination: {...} }

// Step 2: Get full payload for specific event
get_event_details({
  session_id: "fb3cff44",
  event_index: 42
})
// Returns: { session_id, event_index, payload: {...} }
```

**Why better?**
- Clear separation of concerns
- Each response < 30 KB
- Progressive disclosure
- Composable tools
