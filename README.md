<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>Observability and Debugging for AI Coding Agent Sessions.</strong></p>
  <p>
    Real-time telemetry and session debugging for Claude Code, Codex, and Gemini.
    A local-only “top + trace viewer” for agent runs: context, tools, costs, and drift —
    <strong>with zero overhead</strong>.
  </p>

  [![npm version](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace.svg)](https://crates.io/crates/agtrace)
</div>

---

## 📉 The Problem: Agent Sessions Are Stateful — But Unobservable

AI coding agents behave like stateful, long-running programs. They accumulate context, call tools, read/write files, and make decisions turn by turn.

But when something goes wrong — instruction loss, constraint violations, sudden behavior drift, cost spikes — we usually have no way to answer basic operational questions:

- **What changed?** (model, context pressure, tool usage, files touched)
- **When did it start?** (the exact boundary where behavior shifted)
- **Why this run?** (what was different compared to previous sessions)

Context window compaction is a common example: it’s expected behavior, but compaction boundaries are often invisible. Without telemetry, you only notice after the agent starts behaving differently.

In practice, we are operating a lossy, stateful system without logs, metrics, or traces.

## ⚡ The Solution: agtrace

**agtrace** adds an observability layer to AI coding agents by turning messy provider logs into a consistent, queryable timeline.

It gives you a quick, practical on-ramp — a live “fuel gauge” for your session — while building toward a deeper goal: normalized telemetry you can use to profile, compare, and improve agent workflows.

agtrace’s core loop:

1. **Capture** local provider logs (no cloud)
2. **Normalize** them into a single event model across providers
3. **Index** metadata without duplicating large logs (pointer-based)
4. **Analyze** sessions into turns/steps/metrics (schema-on-read)
5. **Visualize** live and historical runs in human-friendly views

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

*Live demo of `agtrace watch` — real-time session telemetry*

![agtrace watch TUI dashboard](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/watch-screenshot-claude.png)

*The dashboard showing context usage, current turn, and token costs*

---

## ✨ Key Features

### 1) Live Telemetry (`watch`)
A TUI dashboard for “session vitals”:
- context window usage and pressure (useful around compaction boundaries)
- current turn, recent activity, and tool usage signals
- token/cost telemetry (where available)

### 2) Session Summaries (`session list` / `session show`)
Inspect recent sessions and drill into a specific run:
- context usage, turns, models
- high-level structure you can compare across runs

### 3) Debugging (“Lab”)
Search and inspect history at scale:
- `agtrace lab grep` across thousands of past sessions
- analyze tool usage patterns
- inspect raw provider events when debugging schema changes (`--raw`)

### 4) Provider Normalization
Whether you use **Claude Code**, **Codex**, or **Gemini**, agtrace converts events into a consistent internal format so you can reason about sessions the same way across providers.

### 5) Local-Only by Default
Agent logs contain sensitive code and secrets. **agtrace runs 100% locally** and reads directly from your local log files (e.g., `~/.claude`). No data is sent to the cloud.

### 6) High-Performance, Minimal Footprint
Built in **Rust**, agtrace is designed to run continuously without slowing down your machine while you work with heavyweight AI agents.

---

## 📦 Installation

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

---

## 🚀 Quick Start

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

That’s it. No integration required — `watch` follows sessions by monitoring logs scoped to your current working directory.

### 4) Analyze Past Sessions

```bash
# List recent sessions
agtrace session list

# Show analysis of a specific session (context usage, turns, models)
agtrace session show <session_id>
```

### 5) Advanced: The Lab

Debug agent interactions or search for specific patterns:

```bash
# Find all file write operations across history
agtrace lab grep "write_file" --json

# Inspect a raw provider event (useful for debugging schema changes)
agtrace lab grep "mcp" --raw --limit 1
```

---

## 🧭 CWD-Scoped Monitoring

agtrace uses your current working directory (cwd) as the scope boundary for log discovery and session tracking.
To monitor a different project, run `agtrace watch` from that project's directory.

---

## 🏗️ Architecture

agtrace is designed around **pointer-based indexing** and **schema-on-read**:

1. **No Data Duplication**
   agtrace does not copy your massive log files. It indexes metadata and points to the original logs.

2. **Resilient to Schema Drift**
   Provider log schemas change frequently. agtrace parses logs at read time, so schema updates are less likely to corrupt or invalidate historical indexes.

3. **Project Isolation**
   Sessions are scoped by cwd/project boundaries and grouped by a project root hash to keep workspaces clean and prevent cross-project mixing.

---

## 🤝 Supported Providers

* **Claude Code** (Anthropic)
* **Codex** (OpenAI)
* **Gemini** (Google)

---

## 📜 License

Dual-licensed under the MIT and Apache 2.0 licenses.
