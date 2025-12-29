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
- `--json` - Output in JSON format for programmatic use
- `--limit N` - Show only the N most recent sessions

**Example:**
```bash
agtrace session list --limit 10
```

### session show

Show detailed information about a specific session.

```bash
agtrace session show <SESSION_ID> [OPTIONS]
```

**Output includes:**
- Session metadata (ID, timestamps, model)
- Context window usage over time
- Turn-by-turn breakdown
- Tool usage summary
- Token counts and costs

**Options:**
- `--json` - Output in JSON format
- `--raw` - Show raw provider events (useful for debugging)

**Example:**
```bash
agtrace session show abc123def456
agtrace session show abc123def456 --json > session.json
```

## Use Cases

### Compare Sessions

Compare two sessions to understand what changed:

```bash
agtrace session show session1 --json > s1.json
agtrace session show session2 --json > s2.json
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
agtrace session show <SESSION_ID> --json | jq '.turns[].steps[] | select(.type == "tool_call")'
```

## See Also

- [watch](watch.md) - Live session monitoring
- [lab](lab.md) - Advanced history search
- [FAQ: CWD-Scoped Monitoring](../faq.md#cwd-scoped-monitoring)
