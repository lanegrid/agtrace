# Why agtrace?

## The Problem: AI Agent Sessions Are Invisible

AI coding agents run complex multi-step workflows, but you can't see what's happening:

- **No visibility** — What tools did the agent call? What files did it read?
- **Hard to debug** — When something fails, you scroll through chat trying to reconstruct what happened
- **Lost context** — Previous sessions are buried in provider-specific log files
- **No cross-session search** — You can't easily find "that thing we did last week"

Each provider (Claude Code, Codex, Gemini) stores logs differently, in different locations, with different formats. There's no unified way to monitor, search, or analyze agent behavior.

## The Solution: Unified Observability

**agtrace** indexes logs from multiple providers and gives you:

### 1. Live Monitoring

See what your agent is doing in real-time:

```bash
agtrace watch
```

Like `top` for AI agents — token usage, tool calls, reasoning traces, all in one dashboard.

### 2. Session History Search

Find past sessions and search across them:

```bash
agtrace session list
agtrace lab grep "database"
```

Search for specific tool calls, errors, or patterns across all your sessions.

### 3. Agent Self-Query via MCP

Let agents search their own history:

```bash
claude mcp add agtrace -- agtrace mcp serve
```

Your agent can now:
- *"Show me errors from yesterday's session"*
- *"What files did we modify in the last refactoring?"*
- *"Find previous tool calls related to the database"*

This isn't "memory" in the sense of learning or reasoning about the past. It's **searchable execution history** — agents can retrieve what happened, but interpreting it is still their job.

## How It Works

1. **Auto-discovery** — Finds logs from Claude Code, Codex, and Gemini
2. **Normalization** — Converts provider-specific formats into unified events
3. **Indexing** — SQLite pointers to original logs (zero-copy, minimal overhead)
4. **Access layers** — CLI for developers, MCP for agents, SDK for builders

## What agtrace Is (and Isn't)

**agtrace is:**
- An observability tool for AI agent sessions
- A unified search interface across providers
- A way for agents to query their execution history

**agtrace is not:**
- A memory system that learns or reasons
- A knowledge base that extracts decisions
- A replacement for good prompting or documentation

## Core Principles

1. **Local-first** — All data stays on your machine
2. **Zero instrumentation** — No code changes, auto-discovers existing logs
3. **Schema-on-read** — Raw logs are source of truth
4. **Provider-agnostic** — Same interface for Claude, Codex, Gemini
