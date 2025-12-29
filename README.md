<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>top + tail -f for AI Coding Agent Sessions.</strong></p>
  <p>
    A local-only TUI for Claude Code, Codex, and Gemini.
    See context pressure, tool activity, and costs in real time.
  </p>

  [![npm version](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace.svg)](https://crates.io/crates/agtrace)
</div>

---

## 📉 The Problem: Session Telemetry Exists — But It’s Fragmented and Hard to Use

AI coding agents behave like stateful, long-running programs. They accumulate context, call tools, read/write files, and make decisions turn by turn.

Most agents *do* provide signals like “X% remaining before compaction” — but in practice those signals are:
- **inconsistent** (different wording, thresholds, and timing across vendors)
- **easy to miss** (buried in chat output, not always surfaced at the right moment)
- **hard to compare** (no shared units, no shared event model, no shared history)

So when something goes wrong — instruction loss, constraint violations, sudden behavior drift, cost spikes — it’s painful to answer basic questions:

- **What changed?** (model, context pressure, tool usage, files touched)
- **When did it start?** (the boundary where behavior shifted)
- **Why this run?** (what was different compared to previous sessions)

In short: we're operating stateful systems with no `top`, no `tail -f`, and no cross-provider trace.

## ⚡ The Solution: agtrace

**agtrace** is `top` + `tail -f` for AI coding agent sessions — powered by a normalized event timeline.

It reads local provider logs, normalizes them into a consistent model, and turns them into views you can actually operate with:
- a live “top” view (`watch`) for context pressure and activity
- a history you can query (`session` / `lab`) to understand what happened and when

agtrace’s core loop:

1. **Capture** local provider logs (no cloud)
2. **Normalize** them into a single event model across providers
3. **Index** metadata without duplicating large logs (pointer-based)
4. **Analyze** sessions into turns/steps/metrics (schema-on-read)
5. **Visualize** live and historical runs in human-friendly views

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

*Live demo of `agtrace watch` — a `top`-like view for your session*

![agtrace watch TUI dashboard](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/watch-screenshot-claude.png)

*The dashboard showing context usage, current turn, and token costs*

---

## ✨ Key Features

### 1) Live "top" View (`watch`)
A TUI dashboard for "session vitals":
- context window usage and pressure (useful around compaction boundaries)
- current turn, recent activity, and tool usage signals
- token/cost telemetry

### 2) Always-On Session Tracking
Keep `watch` running in a terminal pane. It detects new sessions automatically — no restart needed.

### 3) Session Summaries (`session list` / `session show`)
Inspect recent sessions and drill into a specific run:
- context usage, turns, models
- high-level structure you can compare across runs

### 4) History Search (`lab grep`)
Search and inspect past sessions at scale:
- `agtrace lab grep` across thousands of sessions
- analyze tool usage patterns
- inspect raw provider events when debugging schema changes (`--raw`)

### 5) Provider Normalization
Whether you use **Claude Code**, **Codex**, or **Gemini**, agtrace converts events into a consistent internal format so you can reason about sessions the same way across providers.

### 6) Local-Only by Default
Agent logs contain sensitive code and secrets. **agtrace runs 100% locally** and reads directly from your local log files (e.g., `~/.claude`). No data is sent to the cloud.

### 7) High-Performance, Minimal Footprint
Built in **Rust**. Parses gigabytes of JSONL logs in seconds. Runs continuously with minimal CPU/memory footprint.

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

That's it. No integration required — `watch` follows sessions by monitoring logs.

### 4) Analyze Past Sessions

```bash
agtrace session list
agtrace session show <session_id>
```

### 5) Advanced: History Search (“Lab”)

```bash
agtrace lab grep "write_file" --json
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
