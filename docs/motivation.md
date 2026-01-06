# Why agtrace?

## The Frustration

I use AI coding agents daily. Claude Code, Codex, Gemini — they're powerful, but they're also **black boxes**.

When I'm in a session, I can't see:
- How much of the context window I've used
- Whether the agent is stuck in a loop
- What's happening between my prompts

I found myself guessing. "Is it about to hit context limits?" "Should I start a new session?" "What did it actually do to those files?"

When something goes wrong, I scroll through chat history trying to reconstruct what happened. Previous sessions are buried in provider-specific log files I never look at.

## What I Built

**agtrace** sits alongside my coding agent and shows me what's happening:

```bash
agtrace watch
```

Now I can see:
- **Context window filling up** — colored bar, real-time
- **Token consumption per task** — finally understanding how much different operations cost
- **Live activity** — tool calls, file reads, reasoning traces

For the first time, I can make informed decisions instead of guessing.

## Unexpected Benefit: Agent Memory

I didn't plan this, but agents can also query their own history via MCP:

```bash
claude mcp add agtrace -- agtrace mcp serve
```

Now my agent can search past sessions, find previous errors, see what files it modified last time.

This isn't "memory" in the AI sense — it's **searchable execution history**. The agent retrieves what happened; interpreting it is still its job.

## How It Works

1. **Auto-discovers** logs from Claude Code, Codex, Gemini
2. **Normalizes** different formats into unified events
3. **Indexes** via SQLite pointers (zero-copy, minimal overhead)
4. **Exposes** through CLI, MCP, and SDK

## Principles

- **Local-first** — All data stays on your machine
- **Zero instrumentation** — No code changes needed
- **Schema-on-read** — Raw logs are source of truth
- **Provider-agnostic** — Same interface for all agents
