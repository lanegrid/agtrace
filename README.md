<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>Observability for AI Agents</strong></p>
  <p>Local-first monitoring for Claude Code, Codex, and Gemini.</p>

  [![npm](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat&label=npm)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace-sdk.svg?label=SDK)](https://crates.io/crates/agtrace-sdk)
</div>

---

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

**agtrace** monitors AI agent sessions in real-time and lets agents query their own execution history via MCP.

- **Zero instrumentation** — Auto-discovers provider logs
- **100% local** — Privacy by design, no cloud dependencies
- **Universal timeline** — Unified view across all providers

## Quick Start

```bash
npm install -g @lanegrid/agtrace
cd my-project
agtrace init      # Initialize workspace (one-time setup)
agtrace watch     # Launch live dashboard
```

## MCP: Let Agents Query Their Own History

Connect your AI assistant to search past sessions via [Model Context Protocol](https://modelcontextprotocol.io):

**Claude Code:**
```bash
claude mcp add agtrace -- agtrace mcp serve
```

**Codex (OpenAI):**
```bash
codex mcp add agtrace -- agtrace mcp serve
```

**Claude Desktop:**
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

Your agent can now:
- Search past sessions for tool calls and errors
- Retrieve tool calls and results from previous work
- Analyze failure patterns

**Example queries:**
- *"Show me sessions with failures in the last hour"*
- *"Search for tool calls that modified the database schema"*
- *"Analyze the most recent session for performance issues"*

For detailed setup and examples, see the [MCP Integration Guide](docs/mcp-integration.md).

## CLI Commands

Debug and inspect agent behavior manually:

```bash
agtrace watch              # Live TUI dashboard
agtrace session list       # Browse session history
agtrace lab grep "error"   # Search across sessions
```

## Building with the SDK

Embed agent observability into your own tools (dashboards, IDE plugins, custom analytics).

```toml
[dependencies]
agtrace-sdk = "0.5"
```

```rust
use agtrace_sdk::{Client, types::SessionFilter};

let client = Client::connect_default().await?;
let sessions = client.sessions().list(SessionFilter::all())?;
if let Some(summary) = sessions.first() {
    let handle = client.sessions().get(&summary.id)?;
    let session = handle.assemble()?;
    println!("{} turns, {} tokens",
        session.turns.len(),
        session.stats.total_tokens);
}
```

See [SDK Documentation](https://docs.rs/agtrace-sdk), [Examples](crates/agtrace-sdk/examples/), and [SDK README](crates/agtrace-sdk/README.md).

## Supported Providers

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini** (Google)

## Documentation

- [Getting Started](docs/getting-started.md) - Detailed installation and usage guide
- [MCP Integration](docs/mcp-integration.md) - Connect agents to their execution history
- [Architecture](docs/architecture.md) - Platform design and principles
- [Why agtrace?](docs/motivation.md) - Understanding the problem and solution
- [Full Documentation](docs/README.md) - Commands, FAQs, and more

## License

Dual-licensed under the MIT and Apache 2.0 licenses.
