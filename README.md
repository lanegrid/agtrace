# agtrace

**The Vital Monitor for AI Coding Agents.**

Real-time telemetry, context window tracking, and session forensics for Claude Code, Codex, and Gemini. **Built in Rust for zero-overhead monitoring.**

[![npm version](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat)](https://www.npmjs.com/package/@lanegrid/agtrace)
[![crates.io](https://img.shields.io/crates/v/agtrace.svg)](https://crates.io/crates/agtrace)

---

## 📉 The Problem: "Context Window Anxiety"

AI Coding Agents (Claude Code, Codex, etc.) are evolving rapidly, but managing their **"Context Window"** has become a hidden, cognitively heavy burden for humans.

When a conversation exceeds the token limit, agents trigger **"Auto Compaction"** (silent compression). They start to "forget" previous instructions, file contents, or architectural decisions. Currently, this happens invisibly. You only realize it when the agent starts hallucinating or making regression errors.

You are effectively flying a plane without a fuel gauge.

## ⚡ The Solution: agtrace

**agtrace** is a local-only telemetry tool that acts as a "Vital Check" for your AI agents. by normalizing logs from various providers, it visualizes the internal state of your agent in real-time.

![agtrace watch TUI dashboard](docs/images/watch-screenshot-claude.png)

*The dashboard showing Context Window usage, current turn, and token costs*

### Key Features

* **👁️ Live Vital Monitoring (`watch`)**
  A TUI (Terminal User Interface) dashboard that visualizes the "health" of your session. See exactly how much Context Window is remaining before auto-compaction hits.

* **🔌 Provider Normalization**
  Whether you use `Claude Code`, `Codex`, or `Gemini`, agtrace normalizes the events into a standard format.

* **🔒 Local & Private**
  Agent logs contain sensitive code and secrets. **agtrace runs 100% locally.** No data is sent to the cloud. It reads directly from your local log files (`~/.claude`, etc.).

* **🚀 Auto-Tracking**
  The `watch` command automatically detects new sessions as they are created. Just keep it running in a separate terminal pane.

* **🥼 Forensics Lab**
  Use `agtrace lab grep` to search across thousands of past sessions, analyze tool usage patterns, or debug agent behavior with `--raw` inspection.

* **⚡ Zero-Overhead Monitoring**
  Built in **Rust**, agtrace is designed to run in the background with a minimal footprint. It won't slow down your machine while you work with heavy AI agents.

* **🔍 Instant Forensics**
  Parse and grep through gigabytes of JSONL logs in milliseconds. The schema-on-read architecture combined with Rust's performance makes analyzing history instantaneous.

---

## 📦 Installation

We recommend installing `agtrace` globally for the best performance and quick access to the `watch` command.

### via npm (Recommended)

```bash
npm install -g @lanegrid/agtrace
```

### via npx (no installation)

If you prefer not to install it globally, you can run commands using `npx`.
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

### 1. Initialize in Your Project

Navigate to your project directory and run:

```bash
cd /path/to/your/project
agtrace init
```

### 2. Start Your AI Coding Agent

In one terminal, launch your usual AI coding agent:

```bash
# Example: Claude Code
claude

# Or Codex, Gemini, etc.
```

### 3. Watch in Another Terminal

Open a separate terminal pane in the same project directory and run:

```bash
agtrace watch
```

That's it. No integration required—agtrace automatically detects and monitors your agent session.

* **Visualizes:** Context Window usage, Cost, Turns, and Last Activity.
* **Auto-Switch:** When you start a new session, agtrace automatically latches onto it.

### 4. Analyze Past Sessions

List recent sessions across all providers or inspect a specific one.

```bash
# List recent sessions
agtrace session list

# Show analysis of a specific session (Context usage, turns, models)
agtrace session show <session_id>

```

### 5. Advanced: The "Lab"

Debug agent interactions or search for specific patterns (e.g., "When did the agent try to write to `package.json`?").

```bash
# Find all file write operations across history
agtrace lab grep "write_file" --json

# Inspect raw provider event (useful for debugging schema changes)
agtrace lab grep "mcp" --raw --limit 1

```

---

## 🏗️ Architecture

agtrace is designed with **"Pointer-Based"** and **"Schema-on-Read"** philosophies:

1. **No Data Duplication:** We don't copy your massive log files. We index metadata and point to the original logs.
2. **Resilience:** Provider log schemas change frequently. agtrace parses logs at read-time, meaning a schema update won't corrupt your historical index.
3. **Project Isolation:** Sessions are grouped by project root hash, keeping your workspaces clean


## 🤝 Supported Providers

* **Claude Code** (Anthropic)
* **Codex** (OpenAI)
* **Gemini** (Google)

---

## 📜 License

This project is dual-licensed under the MIT and Apache 2.0 licenses.
