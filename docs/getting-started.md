# Getting Started with agtrace

This guide walks you through installing and using agtrace to monitor AI agent sessions and enable agents to query their execution history.

## Installation

agtrace works in two modes:
1. **MCP Server** — Gives agents access to their session history (recommended)
2. **CLI Tools** — For manual session inspection and debugging

For both modes, install globally:

### via npm (Recommended)

```bash
npm install -g @lanegrid/agtrace
```

### via npx (no installation)

If you prefer not to install globally, run via `npx`.

*Note: In the examples below, replace `agtrace` with `npx @lanegrid/agtrace`.*

```bash
npx @lanegrid/agtrace@latest init
```

### via Cargo (Rust)

```bash
cargo install agtrace
```

## Quick Start

### Option A: MCP Setup (Recommended)

Let your AI agent query its execution history by connecting agtrace via MCP:

**1. Initialize agtrace:**
```bash
agtrace init
```

**2. Connect to your AI assistant:**

For **Claude Code**:
```bash
claude mcp add agtrace -- agtrace mcp serve
```

For **Codex (OpenAI)**:
```bash
codex mcp add agtrace -- agtrace mcp serve
```

For **Claude Desktop**, add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "agtrace": { "command": "agtrace", "args": ["mcp", "serve"] }
  }
}
```

**3. Start using it:**

Now your agent can query its own history:
- *"What did we decide about the database schema?"*
- *"Show me errors from previous attempts"*
- *"Continue where we left off yesterday"*

See [MCP Integration Guide](mcp-integration.md) for details.

### Option B: CLI Tools

For manual session inspection and debugging:

**1. Initialize agtrace:**
```bash
agtrace init
```

**2. Navigate to your project:**
```bash
cd /path/to/your/project
```

**3. Start live monitoring:**
```bash
agtrace watch
```

**4. Start your AI agent (same directory):**
```bash
claude  # or codex, gemini, etc.
```

**5. Explore session history:**
```bash
agtrace session list
agtrace session show <session_id>
agtrace lab grep "error" --json
```

## Next Steps

- Learn about [watch command](commands/watch.md) for live monitoring
- Explore [session command](commands/session.md) for history inspection
- Understand [CWD-scoped monitoring](faq.md#cwd-scoped-monitoring)
