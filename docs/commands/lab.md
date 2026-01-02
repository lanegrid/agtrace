# agtrace lab

Advanced history search and analysis for AI coding agent sessions.

## Overview

The `lab` command provides powerful tools for searching and analyzing session history at scale:
- Search across thousands of sessions
- Inspect raw provider events
- Analyze tool usage patterns

## Commands

### lab grep

Search session history by pattern.

```bash
agtrace lab grep <PATTERN> [OPTIONS]
```

**Arguments:**
- `PATTERN` - Text or regex pattern to search for in session events

**Options:**
- `--json` - Output matching events in JSON format
- `--raw` - Show raw provider events (before normalization)
- `--limit N` - Limit results to N matches
- `--provider <PROVIDER>` - Filter by provider (claude_code, codex, gemini)
- `--type <TYPE>` - Filter by event type (ToolCall, ToolResult, User, Message, etc.)
- `--tool <TOOL>` - Filter by tool name (only for ToolCall events)

**Examples:**

Search for tool calls:
```bash
agtrace lab grep "write_file" --json
```

Find MCP usage (raw events):
```bash
agtrace lab grep "mcp" --raw --limit 1
```

Search for specific tool usage:
```bash
agtrace lab grep "Read" --type ToolCall --limit 5
```

## Use Cases

### Analyze Tool Usage Patterns

Find all instances of a specific tool across sessions:

```bash
agtrace lab grep "read_file" --json | jq '.[] | {session: .session_id, tool: .tool_name}'
```

### Debug Schema Changes

Inspect raw provider events to understand schema changes:

```bash
agtrace lab grep "content_block" --raw --limit 5
```

### Find Specific Agent Behaviors

Search for specific assistant outputs or reasoning:

```bash
agtrace lab grep "refactor" --json
```

### Extract Data for Analysis

Export matching events for external analysis:

```bash
agtrace lab grep "error" --json > errors.json
python analyze_errors.py errors.json
```

## Performance

`lab grep` is optimized for searching large log directories:
- Searches are parallelized across sessions
- Raw log files are scanned directly (no intermediate database)
- Matches are streamed incrementally

For very large searches, use `--limit` to control output size.

## See Also

- [session](session.md) - Inspect specific sessions
- [Architecture: Schema-on-Read](../architecture.md#2-resilient-to-schema-drift) - Why `--raw` is useful
