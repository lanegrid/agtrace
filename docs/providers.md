# Supported Providers

agtrace normalizes the logs of several AI coding agents into one `AgentEvent` model.

## Provider List

| Provider | Supported versions | Default log path |
|----------|--------------------|------------------|
| **Claude Code** (Anthropic) | **≥ 2.1.24x** (logs written from 2026-09 on) | `~/.claude/projects` |
| **Codex** (OpenAI) | **≥ 0.153** (paginated rollouts, `multi_agent` v2) | `~/.codex/sessions` |

Older log formats (Claude Code before 2.1.24x, Codex before 0.153) are **not supported**.
agtrace does not keep parsers for them.

## How It Works

- **Configuration.** `agtrace init` (or `agtrace provider detect`) finds the installed
  providers and writes their log roots to `config.toml`. Use `agtrace provider set` to change
  them.
- **Discovery.** Each provider lists its agent files and reads a cheap **header** (the Codex
  `session_meta` line; the first records of a Claude transcript, plus sidecar files). The
  header gives the agent's identity, kind, parent, and project directory without parsing the
  whole file.
- **Decoding.** A per-file decoder turns each line into zero or more `AgentEvent`s. The same
  decoder is used for batch reads (`session show`, `lab`, MCP) and for live tailing (`watch`).

### Lenient decoding

Decoding is lenient **per line**. A line that is not valid JSON, has an unknown record kind,
or does not match the expected shape is **counted** as a diagnostic and skipped. A file never
fails to decode because of its content; only I/O errors fail. `agtrace doctor run` lists the
files that have such lines, and the `watch` status bar shows the diagnostic count.

## Common Features Across Providers

- Tool call normalization (file reads/edits, shell commands, search, MCP, agent tools)
- Turn and step reconstruction
- Token usage (uncached input, cache read, cache write, output), deduplicated per API response
- Context window tracking with a single resolver (see below)
- Multi-agent structure: teammates, subagents, forks, Codex child threads, and inter-agent
  messages. See [Multi-Agent Sessions](multi-agent.md).

## Context window

The context window of each agent is resolved in one place. The first source that applies
wins:

1. `[context_window]` in `config.toml` (your override)
2. An explicit number in the log (Codex `model_context_window`)
3. An extended-context marker (Claude `[1m]` model suffix or "(1M context)")
4. `~/.codex/models_cache.json` (Codex)
5. A built-in model table
6. The observed usage, rounded up to a known window size

If the observed usage is larger than the window from sources 2–5, the window is raised to
fit. Usage % = the latest request's input tokens (uncached + cache read + cache write) ÷ the
window. The `watch` focus pane shows which source was used (`cfg`, `log`, `1m`, `cache`,
`table`, `obs`).

To override, add a `[context_window]` section to `config.toml`, which lives in the agtrace
data directory:

```toml
[context_window]
default_claude_code = 1000000   # provider default: default_<provider>
"claude-opus-5" = 1000000       # longest-prefix match on the model id ([..] suffix ignored)
"gpt-5.6" = 258400
```

A model-prefix entry wins over the provider default. An old `[providers.gemini]` section is
ignored with a warning.

## Environment overrides

| Variable | Effect |
|---|---|
| `AGTRACE_PATH` | agtrace data directory (database and `config.toml`) |
| `AGTRACE_CLAUDE_HOME` | Replaces `~/.claude` for `watch` (projects in `<home>/projects`, plus `teams/`, `sessions/`) |
| `AGTRACE_CODEX_HOME` | Replaces `~/.codex` for `watch` and the models cache (rollouts in `<home>/sessions`) |

The provider-home overrides exist mainly for tests and `agtrace demo`, which point agtrace
at a synthetic workspace.

## Provider-Specific Notes

### Claude Code

- Teammates (Agent Teams) are separate top-level transcripts. Async subagents and forks live in
  `<session>/subagents/agent-<id>.jsonl` with a `.meta.json` sidecar.
- `~/.claude/teams/<team>/config.json` links teammates to their lead.
- `~/.claude/sessions/<pid>.json` tells `watch` which sessions still have a running process.
  agtrace never opens `*.key` files.
- Thinking blocks are redacted in current logs and are shown as `[thinking redacted]`.

### Codex

- Every thread (root, child, fork) is its own rollout. Children identify their parent in
  `session_meta.source.subagent.thread_spawn`.
- Inter-agent message bodies are encrypted and shown as `[encrypted]`; only `FINAL_ANSWER` is
  plaintext.
- `exec` tool calls are split into sub-actions (commands, file changes) where the log provides
  them.
