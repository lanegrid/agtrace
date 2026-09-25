# Multi-Agent Sessions

Claude Code and Codex can both run several agents for one piece of work: a lead with
teammates, background subagents, forks, and Codex child threads. This page explains how
agtrace models them, where each provider stores them on disk, how agtrace links them into
one tree, and where the tree shows up.

Supported formats: **Claude Code ≥ 2.1.24x** (logs from 2026-09 on) and **Codex ≥ 0.153**
(paginated rollouts, `multi_agent` v2). Older formats are not supported. See
[Supported Providers](providers.md).

## The model

### One agent = one file = one timeline

Each agent log file is exactly one **agent**:

- every Claude main or teammate transcript,
- every Claude `subagents/agent-*.jsonl`,
- every Codex rollout.

Events carry the `AgentId` of the file they came from. The multi-agent structure (who spawned
whom, who talks to whom) lives in an **agent graph** built next to the events, not inside
them.

### `AgentId`

`AgentId` is a stable, globally unique string key:

| Form | Meaning | File |
|---|---|---|
| `claude:<sessionId>` | Claude main session or teammate | `<project>/<sessionId>.jsonl` |
| `claude:<sessionId>/<agentId>` | Claude async subagent or fork | `<project>/<sessionId>/subagents/agent-<agentId>.jsonl` |
| `codex:<threadId>` | any Codex thread (root or child) | `rollout-*-<threadId>.jsonl` |

`agtrace watch --session` accepts these forms directly, for example
`agtrace watch --session codex:01900000-0000-7000-8000-000000000001`.

### `AgentKind`

| Kind | Provider | What it is |
|---|---|---|
| `main` | both | A root session started by a person or a background job: a Claude main transcript or a Codex root thread. |
| `teammate` | Claude | An Agent Teams member. It has its own top-level transcript that carries `teamName` / `agentName`. |
| `subagent` | Claude | An async or background subagent (`subagents/agent-*.jsonl`) that is not a fork. |
| `fork` | both | A child that starts with a copy of the parent's context. In Claude this is `fork-context-ref` or `meta.isFork`; in Codex it is a thread with `forked_from_id`. |
| `codex_thread` | Codex | A child thread spawned with `thread_spawn` that is not a fork. |

Each agent also gets a **display name**: the teammate's `agentName`, the Codex `agent_path`
leaf or nickname, the subagent's description, or the session's title / `agent-name`. Codex
agents also get a hierarchical **path**, their `agent_path` (`/root`, `/root/judge`), which
Codex uses to address messages.

## How each provider stores children on disk

### Claude Code

```
~/.claude/
├── projects/<encoded cwd>/
│   ├── <lead-session>.jsonl                       main (lead)
│   ├── <teammate-session>.jsonl                   teammate: separate top-level transcript
│   └── <lead-session>/subagents/
│       ├── agent-<aid>.jsonl                      async subagent or fork
│       └── agent-<aid>.meta.json                  sidecar: agentType, description, toolUseId, spawnDepth, isFork, stoppedByUser, model
├── teams/<team>/config.json                       leadSessionId + members (name, agentType, model, isActive)
└── sessions/<pid>.json                            live process registry (sessionId, status busy|idle, updatedAt)
```

- **Teammates** are ordinary top-level transcripts in the project directory. Their records
  carry `teamName` and `agentName`, and the first record is usually an `agent-setting` line
  (the agent type). The team's `config.json` names the lead session (`leadSessionId`).
- **Async subagents and forks** are written under the parent session's directory as
  `subagents/agent-<aid>.jsonl`. The `<aid>` is `a` followed by 16 hex digits. The
  `.meta.json` sidecar holds the spawning tool call id (`toolUseId`), the agent type and
  description, the spawn depth, the model, `isFork`, and `stoppedByUser`. A file whose first
  line is `fork-context-ref`, or whose meta has `isFork`, is a **fork**. A subagent without a
  meta file is still tracked; the meta file is read again when it appears.
- **The session registry** `~/.claude/sessions/<pid>.json` has one entry per running Claude
  process. agtrace lists only `<digits>.json` entries and never opens `*.key` files. An entry
  counts as live when it exists **and** the pid is alive (`kill(pid, 0)` on Unix; on other
  platforms the entry existing is enough).
- `tool-results/` directories are not agent files.

### Codex

```
~/.codex/
├── sessions/YYYY/MM/DD/rollout-<ts>-<threadId>.jsonl   one file per thread (root, child, fork)
├── session_index.jsonl                                  thread titles (latest line per id wins)
└── models_cache.json                                    model slugs → context windows
```

- Every thread, root or child, is its own rollout file. Rollouts are **paginated**
  (`history_mode: "paginated"`): every record carries an `ordinal`.
- The first line, `session_meta`, identifies the thread:
  - `id`: the thread id, giving `codex:<id>`.
  - `session_id`: the root thread of the tree.
  - `source`: a string such as `"cli"` or `"vscode"` for a root. For a child it is an object,
    `{"subagent": {"thread_spawn": {"parent_thread_id", "depth", "agent_path", "agent_nickname", "agent_role"}}}`.
    Top-level `parent_thread_id` / `agent_path` / `agent_nickname` are used when present.
  - `forked_from_id`: set for a **fork**. A fork's rollout starts with a copy of the parent's
    history, including a second `session_meta`. Records whose `ordinal` is below
    `subagent_history_start_ordinal` are that copied prefix. They are skipped and counted as
    `fork_prefix` in the diagnostics.

## How links are resolved

Parent links come from the file header whenever possible. Everything else is resolved by
the agent graph (`agtrace-engine::workspace`) as agents are discovered. It retries whenever
a new agent appears, so it does not matter which file is read first.

| Child | Parent comes from |
|---|---|
| Claude subagent / fork | Its location: `<sid>/subagents/` gives parent `claude:<sid>`. The meta file's `toolUseId` gives the `spawn_call_id`. |
| Claude teammate | (1) `~/.claude/teams/<team>/config.json` `leadSessionId`. (2) Otherwise, the agent whose log contains the `AgentSpawn` of that team member (the `Agent` tool result with `status: teammate_spawned`). |
| Codex child thread / fork | `session_meta` `parent_thread_id` (and `session_id` for the root). |

Inside a parent's log, a spawn is decoded as an **`AgentSpawn`** event. The event refers to
the child through a *handle*, which the graph then resolves to an `AgentId`:

- Claude: the `Agent` tool result. `teammate_spawned` refers to the child by team and member
  name; `async_launched` refers to it by `agentId`.
- Codex: `SubAgentActivity` `started` in the parent, which carries the child's thread id.

Handles are resolved as follows:

- **Claude team member names** (`audit-A`, `team-lead`): resolved within the team.
  `team-lead` means the lead.
- **Codex agent paths** (`/root/judge`): resolved within the same root.
- **Claude subagent ids**: resolved within the same session.

`spawn_call_id` is the provider's call id of the spawning tool call in the parent: Claude
`tool_use.id` (the same value as `meta.toolUseId`), or the Codex `spawn_agent` call id. It
matches a turn's tool call to the child it created, and `session show` uses it to list the
children spawned in each turn.

A child whose parent cannot be resolved yet is still shown, under its root: for Claude, the
same session directory or team lead; for Codex, the `session_id`. Children are never hidden.

## Inter-agent messages

Messages between agents are decoded as `AgentMessage` events with a direction (incoming or
outgoing relative to the log they are in), sender, recipients, kind, and an optional body.
Bodies are truncated to 4 KiB.

| Provider | Source in the log | Kind |
|---|---|---|
| Claude | `SendMessage` tool call and its result | `MESSAGE` (outgoing) |
| Claude | `<teammate-message teammate_id=… summary=…>` in a user record (a record can hold several) | `MESSAGE`, or `NEW_TASK` for a teammate's first prompt |
| Claude | `<teammate-message>` whose body is an `idle_notification` JSON | lifecycle: idle, or failed when `idleReason` is `failed` |
| Claude | `<task-notification>` for an agent task (task id `a…`), or a `queued_command` attachment with `commandMode: task-notification` | `TASK_NOTIFY` plus lifecycle `completed` / `failed` / `killed` |
| Claude | `SubagentHandback`, or a `queued_command` handback from a peer | `HANDBACK` (and the subagent is Done) |
| Claude | `<agent-message from=…>` between unrelated sessions | `PEER` (shown in the feed with the sender label only; no tree edge) |
| Codex | `collaboration` tools: `spawn_agent` / `followup_task`, `send_message`, `interrupt_agent` | `NEW_TASK`, `MESSAGE`, `INTERRUPT` (outgoing) |
| Codex | `agent_message` records (`Message Type: NEW_TASK \| MESSAGE \| FINAL_ANSWER`) | incoming, same kind |

A notification that reaches the log through both a `<task-notification>` tag and a
`queued_command` attachment is shown only once.

Codex encrypts inter-agent message bodies (`gAAAA…` tokens). agtrace shows them as
`[encrypted]` and never tries to decode them. The only plaintext body is **`FINAL_ANSWER`**,
the child's result, which is shown in full. Claude message bodies are plaintext.

The same message often appears twice: outgoing in the sender's log and incoming in the
recipient's log. The live feed deduplicates on `(from, to, kind)` with a ±2 s window and keeps
the copy that has a plaintext body.

## Status

Every agent has a status: **running**, **idle**, **done**, **failed**, **killed**, or
**unknown**. Several signals feed it, checked in this order:

1. **Parent-side terminal events** set a terminal status:
   - Claude task-notification `completed` / `failed` / `killed`
   - handback ⇒ done
   - `TaskStop` on a teammate ⇒ killed
   - `system/agents_killed` in the parent ⇒ killed for its running children
   - meta `stoppedByUser`
   - team config `isActive: false`
   - Codex `SubAgentActivity` `completed` ⇒ done

   A terminal status is sticky unless the agent's own log shows new activity afterwards (a
   Codex child can be re-tasked with `followup_task`).
2. **The Claude session registry** decides for Claude mains and teammates **while the pid is
   alive**: `busy` ⇒ running, `idle` ⇒ idle. Once the entry is gone or the pid is dead, the
   agent is done (unless step 1 already marked it killed or failed).
3. **The agent's own log**:
   - Claude: activity ⇒ running; turn end ⇒ idle.
   - Codex: `task_started` ⇒ running; `task_complete` ⇒ idle (failed on error);
     `turn_aborted` ⇒ idle.
   - Codex parent-side `SubAgentActivity`: `interacted` ⇒ running; `interrupted` ⇒ idle.
4. **Staleness**, when nothing else applies:
   - Claude: no registry entry and no write for 10 min ⇒ idle; idle for 2 h ⇒ done.
   - Claude subagent / fork: running with no write for 10 min while the parent is done or
     gone ⇒ done.
   - Codex: running with no write for 30 min ⇒ idle; a root idle for 2 h ⇒ done.

## Where it shows up

- **`agtrace watch`**: the live multi-agent TUI. The left pane is the agent tree with
  status and context %, the right pane is the selected agent's timeline, and the bottom pane
  is the workspace-wide message feed. See [watch](commands/watch.md).
- **`agtrace session show`**: prints an `Agents:` tree for the session. The JSON output
  has `content.agents` and one stream per agent, and each turn lists the children it spawned.
  See [session](commands/session.md).
- **MCP**: `get_agent_tree` returns the same tree for a session.
  `list_sessions` with `include_children` returns child sessions with `agent_kind`,
  `agent_name`, `agent_path`, `team_name`, `root_session_id`, `parent_session_id` and
  `spawn_call_id`. See [MCP Integration](mcp-integration.md).
- **Index**: sessions (mains, teammates, Codex threads and forks) carry the agent columns
  above. Claude subagents and forks are `log_files` rows (role `subagent` / `fork`) under their
  parent session. See [Architecture](architecture.md#index-schema-v7).

## Try it without your own logs

```bash
agtrace demo            # --speed slow|normal|fast
```

`demo` replays a synthetic multi-agent workspace into a temporary directory at real-time
pace and runs the real watcher and TUI against it. The workspace has a Claude lead with a
teammate `audit-A` and a background subagent, plus a Codex root with a `/root/judge` child
thread.
