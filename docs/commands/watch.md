# agtrace watch

Live "top"-style dashboard for AI coding agent sessions.

## Overview

`agtrace watch` provides a real-time TUI (Terminal User Interface) that shows session vitals as your AI coding agent runs:
- Context window usage and pressure
- Current turn and recent activity
- Tool usage signals
- Token counts and cost telemetry

## Usage

```bash
agtrace watch [OPTIONS]
```

## Behavior

### Auto-detection

`watch` can be started before or after your AI coding agent:

- **No active session:** Stays in waiting mode (or opens the latest session if available)
- **New session detected:** Automatically switches to the new session
- **No restart needed:** Keep `watch` running across multiple agent sessions

### CWD Scoping

`watch` monitors sessions scoped to the current working directory. To monitor a different project, run `watch` from that project's directory.

## Key Features

### Live Session Vitals

The dashboard displays:
- **Context Usage:** Percentage of context window used, with warnings as you approach compaction
- **Current Turn:** The active conversation turn number
- **Recent Activity:** Latest tool calls and agent actions
- **Token Telemetry:** Input/output tokens and estimated costs

### Always-On Monitoring

Keep `watch` running in a terminal pane or tmux session. It will automatically pick up new sessions without requiring a restart.

## Tips

- **Split terminal workflow:** Run `watch` in one pane, your agent in another
- **tmux integration:** Keep a persistent `watch` session running in tmux
- **Multiple projects:** `cd` to different project directories and start separate `watch` instances

## Example Workflow

```bash
# Terminal 1: Start watch
cd /path/to/your/project
agtrace watch

# Terminal 2: Start your agent (same directory)
cd /path/to/your/project
claude

# watch automatically detects and displays the new session
```

## See Also

- [session](session.md) - Inspect historical sessions
- [FAQ: CWD-Scoped Monitoring](../faq.md#cwd-scoped-monitoring)
