<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>Memory for AI Agents.</strong></p>
  <p>Let your agents learn from their past sessions.</p>

  [![npm](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat&label=npm)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace-sdk.svg?label=SDK)](https://crates.io/crates/agtrace-sdk)
</div>

---

## The Problem

AI coding agents start every session fresh. They can't remember:
- Why a decision was made yesterday
- What approaches already failed
- The context behind existing code

You end up re-explaining the same constraints, watching the same mistakes, and losing accumulated knowledge.

## The Solution

**agtrace** gives AI agents access to their own execution history via [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

Your agent can now:
- Query past sessions: *"What did we decide about the database schema?"*
- Learn from failures: *"Show me errors from previous attempts"*
- Maintain context: *"Continue where we left off yesterday"*

**Zero instrumentation.** agtrace auto-discovers logs from Claude Code, Codex, and Gemini. No code changes required

## Quick Start

```bash
npm install -g @lanegrid/agtrace
agtrace init
```

Then connect to your AI assistant:

**Claude Code:**
```bash
claude mcp add agtrace -- agtrace mcp serve
```

**Codex (OpenAI):**
```bash
codex mcp add agtrace -- agtrace mcp serve
```

**Claude Desktop:** Add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "agtrace": { "command": "agtrace", "args": ["mcp", "serve"] }
  }
}
```

That's it. Your agent now has memory.

## How Agents Use It

Once connected, your agent can query its own history:

| You ask | Agent does |
|---------|------------|
| *"Why did we choose PostgreSQL?"* | Searches past sessions for database discussions |
| *"Fix this bug, we tried before"* | Retrieves previous failed attempts and avoids them |
| *"Continue the refactoring"* | Loads context from yesterday's session |

**Real example:** An agent retrieved 34KB of historical context across 5 sessions, then generated a specification that respected all past design constraints—without you re-explaining anything.

## MCP Tools

agtrace exposes these tools to your agent:

| Tool | Purpose |
|------|---------|
| `list_sessions` | Browse session history with filters |
| `list_turns` | Get turn-by-turn overview of a session |
| `get_turns` | Retrieve detailed content of specific turns |
| `search_events` | Find specific tool calls or patterns |
| `analyze_session` | Detect failures, loops, and issues |
| `get_project_info` | List indexed projects |

See [MCP Integration Guide](docs/mcp-integration.md) for details.

## CLI Tools for Developers

Debug and inspect agent behavior manually:

```bash
agtrace watch              # Live TUI dashboard
agtrace session list       # Browse session history
agtrace lab grep "error"   # Search across sessions
```

![agtrace watch](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

## SDK for Builders

Build custom tools on top of agtrace:

```toml
[dependencies]
agtrace-sdk = "0.5"
```

```rust
use agtrace_sdk::{Client, Lens, types::SessionFilter};

let client = Client::connect_default().await?;
let sessions = client.sessions().list(SessionFilter::all())?;
let report = client.sessions().get(&sessions[0].id)?
    .analyze()?.through(Lens::Failures).report()?;
```

See [SDK Documentation](https://docs.rs/agtrace-sdk) and [examples](crates/agtrace-sdk/examples/).

## Supported Providers

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini** (Google)

All providers auto-discovered. Logs stay local.

## Documentation

- [MCP Integration Guide](docs/mcp-integration.md)
- [Getting Started](docs/getting-started.md)
- [Architecture](docs/architecture.md)
- [Full Documentation](docs/README.md)

## License

Dual-licensed under the MIT and Apache 2.0 licenses.
