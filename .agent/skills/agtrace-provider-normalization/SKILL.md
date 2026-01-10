---
name: agtrace-provider-normalization
description: Investigate AI agent provider tool schemas (Claude Code, Codex, Gemini) and design unified domain abstractions in agtrace architecture.
---

# Agtrace Provider Normalization Expert

This skill provides deep knowledge of how agtrace normalizes diverse AI agent log formats into unified domain types. Use this when working with provider implementations, tool normalization, or understanding the schema-on-read architecture.

## Three-Tier Provider Architecture

### Tier 1: Trait-Based Adapter Pattern

Every provider implements three core traits bundled in a `ProviderAdapter`:

```rust
LogDiscovery  → File discovery and session location
SessionParser → Raw log parsing to AgentEvent timeline
ToolMapper    → Tool call normalization and classification
```

**Key Files:**
- `crates/agtrace-providers/src/traits.rs` - trait definitions
- `crates/agtrace-providers/src/registry.rs` - adapter factory

### Tier 2: Provider-Specific Implementation

Each provider has this structure:
```
provider/
├── discovery.rs      # LogDiscovery trait impl
├── parser.rs         # SessionParser trait impl
├── mapper.rs         # ToolMapper trait impl
├── io.rs             # Raw file I/O
├── schema.rs         # Provider-specific schemas
├── tools.rs          # Provider-specific tool args
├── tool_mapping.rs   # Tool classification
└── models.rs         # Data structures
```

**Provider directories:**
- `crates/agtrace-providers/src/claude/`
- `crates/agtrace-providers/src/codex/`
- `crates/agtrace-providers/src/gemini/`

### Tier 3: Unified Domain Types

All providers normalize to common types in `agtrace-types`:

| Type | Location | Variants |
|------|----------|----------|
| `EventPayload` | `event/payload.rs` | User, Reasoning, ToolCall, ToolResult, Message, TokenUsage, Notification, SlashCommand |
| `ToolCallPayload` | `tool/payload.rs` | FileRead, FileEdit, FileWrite, Execute, Search, Mcp, Generic |
| `ToolKind` | `tool/types.rs` | Read, Write, Execute, Plan, Search, Ask, Other |
| `ToolOrigin` | `tool/types.rs` | System, Mcp |

## Schema-on-Read Architecture

The architecture enforces Schema-on-Read where:

1. **Raw logs are source of truth** - Original files never modified
2. **Lazy parsing** - Files parsed on demand
3. **Type-safe conversion** - Raw JSON → Provider Args → Domain ToolCallPayload

### Provider-Specific Args Pattern

Each provider defines schema-specific argument types:

```rust
// Claude: crates/agtrace-providers/src/claude/tools.rs
ClaudeReadArgs, ClaudeGlobArgs, ClaudeEditArgs, ClaudeWriteArgs,
ClaudeBashArgs, ClaudeGrepArgs, ClaudeTodoWriteArgs

// Codex: crates/agtrace-providers/src/codex/tools.rs
ShellArgs, ApplyPatchArgs, ReadMcpResourceArgs

// Gemini: crates/agtrace-providers/src/gemini/tools.rs
GeminiReadFileArgs, GeminiWriteFileArgs, GeminiReplaceArgs,
GeminiRunShellCommandArgs, GeminiGoogleWebSearchArgs
```

Each has conversion methods:
```rust
impl ClaudeReadArgs {
    fn to_file_read_args(&self) -> FileReadArgs { ... }
}
```

## Tool Call Normalization Flow

```
Raw JSON Arguments
    ↓
Provider-specific Args deserialization (strict)
    ↓
Optional: Semantic reclassification (e.g., shell → Read/Write/Search)
    ↓
Convert to typed ToolCallPayload variant
    ↓
If parse fails → Generic variant (safe fallback)
```

### Semantic Reclassification Example

Codex `shell` commands are intelligently classified:

```rust
// File: codex/mapper.rs
match tool_name {
    "shell" => {
        if let Ok(shell_args) = parse::<ShellArgs>(arguments) {
            match classify_execute_command(&command) {
                Some(ToolKind::Search) => return Search variant
                Some(ToolKind::Read) => return FileRead variant
                _ => return Execute variant
            }
        }
    }
}
```

## MCP Tool Handling

Different providers use different MCP naming conventions:

| Provider | Format | Example |
|----------|--------|---------|
| Claude | `mcp__server__tool` | `mcp__filesystem__read_file` |
| Codex | `mcp__server__tool` | `mcp__memory__store` |
| Gemini | `tool-name` + display_name | Display: "(ServerName MCP Server)" |

All normalize to `ToolCallPayload::Mcp` with `McpArgs`:
```rust
pub struct McpArgs {
    pub server: Option<String>,
    pub tool: Option<String>,
    pub inner: Value,
}
```

## Event Building

The `EventBuilder` in `builder.rs` provides deterministic event construction:

```rust
pub struct EventBuilder {
    session_id: Uuid,
    stream_tips: HashMap<StreamId, Uuid>,  // Per-stream parent tracking
    tool_map: HashMap<String, Uuid>,        // Provider ID → UUID mapping
}
```

Key features:
- **Deterministic UUIDs**: v5 UUID from session_id + "base_id:suffix"
- **Multi-stream support**: Main, Sidechain, Subagent independent chains
- **Tool result linking**: O(1) lookup from provider tool ID to UUID

## Adding a New Provider

1. **Create provider module**: `crates/agtrace-providers/src/<provider_name>/`

2. **Implement Discovery**:
```rust
pub struct NewProviderDiscovery;

impl LogDiscovery for NewProviderDiscovery {
    fn id(&self) -> &'static str { "new_provider" }
    fn probe(&self, path: &Path) -> ProbeResult { ... }
    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> { ... }
}
```

3. **Implement Parser**:
```rust
pub struct NewProviderParser;

impl SessionParser for NewProviderParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> { ... }
    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>> { ... }
}
```

4. **Implement ToolMapper**:
```rust
pub struct NewProviderToolMapper;

impl ToolMapper for NewProviderToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) { ... }
    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>)
        -> ToolCallPayload { ... }
    fn summarize(&self, kind: ToolKind, args: &Value) -> String { ... }
}
```

5. **Register in registry.rs**:
```rust
pub fn create_adapter(name: &str) -> Result<ProviderAdapter> {
    match name {
        "new_provider" => Ok(ProviderAdapter::new(
            Box::new(crate::new_provider::NewProviderDiscovery),
            Box::new(crate::new_provider::NewProviderParser),
            Box::new(crate::new_provider::NewProviderToolMapper),
        )),
        // ...
    }
}
```

## Tool Classification Pattern

### Provider-Specific Classification

Each provider defines in `tool_mapping.rs`:
```rust
fn classify_tool(tool_name: &str) -> Option<(ToolOrigin, ToolKind)>
```

Returns `Some` if recognized, `None` for fallback.

### Common Heuristic Fallback

When provider doesn't recognize a tool (`tool_analyzer.rs`):
```rust
pub fn classify_common(tool_name: &str) -> (ToolOrigin, ToolKind) {
    // name.contains("search") → ToolKind::Search
    // name.contains("read") → ToolKind::Read
    // name.starts_with("mcp__") → ToolOrigin::Mcp
}
```

## Key Architectural Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Schema Location | Provider Args in providers, ToolCallPayload in types | Domain model pure |
| Parsing Errors | Fall back to Generic, never fail | No data loss |
| Tool Classification | Provider-specific first, then heuristics | Accuracy + graceful fallback |
| Event IDs | Deterministic v5 UUIDs | Reproducible sessions |
| Multi-Stream | Separate parent chains per StreamId | Independent flows |

## Testing Patterns

Each provider has tests for:
- Discovery: probe, scan, session extraction
- Parser: record parsing, content blocks
- Mapper: tool normalization per tool type
- Edge cases: malformed data, unknown tools

```rust
#[test]
fn test_normalize_read() {
    let payload = normalize_claude_tool_call(
        "Read".to_string(),
        serde_json::json!({"file_path": "src/main.rs"}),
        Some("call_123".to_string()),
    );

    match payload {
        ToolCallPayload::FileRead { arguments, .. } => {
            assert_eq!(arguments.file_path, Some("src/main.rs".to_string()));
        }
        _ => panic!("Expected FileRead"),
    }
}
```

## When to Use This Skill

This skill should be activated when:
- Adding a new AI agent provider to agtrace
- Modifying tool normalization logic
- Understanding how provider logs become unified events
- Debugging provider-specific parsing issues
- Designing new tool classification strategies
- Working with MCP tool handling
- Understanding the schema-on-read architecture

## Key Files Reference

| Purpose | File Path |
|---------|-----------|
| Trait definitions | `crates/agtrace-providers/src/traits.rs` |
| Provider registry | `crates/agtrace-providers/src/registry.rs` |
| Event builder | `crates/agtrace-providers/src/builder.rs` |
| Claude parser | `crates/agtrace-providers/src/claude/parser.rs` |
| Claude tools | `crates/agtrace-providers/src/claude/tools.rs` |
| Codex parser | `crates/agtrace-providers/src/codex/parser.rs` |
| Gemini parser | `crates/agtrace-providers/src/gemini/parser.rs` |
| Domain events | `crates/agtrace-types/src/event/payload.rs` |
| Tool payload | `crates/agtrace-types/src/tool/payload.rs` |
| Tool types | `crates/agtrace-types/src/tool/types.rs` |
