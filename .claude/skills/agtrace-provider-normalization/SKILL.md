---
name: agtrace-provider-normalization
description: Investigate AI agent provider tool schemas (Claude Code, Codex, Gemini) and design unified domain abstractions in agtrace architecture.
---

# Agtrace Provider Normalization Expert

This skill provides deep knowledge of how agtrace normalizes diverse AI agent log formats into unified domain types. Use this when working with provider implementations, tool normalization, or understanding the schema-on-read architecture.

## Quick Reference

```bash
mise run test:providers          # Run provider tests
mise run test:types              # Run types tests
mise run lab:grep -- "pattern" --json --limit 5  # Search real event data
mise run verify                  # Full check (fmt + clippy + test + build)
```

## Three-Tier Provider Architecture

### Tier 1: Trait-Based Adapter Pattern

Every provider implements three core traits bundled in a `ProviderAdapter`:

```rust
LogDiscovery  -> File discovery and session location
SessionParser -> Raw log parsing to AgentEvent timeline
ToolMapper    -> Tool call normalization and classification
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

1. **Raw logs are source of truth** - Original files never modified
2. **Lazy parsing** - Files parsed on demand
3. **Type-safe conversion** - Raw JSON -> Provider Args -> Domain ToolCallPayload

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
    |
Provider-specific Args deserialization (strict)
    |
Optional: Semantic reclassification (e.g., shell -> Read/Write/Search)
    |
Convert to typed ToolCallPayload variant
    |
If parse fails -> Generic variant (safe fallback)
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
    tool_map: HashMap<String, Uuid>,        // Provider ID -> UUID mapping
}
```

Key features:
- **Deterministic UUIDs**: v5 UUID from session_id + "base_id:suffix"
- **Multi-stream support**: Main, Sidechain, Subagent independent chains
- **Tool result linking**: O(1) lookup from provider tool ID to UUID

## Adding a New Provider

1. Create provider module: `crates/agtrace-providers/src/<provider_name>/`
2. Implement `LogDiscovery`, `SessionParser`, `ToolMapper` traits
3. Register in `registry.rs`
4. Run `mise run test:providers` to validate
5. Run `mise run lab:grep -- "tool_name" --json --limit 5` to verify against real data
6. Run `mise run verify` before submitting PR

## Key Architectural Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Schema Location | Provider Args in providers, ToolCallPayload in types | Domain model pure |
| Parsing Errors | Fall back to Generic, never fail | No data loss |
| Tool Classification | Provider-specific first, then heuristics | Accuracy + graceful fallback |
| Event IDs | Deterministic v5 UUIDs | Reproducible sessions |
| Multi-Stream | Separate parent chains per StreamId | Independent flows |

## Testing

```bash
mise run test:providers    # Run all provider tests
mise run test:types        # Run domain type tests
mise run test              # Run full test suite
```

Each provider has tests for:
- Discovery: probe, scan, session extraction
- Parser: record parsing, content blocks
- Mapper: tool normalization per tool type
- Edge cases: malformed data, unknown tools

## When to Use This Skill

This skill should be activated when:
- Adding a new AI agent provider to agtrace
- Modifying tool normalization logic
- Understanding how provider logs become unified events
- Debugging provider-specific parsing issues
- Designing new tool classification strategies
- Working with MCP tool handling

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
