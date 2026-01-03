# MCP Integration Guide

agtrace exposes a [Model Context Protocol (MCP)](https://modelcontextprotocol.io) server that enables AI coding assistants to query agent execution history, analyze failures, and debug behavior.

## Supported MCP Clients

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini CLI** (Google)
- **Claude Desktop** (Anthropic)

All clients support both local (stdio) and remote (HTTP) MCP servers as of 2025.

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

### Gemini CLI

```bash
gemini mcp add agtrace -- agtrace mcp serve
```

Verify:
```bash
gemini mcp list
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
| `list_sessions` | Browse session history with filtering (by time, provider, project) |
| `get_session_summary` | Lightweight overview (≤5 KB): metadata, turn count, token stats |
| `get_session_turns` | Turn-level summaries with pagination (10-30 KB per page) |
| `get_turn_steps` | Detailed steps for a specific turn (20-50 KB): tool calls, results |
| `get_session_full` | Complete session data with full payloads (50-100 KB per chunk) |
| `analyze_session` | Diagnostic analysis: failures, loops, bottlenecks |
| `search_event_previews` | Search event payloads (returns ~300 char snippets) |
| `get_event_details` | Retrieve full event payload by session and index |
| `get_project_info` | List all indexed projects |

## Example Queries

Once MCP is configured, you can ask your AI assistant:

**Session exploration:**
- *"Show me sessions from the last 2 hours"*
- *"List all sessions from the my-app project"*
- *"What sessions had failures today?"*

**Event search:**
- *"Search for tool calls containing 'write_file'"*
- *"Find all reasoning events with 'refactor' in them"*
- *"Show me sessions where the agent used the Bash tool"*

**Analysis:**
- *"Analyze the most recent session for performance issues"*
- *"Check the last session for loops or repeated tool calls"*
- *"What were the failure modes in yesterday's sessions?"*

**Deep inspection:**
- *"Show me the full conversation from session abc123"*
- *"What tool calls were made in turn 5 of the current session?"*
- *"Get the complete payload for event 42 in session xyz789"*

## How It Works

1. **agtrace indexes logs** from `~/.claude/projects`, `~/.codex/sessions`, `~/.gemini/tmp`
2. **MCP server exposes tools** via `agtrace mcp serve`
3. **AI assistant calls tools** to query the index
4. **Results are returned** as structured JSON for analysis

The MCP server provides a lightweight, paginated API to prevent overwhelming the AI assistant with large payloads. Use `get_session_summary` and `get_session_turns` for quick overviews, and `get_session_full` only when you need complete data.

## Learn More

- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io)
- [agtrace CLI Documentation](./README.md)
- [MCP Tools Reference](./mcp-tools.md) *(coming soon)*
