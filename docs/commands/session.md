# agtrace session

Inspect and query historical AI coding agent sessions.

## Overview

The `session` command provides tools to explore past sessions:
- List recent sessions for the current project
- Show detailed information about a specific session
- Analyze session structure, turns, and metrics

## Commands

### session list

List recent sessions for the current project.

```bash
agtrace session list [OPTIONS]
```

**Output includes:**
- Session ID
- Start time
- Duration
- Model used
- Turn count
- Context usage

**Options:**
- `--format json` - Output in JSON format for programmatic use
- `--limit N` - Show only the N most recent sessions
- `--provider <claude_code|codex>` - Filter by provider
- `--all` - Include child sessions (teammates, Codex child threads and forks)

**Example:**
```bash
agtrace session list --limit 10
agtrace session list --format json --limit 5
```

### session show

Show detailed information about a specific session.

```bash
agtrace session show <SESSION_ID> [OPTIONS]
```

**Output includes:**
- Session metadata (ID, provider, project, log files)
- The session's **agent tree** (see below)
- Context window usage over time
- Turn-by-turn breakdown, per agent
- Tool usage and token counts

**Options:**
- `--format json` - Output in JSON format
- `--compact` / `--verbose` / `--quiet` - Output density

**Example:**
```bash
agtrace session show 00000000
agtrace session show 00000000 --format json > session.json
```

#### Agent tree

When a session has children, `session show` prints an `Agents:` tree:

```
Agents:
               Demo project audit (main)  claude:00000000-0000-4000-8000-000000000001
               └ audit-A (teammate)  claude:00000000-0000-4000-8000-000000000002
               └ Count source files (subagent)  claude:00000000-0000-4000-8000-000000000001/a0000000000000001
               └ docs-fork (fork)  claude:00000000-0000-4000-8000-000000000001/a0000000000000002
```

The tree comes from the index:
- **Claude subagents and forks** are log files of the session (`log_files.role` `subagent` / `fork`).
- **Claude teammates** and **Codex child threads and forks** are child sessions (`sessions.parent_session_id`), included recursively.

Each agent's timeline is shown as its own stream after the main conversation. A turn whose tool
call spawned an agent lists it (`🔀 Spawned: …`). The match uses the index's `spawn_call_id`,
the provider call id of the spawning tool call. See [Multi-Agent Sessions](../multi-agent.md).

In JSON (`--format json`):
- `content.agents` is the tree, with nodes of `{agent_id, session_id, provider, kind, name, path, spawn_call_id, children}`.
- `content.streams[]` has one entry per agent timeline: `agent_id`, `name`, and a `stream_id` label (`main`, or `sidechain:<agentId>` for Claude subagents).
- `turns[].spawned_children` lists the agents spawned in that turn.

## Use Cases

### Compare Sessions

Compare two sessions to understand what changed:

```bash
agtrace session show session1 --format json > s1.json
agtrace session show session2 --format json > s2.json
diff s1.json s2.json
```

### Debug Context Pressure

Identify when context pressure became an issue:

```bash
agtrace session show <SESSION_ID>
# Look for high context usage percentages in the output
```

### Analyze Tool Usage

See which tools were called and when:

```bash
agtrace session show <SESSION_ID> --format json | jq '.content.streams[].turns[].steps[] | select(.kind == "ToolCall")'
```

## See Also

- [watch](watch.md) - Live session monitoring
- [lab](lab.md) - Advanced history search
- [FAQ: CWD-Scoped Monitoring](../faq.md#cwd-scoped-monitoring)
