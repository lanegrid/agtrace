# Why agtrace?

## The Frustration

I use AI coding agents daily. Claude Code, Codex: they're powerful, but they're also **black boxes**. And these days one agent is rarely alone: it spawns teammates, background subagents, forks, and child threads.

When I'm in a session, I can't see:
- How much of the context window I've used
- Whether the agent is stuck in a loop
- What's happening between my prompts
- What the agents it spawned are doing, and what they tell each other

I found myself guessing. "Is it about to hit context limits?" "Should I start a new session?" "What did it actually do to those files?"

When something goes wrong, I scroll through chat history trying to reconstruct what happened. Previous sessions are buried in provider-specific log files I never look at.

## What I Built

**agtrace** sits alongside my coding agent and shows me what's happening:

```bash
agtrace watch
```

Now I can see:
- **Every agent at once**: the lead and everything it spawned, as one live tree with status and context window usage
- **Live activity**: tool calls, spawns, compactions, and model changes of the selected agent
- **Inter-agent messages**: who asked whom to do what, and what came back

For the first time, I can make informed decisions instead of guessing.

## Unexpected Benefit: Agent Memory

I didn't plan this, but agents can also query their own history via MCP:

```bash
claude mcp add agtrace -- agtrace mcp serve
```

Now my agent can search past sessions, find previous errors, see what files it modified last time.

This isn't "memory" in the AI sense — it's **searchable execution history**. The agent retrieves what happened; interpreting it is still its job.

## How It Works

1. **Auto-discovers** logs from Claude Code and Codex
2. **Normalizes** different formats into unified events
3. **Indexes** via SQLite pointers (zero-copy, minimal overhead)
4. **Exposes** through CLI, MCP, and SDK

## Principles

- **Local-first** — All data stays on your machine
- **Zero instrumentation** — No code changes needed
- **Schema-on-read** — Raw logs are source of truth
- **Provider-agnostic** — Same interface for all agents
