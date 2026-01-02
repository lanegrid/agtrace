# AgTrace MCP Server

The AgTrace MCP (Model Context Protocol) Server enables AI agents like Claude Desktop to query historical sessions, analyze failures, search event payloads, and debug agent behavior through a standardized protocol.

## Overview

The MCP server exposes agtrace's observability capabilities as tools that AI agents can call to introspect their own execution history. This creates a "self-debugging" workflow where agents can analyze past sessions to understand failures, identify patterns, and improve their behavior.

## Installation

Ensure you have agtrace installed:

```bash
cargo install agtrace
# or
npm install -g @lanegrid/agtrace
```

Initialize agtrace to index your agent sessions:

```bash
agtrace init
```

## Configuration

### Claude Desktop

Add agtrace to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "agtrace": {
      "command": "agtrace",
      "args": ["serve"]
    }
  }
}
```

Restart Claude Desktop to load the MCP server.

## Available Tools

The MCP server provides the following tools:

### `list_sessions`

List recent AI agent sessions with optional filtering.

**Parameters**:
- `limit` (number, optional): Maximum sessions to return (default: 10, max recommended: 50)
- `provider` (string, optional): Filter by provider (claude_code, codex, gemini)
- `project_hash` (string, optional): Filter by project hash
- `since` (string, optional): Show sessions after this timestamp
- `until` (string, optional): Show sessions before this timestamp

**Note**: Session snippets are truncated to 200 characters to prevent large responses.

**Example**:
```json
{
  "limit": 10,
  "provider": "claude_code"
}
```

### `get_session_details`

Get session details with configurable verbosity to control response size.

**Parameters**:
- `session_id` (string, required): Session ID (8-char prefix or full UUID)
- `detail_level` (string, optional): Response verbosity - `summary` (5-10 KB), `turns` (15-30 KB), `steps` (50-100 KB), or `full` (unbounded). Default: `summary`
- `include_reasoning` (boolean, optional): Include thinking blocks in turn summaries (only applies to `turns` level). Default: `false`

**Example**:
```json
{
  "session_id": "fb3cff44",
  "detail_level": "turns"
}
```

### `analyze_session`

Run diagnostic analysis on a session to identify failures, infinite loops, and other issues. Returns a health score (0-100) and detailed insights about problematic turns.

**Parameters**:
- `session_id` (string, required): Session ID to analyze
- `include_failures` (boolean, optional): Include failure analysis (default: true)
- `include_loops` (boolean, optional): Include loop detection (default: false)

**Example**:
```json
{
  "session_id": "abc123",
  "include_failures": true,
  "include_loops": true
}
```

### `search_event_previews`

Search for patterns in event payloads across sessions, returning lightweight previews (~300 char snippets). Use with `get_event_details` for full payloads.

**Parameters**:
- `query` (string, required): Search query (substring match in event JSON)
- `limit` (number, optional): Maximum matches to return (default: 10, max: 50)
- `cursor` (string, optional): Pagination cursor from previous response
- `provider` (string, optional): Filter by provider (`claude_code`, `codex`, `gemini`)
- `event_type` (string, optional): Filter by event type (`ToolCall`, `ToolResult`, `Message`, `User`, `Reasoning`, `TokenUsage`, `Notification`)
- `session_id` (string, optional): Search within specific session only

**Example**:
```json
{
  "query": "Read",
  "event_type": "ToolCall",
  "limit": 10
}
```

**Response includes**:
- Event previews with `event_index` for retrieval
- Pagination metadata (`next_cursor`, `has_more`)
- Hint to use `get_event_details` for full payloads

### `get_event_details`

Retrieve full event payload by session ID and event index (obtained from `search_event_previews`).

**Parameters**:
- `session_id` (string, required): Session ID (8-char prefix or full UUID)
- `event_index` (number, required): Zero-based event index within session

**Example**:
```json
{
  "session_id": "fb3cff44",
  "event_index": 42
}
```

**Response includes**: Complete event with timestamp, type, and full untruncated payload.

### `get_project_info`

List all projects that have been indexed by agtrace with their metadata.

**Parameters**: None

## Usage Examples

Once configured, you can ask Claude Desktop questions like:

- *"Show me my recent sessions"*
- *"Analyze the last failed session and tell me what went wrong"*
- *"Search for all Read tool calls and show me the first one in detail"*
- *"What sessions have I run in this project?"*
- *"Find tool failures in session fb3cff44"*

Claude will automatically use the appropriate agtrace tools to answer these questions by querying your indexed session history.

### Progressive Disclosure Workflow

For efficient context usage, follow this pattern:

1. **Discover**: Use `search_event_previews` to find relevant events
2. **Drill Down**: Use `get_event_details` to retrieve full payloads for specific events
3. **Analyze**: Use `analyze_session` for health diagnostics
4. **Context**: Use `get_session_details` with appropriate `detail_level`

Example conversation:
```
User: "Find errors in my last session"
→ search_event_previews(query="error") returns preview with event_index: 87
→ get_event_details(session_id="fb3cff44", event_index=87) returns full error
```

## Architecture

The MCP server:
1. Reads JSON-RPC requests from stdin
2. Dispatches to agtrace-sdk APIs
3. Returns responses via stdout

This follows the MCP protocol specification for stdio-based servers.

## Troubleshooting

### Server not appearing in Claude Desktop

1. Check that your `claude_desktop_config.json` is valid JSON
2. Ensure `agtrace` is in your PATH: `which agtrace`
3. Restart Claude Desktop completely
4. Check Claude Desktop logs for errors

### No sessions found

1. Run `agtrace init` to index your sessions
2. Verify sessions exist: `agtrace sessions`
3. Check provider configuration: `agtrace provider list`

### Tools returning errors

1. Ensure agtrace database is initialized: `agtrace init`
2. Check that session IDs are valid: `agtrace sessions`
3. Verify the tool parameters match the schema above

## Development

To test the MCP server manually:

```bash
# Start the server
agtrace serve

# Send a JSON-RPC request (in another terminal)
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | agtrace serve
```

## See Also

- [Model Context Protocol Specification](https://modelcontextprotocol.io)
- [AgTrace CLI Documentation](https://github.com/lanegrid/agtrace)
- [Claude Desktop MCP Guide](https://claude.com/docs/mcp)
