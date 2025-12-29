# Getting Started with agtrace

This guide walks you through installing and using agtrace for the first time.

## Installation

For best performance and easy access to `watch`, install globally.

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

### 0) Initialize Once (Global)

Run `agtrace init` **once** on your machine.

This creates local configuration and caches under `~/.agtrace`.
It does **not** modify any project directory, and you do **not** need to run it per project.

```bash
agtrace init
```

### 1) Open Your Project Directory (CWD matters)

`agtrace` scopes monitoring by the **current working directory (cwd)**.

To ensure `agtrace` can locate and follow the right session logs, run it from the **same working directory** where your AI coding agent is started.

```bash
cd /path/to/your/project
```

### 2) Start `watch` (either order works)

In one terminal pane (from the project directory), run:

```bash
agtrace watch
```

`watch` can be started before or after your AI coding agent.

* If no active session exists yet, it stays in **waiting mode** (or opens the latest session if available).
* When a new session starts, agtrace detects the new logs and **automatically switches** to it.
* You do **not** need to restart `agtrace watch` per session.

### 3) Start Your AI Coding Agent (Same CWD)

In another terminal (same project directory), launch your agent:

```bash
# Example: Claude Code
claude

# Or Codex, Gemini, etc.
```

That's it. No integration required — `watch` follows sessions by monitoring logs.

### 4) Analyze Past Sessions

```bash
agtrace session list
agtrace session show <session_id>
```

### 5) Advanced: History Search ("Lab")

```bash
agtrace lab grep "write_file" --json
agtrace lab grep "mcp" --raw --limit 1
```

## Next Steps

- Learn about [watch command](commands/watch.md) for live monitoring
- Explore [session command](commands/session.md) for history inspection
- Understand [CWD-scoped monitoring](faq.md#cwd-scoped-monitoring)
