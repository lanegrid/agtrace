# MCP Tools Schema Review: Best Practices Analysis

## Executive Summary

Based on MCP best practices research (2024-2025), this document reviews `search_events` and `get_session_details` against industry standards and proposes specific improvements.

## Best Practices Framework

### 1. Schema Design Principles (from o3 research)

**Core Rules:**
- **Minimal schemas**: Only expose parameters models truly need
- **Explicit types**: Use enums over strings for closed domains
- **Required vs Optional**: Make requirements explicit; never rely on model defaults
- **Inline documentation**: "description" fields are part of the prompt—write for models
- **Consistent naming**: Use snake_case consistently; avoid implementation details

**Tool Granularity:**
- One tool = one conceptual operation
- Favor narrow, composable tools over kitchen-sink designs
- Split when intents/auth/responses differ fundamentally
- Target < 8 parameters per tool

**Response Structure:**
- Consistent top-level wrappers across all tools
- Stable error location: `{ "status": "error", "error": {...} }`
- Include pagination metadata (`next_cursor`, `has_more`)
- Document guarantees (ordering, max size)

---

## Current Implementation Analysis

### `search_events` Tool

#### Current Schema
```rust
pub struct SearchEventsArgs {
    pub pattern: String,                      // required
    pub limit: Option<usize>,                 // default: 5, max: 20
    pub cursor: Option<String>,               // pagination
    pub provider: Option<String>,             // filter
    pub event_type: Option<String>,           // filter
    pub include_full_payload: Option<bool>,   // default: false
}
```

#### Issues vs Best Practices

| Issue | Severity | Best Practice Violation |
|-------|----------|------------------------|
| Parameter name `pattern` is vague | Medium | "Make names model-aligned (semantic) rather than implementation-aligned" |
| `include_full_payload` boolean toggle | High | "If parameter sets are mutually exclusive, split tools" / Progressive disclosure |
| `event_type` as free-form string | Medium | "Use enums instead of strings for closed domains" |
| 6 parameters (acceptable but borderline) | Low | "Target < 8 parameters" |
| No `session_id` filter option | Medium | Missing useful composition |

#### Response Structure
```rust
pub struct SearchEventsResponse {
    pub matches: Vec<EventMatchDto>,
    pub total: usize,                  // ❌ Ambiguous: page total or global total?
    pub next_cursor: Option<String>,
    pub hint: String,
}

pub enum EventMatchDto {
    Snippet { ... },  // When include_full_payload=false
    Full { ... },     // When include_full_payload=true
}
```

**Issues:**
- `total` field is ambiguous—does it mean total in page or total matches across all pages?
- Using `#[serde(untagged)]` for `EventMatchDto` means no explicit type discriminator in JSON
- `hint` is good but inconsistently applied across tools

---

### `get_session_details` Tool

#### Current Schema
```rust
pub struct GetSessionDetailsArgs {
    pub session_id: String,                   // required
    pub detail_level: Option<DetailLevel>,    // default: summary
    pub include_reasoning: Option<bool>,      // default: false
}

pub enum DetailLevel {
    Summary,  // 5-10 KB
    Turns,    // 15-30 KB
    Steps,    // 50-100 KB
    Full,     // unbounded
}
```

#### Issues vs Best Practices

| Issue | Severity | Best Practice Violation |
|-------|----------|------------------------|
| `include_reasoning` only applies to `Turns` level | High | "Hidden coupling between fields" / "Call out hidden requirements" |
| `DetailLevel` as enum is good ✅ | N/A | Follows best practice |
| No explicit size warnings in schema | Low | Missing safety metadata |
| `session_id` accepts prefix or full UUID | Medium | Implicit normalization—should document |

**Description Quality Issue:**
Current: "Include reasoning/thinking content in summaries (only applies to 'turns' level)"

Better (following "natural language contract" principle):
```
Include <thinking> blocks in turn summaries.
Only valid when detail_level='turns'.
Ignored for other levels.
Adds ~5-10 KB per turn with thinking content.
```

---

## Proposed Improvements

### Option A: Minimal Changes (Conservative)

Keep current tools but improve parameter clarity:

#### `search_events` Changes

```diff
pub struct SearchEventsArgs {
-   pub pattern: String,
+   /// Search query (substring match in event JSON payloads)
+   pub query: String,

    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub provider: Option<String>,

-   pub event_type: Option<String>,
+   /// Filter by event type (e.g., "ToolCall", "ToolResult", "Message")
+   #[schemars(example = "event_type_examples")]
+   pub event_type: Option<EventType>,  // Use enum

-   pub include_full_payload: Option<bool>,
+   /// Response format: 'preview' (default, ~300 chars) or 'full' (complete event)
+   pub response_format: Option<SearchResponseFormat>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ToolCall,
    ToolResult,
    Message,
    User,
    Reasoning,
    TokenUsage,
    Notification,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchResponseFormat {
    Preview,  // default
    Full,
}
```

#### `get_session_details` Changes

```diff
pub struct GetSessionDetailsArgs {
-   /// Session ID (short or full UUID)
+   /// Session ID (8-char prefix like "fb3cff44" or full UUID).
+   /// Ambiguous prefixes will return an error.
    pub session_id: String,

-   /// Detail level: 'summary' (5-10KB), 'turns' (15-30KB), 'steps' (50-100KB), or 'full' (complete data)
+   /// Detail level controls response size:
+   /// - 'summary': 5-10 KB (turn list, no tools)
+   /// - 'turns': 15-30 KB (tool summaries per turn)
+   /// - 'steps': 50-100 KB (full structure, truncated payloads)
+   /// - 'full': Unbounded (complete session, use with caution)
+   /// Default: 'summary'
    pub detail_level: Option<DetailLevel>,

-   /// Include reasoning/thinking content in summaries (only applies to 'turns' level)
+   /// Include <thinking> blocks in turn summaries.
+   /// WARNING: Only valid when detail_level='turns'. Ignored otherwise.
+   /// Adds ~5-10 KB per turn with thinking content.
+   /// Default: false
    pub include_reasoning: Option<bool>,
}
```

---

### Option B: Split `search_events` (Recommended)

Following "favor narrow, composable tools" principle:

#### New Tool: `search_event_previews`
```rust
/// Quick search across event payloads (returns previews only)
pub struct SearchEventPreviewsArgs {
    /// Search query (substring match)
    pub query: String,
    pub limit: Option<usize>,     // default: 10, max: 50
    pub cursor: Option<String>,
    pub provider: Option<String>,
    pub event_type: Option<EventType>,
    pub session_id: Option<String>,  // NEW: search within session
}

// Response: ~5-10 KB per page
pub struct SearchEventPreviewsResponse {
    pub matches: Vec<EventPreview>,
    pub total_in_page: usize,    // CLARIFIED: count in this page
    pub next_cursor: Option<String>,
    pub hint: "Use get_event_details(session_id, event_index) for full payload",
}
```

#### New Tool: `get_event_details`
```rust
/// Retrieve full event payload by session and index
pub struct GetEventDetailsArgs {
    pub session_id: String,
    pub event_index: usize,  // or event_id: String
}

// Response: 1-5 KB (single event)
pub struct EventDetailsResponse {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,  // Full, untruncated
}
```

**Rationale:**
- Separates "search" (breadth) from "retrieve" (depth)
- Eliminates `include_full_payload` toggle
- Each tool < 6 parameters
- Clear intent: one for discovery, one for detail

---

### Option C: Add `detail_level` to `search_events` (Align with `get_session_details`)

Make progressive disclosure pattern consistent across tools:

```rust
pub struct SearchEventsArgs {
    pub query: String,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub provider: Option<String>,
    pub event_type: Option<EventType>,

    /// Response detail level:
    /// - 'preview': Event metadata + 300-char snippet (default)
    /// - 'compact': Metadata + key fields (e.g., tool name, args summary)
    /// - 'full': Complete event payload
    pub detail_level: Option<SearchDetailLevel>,
}

pub enum SearchDetailLevel {
    Preview,   // ~1 KB per event
    Compact,   // ~2-3 KB per event
    Full,      // ~5-10 KB per event
}
```

**Pros:**
- Consistent with `get_session_details` API
- Allows for future intermediate levels
- Single parameter instead of boolean toggle

**Cons:**
- More complex than Option B's tool split
- Still 6 parameters

---

## Response Structure Improvements

### Current Issue: Inconsistent Top-Level Schema

Different tools return different shapes:

```typescript
// list_sessions
{ sessions: [...], total_in_page: 10, next_cursor: "...", hint: "..." }

// search_events
{ matches: [...], total: 5, next_cursor: "...", hint: "..." }

// get_session_details
{ /* varies by detail_level */ }
```

### Recommended: Standardized Wrapper

```rust
/// Standard MCP response wrapper
#[derive(Debug, Serialize)]
pub struct McpResponse<T> {
    pub data: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,  // e.g., "Response truncated to 100 KB"
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
    pub has_more: bool,  // Explicit flag for quick branching
}
```

**Benefits:**
- Consistent structure across all tools
- Easy to add metadata (warnings, deprecation notices)
- `has_more` flag simplifies LLM logic
- Matches o3 research: "consistent top-level wrappers"

---

## Error Handling Improvements

### Current Gap: No Structured Error Schema

Rust `Result<Value, String>` returns free-text errors.

### Recommended: Structured Errors

```rust
#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,  // Human-readable
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidSessionId,
    AmbiguousSessionPrefix,
    SessionNotFound,
    InvalidDetailLevel,
    InvalidCursor,
    SearchTimeout,
    RateLimitExceeded,
}
```

**Response format:**
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

---

## Recommended Action Plan

### Phase 1: Low-Risk Improvements (This Week)
1. ✅ Rename `pattern` → `query` in `search_events`
2. ✅ Convert `event_type` to enum
3. ✅ Improve parameter descriptions (add size warnings, edge cases)
4. ✅ Clarify `total` → `total_in_page` in responses
5. ✅ Add `has_more` boolean to pagination responses

### Phase 2: Medium-Risk Refactoring (Next Sprint)
1. Replace `include_full_payload` with `detail_level` enum in `search_events`
2. Implement standardized `McpResponse<T>` wrapper
3. Add structured error responses
4. Add `session_id` filter to `search_events`

### Phase 3: Breaking Changes (Future)
1. Consider splitting `search_events` → `search_event_previews` + `get_event_details`
2. Version tools (add `_v2` suffix or schema_version field)
3. Deprecate old versions with sunset date in descriptions

---

## Appendix: Best Practice Checklist

Applied to `search_events` and `get_session_details`:

- [x] Tool does one coherent task
- [x] ≤ 8 parameters
- [ ] Each parameter has explicit type (missing enums)
- [x] Required params marked clearly
- [x] Response has stable schema
- [x] Pagination implemented
- [ ] Errors use structured schema (current: free-text)
- [ ] Consistent top-level wrapper (current: varies)
- [x] Documentation with examples
- [ ] Safety metadata (size warnings in descriptions)
- [ ] No hidden field coupling (violates: `include_reasoning` + `detail_level`)

**Score: 7/11** — Room for improvement, especially in error handling and schema consistency.

---

## References

1. MCP Specification 2024-11-05: Pagination
2. o3 Search Results: Tool Schema Best Practices (2024-2025)
3. Internal: `docs/mcp-tools-interface.md`
