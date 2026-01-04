# MCP Integration Guide

agtrace exposes a [Model Context Protocol (MCP)](https://modelcontextprotocol.io) server that lets AI coding assistants query their own execution history, search past sessions, and retrieve context from previous work.

## Supported MCP Clients

- **Claude Code** (Anthropic) ✅
- **Codex** (OpenAI) ✅
- **Claude Desktop** (Anthropic) ✅
- **Gemini CLI** (Google) ⚠️ Not yet supported (see Known Issues below)

## Quick Setup

### Claude Code

```bash
claude code mcp add agtrace -- agtrace mcp serve
```

Verify the server is registered:
```bash
claude code mcp list
```

### Codex (OpenAI)

```bash
codex mcp add agtrace -- agtrace mcp serve
```

Verify:
```bash
codex mcp list
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agtrace": {
      "command": "agtrace",
      "args": ["mcp", "serve"]
    }
  }
}
```

Restart Claude Desktop to apply changes.

## Troubleshooting

### Node.js Version Managers (mise, nvm, asdf, volta)

If you use a Node.js version manager, Claude Desktop may not find `node` in its PATH. Use explicit paths instead:

1. Find your Node.js path:
```bash
which node
```

2. Find your agtrace installation:
```bash
npm root -g
```

3. Update `claude_desktop_config.json` with explicit paths:
```json
{
  "mcpServers": {
    "agtrace": {
      "command": "/path/to/node",
      "args": [
        "/path/to/global/node_modules/@lanegrid/agtrace/run-agtrace.js",
        "mcp",
        "serve"
      ]
    }
  }
}
```

### Verify MCP Server is Running

Test the MCP server manually:
```bash
agtrace mcp serve --help
```

Check if agtrace is indexed:
```bash
agtrace init
agtrace sessions
```

## Available MCP Tools

| Tool | Description |
|------|-------------|
| `list_sessions` | List recent AI agent sessions with cursor-based pagination. WORKFLOW: Call this first to discover available sessions, then use session IDs with other tools. Safe to call multiple times with different filters. |
| `get_project_info` | List all projects that have been indexed by agtrace with their metadata. WORKFLOW: Use this to discover available projects and their hashes. Safe to call anytime. |
| `analyze_session` | Run diagnostic analysis on a session to identify failures, loops, and issues. WORKFLOW: First call list_sessions to obtain session IDs, then use those IDs with this tool. Safe to call in parallel for multiple known session IDs. |
| `search_events` | Search for events and return navigation coordinates (session_id, event_index, turn_index, step_index). Use this to find specific events, then use turn_index with list_turns or get_turns for detailed analysis. |
| `list_turns` | List turns with metadata only (no payload content). Returns turn statistics including step_count, duration_ms, total_tokens, and tools_used. Use this to get an overview before drilling down with get_turns. |
| `get_turns` | Get details for specific turns. Defaults are tuned for safety based on data distribution (max 30 steps/turn, 3000 chars/field). WORKFLOW: Fetch 1-2 turns at a time to avoid token limits. If data is marked '[TRUNCATED]' and critical, retry with higher limits. |

## Real-World Example: Agent Self-Reflection

Here's an actual workflow showing how agents use agtrace MCP for context-aware decision making:

**User Request:**
> "Read previous sessions and help me reconsider a design decision we made earlier."

**Agent Workflow (9 steps, 491 seconds):**

1. **Discover project history** (`list_sessions`):
   ```json
   {"project_root": "/path/to/project", "limit": 10}
   ```
   → Found 5 recent sessions, identified relevant discussion

2. **Understand past decisions** (`list_turns` → `get_turns`):
   ```json
   {"session_id": "cc7fe4ef", "turn_indices": [0, 1, 2]}
   ```
   → Retrieved the original reasoning and implementation details

3. **Search for related changes** (`search_events`):
   ```json
   {"query": "deprecated_feature", "event_type": "ToolCall"}
   ```
   → Examined 34KB of discussion across 3 turns

4. **Create informed response**:
   - Generated comprehensive specification based on past context
   - Addressed why the original decision was made
   - Proposed migration path that respects historical constraints

**Token efficiency:** 334,872 tokens total, with prompt caching reducing costs by 85%

**Key Insight:** Without MCP, the agent would lack:
- Why the original decision was made
- What alternatives were considered
- What constraints shaped the design

This is **Agent Self-Reflection**: understanding history to make better future decisions.

---

## Example Queries

Once MCP is configured, you can ask your AI assistant:

**Context-aware development:**
- *"Read previous sessions for this project and understand why we deprecated feature X"*
- *"What did we decide about error handling in the last session?"*
- *"Show me how we implemented similar features before"*

**Session exploration:**
- *"Show me sessions from the last 2 hours"*
- *"List all sessions from the my-app project"*
- *"What sessions had failures today?"*

**Turn-level analysis:**
- *"List turns from the most recent session and show me what tools were used"*
- *"Get details for turn 3 from session abc123"*
- *"Show me the reasoning from the first few turns of that session"*

**Event search:**
- *"Search for events containing 'write_file'"*
- *"Find all reasoning events with 'refactor' in them"*
- *"Show me where the agent used the Bash tool"*

**Analysis:**
- *"Analyze the most recent session for performance issues"*
- *"Check the last session for loops or repeated tool calls"*
- *"What were the failure modes in yesterday's sessions?"*

## How It Works

1. **agtrace indexes logs** from `~/.claude/projects`, `~/.codex/sessions`, `~/.gemini/tmp`
2. **MCP server exposes tools** via `agtrace mcp serve`
3. **AI assistant calls tools** to query the index
4. **Results are returned** as structured JSON for analysis

The MCP server provides a lightweight, paginated API to prevent overwhelming the AI assistant with large payloads. Use `list_sessions` to discover sessions, `list_turns` for turn-level overviews, and `get_turns` when you need detailed turn data. Use `search_events` to find specific events across sessions.

## Known Issues

### Gemini CLI Not Supported

Gemini CLI currently does not connect to agtrace MCP server. This is because:

1. **Transport framing mismatch**: agtrace uses newline-delimited JSON-RPC (`{json}\n`), while Gemini CLI strictly requires Content-Length framing (`Content-Length: XXX\r\n\r\n{json}`)
2. **MCP_STDIO_MODE not supported**: Gemini CLI does not respect the `MCP_STDIO_MODE=nl` environment variable to enable newline-delimited mode

**Workaround**: None currently available.

**Fix plan**: Implement Content-Length framing support in agtrace MCP server (tracked in issue #TBD).

## Learn More

- [Model Context Protocol Specification](https://modelcontextprotocol.io/specification)
- [agtrace CLI Documentation](./README.md)
