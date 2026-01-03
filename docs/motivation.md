# Why agtrace?

## The Problem: AI Agents Lack Session Memory

AI coding agents work well within a single session, but they start fresh every time.

This creates practical problems:
- **Context loss** — You must re-explain project constraints and design decisions every session
- **Repeated mistakes** — Agents retry approaches that already failed yesterday
- **Knowledge fragmentation** — Important discussions are buried in chat logs across multiple sessions
- **No learning** — Agents can't build on past successes or avoid past failures

When you switch projects or resume work the next day, the agent has no memory of:
- Why you made specific technical decisions
- What alternatives were already considered and rejected
- What implementation approaches worked or failed
- The evolution of your codebase over time

In short: AI agents treat every session as their first session, forcing you to be the only source of historical knowledge.

## The Solution: Agent Memory via MCP

**agtrace** gives AI agents access to their own execution history through the [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

### How It Works

1. **Auto-discovery** — agtrace finds and indexes logs from Claude Code, Codex, and Gemini
2. **Normalization** — Converts diverse log formats into a unified event timeline
3. **MCP Server** — Exposes session history through standardized tools
4. **Agent queries** — Your AI assistant can now search, analyze, and learn from past sessions

### What Agents Can Do

Once connected via MCP, agents gain powerful memory capabilities:

**Context retrieval:**
- *"What did we decide about the authentication system?"*
- *"Show me the discussion about database migration"*
- *"What were the constraints on the API design?"*

**Learning from failures:**
- *"We tried to implement this before—what went wrong?"*
- *"Show me previous attempts at optimizing this query"*
- *"What errors occurred in yesterday's refactoring?"*

**Session continuity:**
- *"Continue the work we started in the last session"*
- *"Review what we accomplished this week"*
- *"What files did we modify during the migration?"*

### Multi-Layer Architecture

agtrace is built as a **platform** with multiple access layers:

**For AI Agents (MCP)**:
- Query historical sessions and analyze past decisions
- Search across all sessions for specific patterns or events
- Build context-aware responses based on project history

**For Developers (CLI)**:
- Live monitoring with `watch` (like `top` for agents)
- Manual session inspection with `session` and `lab` commands
- Debug agent behavior and performance issues

**For Builders (SDK)**:
- Embed agtrace into custom tools and dashboards
- Build IDE plugins with inline session replay
- Create team analytics and vital-checkers

### Core Principles

1. **Local-first** — All data stays on your machine, no cloud dependencies
2. **Zero-copy indexing** — SQLite pointers reference original logs, minimal storage overhead
3. **Schema-on-read** — Raw logs are source of truth, parsing happens at query time
4. **Universal normalization** — Single event model works across all providers
