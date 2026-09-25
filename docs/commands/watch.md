# agtrace watch

Live multi-agent TUI for AI coding agent sessions.

## Overview

`agtrace watch` shows every live agent of the current project, Claude Code and Codex, as
one tree: main sessions, teammates, background subagents, forks, and Codex child threads.
For each agent it shows the status, context window usage, the current tool, and a timeline.
The messages the agents send each other appear in a shared feed.

See [Multi-Agent Sessions](../multi-agent.md) for how the tree is built.

## Usage

```bash
agtrace watch [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--session <ID>` (alias `--id`) | none | Watch one session tree. Accepts a session id or prefix from the index, or an agent id such as `claude:<sessionId>` or `codex:<threadId>`. |
| `--since <DUR>` | `2h` | Include root sessions active within this window (`30m`, `2h`, `1d`, …). |
| `--mode <MODE>` | `tui` | `tui` (interactive) or `console` (line output for non-TTY / CI). |
| `--all-projects` (global) | off | Watch the roots of every project instead of the current one. |
| `--project <DIR>` (global) | cwd | Target project directory. |

`watch` reads the provider log directories directly; it does not need the index. The
exception is `--session` with a plain session id, which is looked up in the index (run
`agtrace init` first).

## Scope

- **Project scope (default).** Includes every **root session** whose working directory is
  inside the project and that either
  - wrote to its log within `--since`, or
  - has a Claude process that is still running (`~/.claude/sessions/<pid>.json` with a live
    pid).

  It also includes **all descendants** of those roots, however old their files are. A
  teammate whose team config names a tracked lead is included as well. Finished roots older
  than `--since` are hidden unless you widen `--since`.
- **Session scope (`--session`).** Exactly one tree: that root and its descendants.

New agents are picked up while `watch` runs, so you can start it before or after your
agents. Discovery is bounded: it lists the project's Claude directory (plus
`subagents/` for tracked sessions) and today's and yesterday's Codex date directories. It
never walks the whole provider tree.

## The TUI

```
┌ Agents ─────────────────────────┐┌ s-lead · claude-opus-5-5[1m] · 42% of 1.0M [1m] ────────┐
│▶ s-lead            ● busy  42%  ││now   ▸ Bash mise run test  (10s)                        │
│  ├ T audit-A       ○ idle  12%  ││12:00 ⇢ spawn          audit-A (teammate, general-purpos│
│  ├ T audit-B       ○ idle   9%  ││12:00 → audit-A        NEW_TASK "review parser"          │
│  ├ S explore call  ✓ done  15%  ││12:00 ⇢ spawn          explore call sites (subagent)     │
│  └ F fork: bench   ● busy  75%  ││12:00 ⏹ interrupted                                      │
│  codex /root       ○ idle  12%  ││12:01 ← audit-A        MESSAGE "done, found 3 bugs"      │
│  ├ judge           ● busy  77%  ││12:01 ◆ model          claude-opus-5 → claude-opus-5-5   │
│  │ └ x             ● busy   5%  ││12:01 ⟲ compact        974k→112k (auto)                  │
│  └ scout           ✓ done  15%  ││12:04 ▸ Bash           mise run test                     │
└─────────────────────────────────┘└─────────────────────────────────────────────────────────┘
┌ Messages ──────────────────────────────────────────────────────────────────────────────────┐
│12:01 audit-A → s-lead            MESSAGE      "done, found 3 bugs"                         │
│12:02 /root/scout → /root         FINAL_ANSWER "3 flaky tests, all in watch_command.rs"     │
│12:02 /root → /root/judge         MESSAGE      [encrypted]                                  │
│12:02 explore call sites → s-lead TASK_NOTIFY  "found 2 call sites"                         │
└────────────────────────────────────────────────────────────────────────────────────────────┘
 scope: project demo-project · 9 agents (4 running, 3 idle) · ?:help q:quit
```

### Agents (left)

Each row is one agent, indented by depth:

- **Label.** A teammate's name, a Codex path relative to its parent row, or a subagent's
  description.
- **Kind badge.** `T` teammate, `S` subagent, `F` fork. Main sessions and Codex threads have
  no badge; Codex roots are prefixed with `codex`.
- **Status.** `● busy`, `○ idle`, `✓ done`, `✗ fail`, `⊘ kill`, or `·` (unknown).
- **Context %.** Yellow from 80%, red from 95%. It is blank while the window or the usage is
  unknown.

A collapsed node shows `▸+N`. The tree title names the scope (`Agents · session 1a2b3c4d`), and the status bar lists active view toggles with an `Esc:reset` hint.

### Focus pane (right)

The focus pane follows the selected agent.

- **Header.** `name · model · pct% of <window> [<source>]`. The source says where the
  context window size came from:
  - `cfg`: `[context_window]` in `config.toml`
  - `log`: an explicit number in the log (Codex)
  - `1m`: a `[1m]` / "(1M context)" marker (Claude)
  - `cache`: `~/.codex/models_cache.json`
  - `table`: the built-in model table
  - `obs`: bumped to fit the observed usage
- **First line.** The current activity (running tool and elapsed time), or how long ago the
  last turn ended.
- **Timeline.** The newest rows are at the bottom and the pane auto-follows until you scroll
  up. It shows:
  - user prompts and slash commands, and one-line assistant text
  - tool calls and errors, and Codex sub-actions
  - messages in and out, spawns, and lifecycle changes
  - compactions (`pre→post`) and model changes
  - turn ends and interrupts
  - prompts queued and absorbed mid-turn

### Messages (bottom)

The workspace-wide inter-agent feed: `time from → to KIND text`.

- Kinds are `MESSAGE`, `NEW_TASK`, `FINAL_ANSWER`, `HANDBACK`, `TASK_NOTIFY`, `IDLE`,
  `INTERRUPT` and `PEER`, plus spawn and lifecycle entries.
- Codex's encrypted bodies are shown as `[encrypted]`; `FINAL_ANSWER` is plaintext.
- A message logged by both the sender and the recipient is shown once.

### Status bar

The scope, agent counts, hidden agents, the number of diagnostics (log lines agtrace could
not decode), the last error, active toggles, and a key hint.

### Keybindings

| Key | Action |
|---|---|
| `j` / `k`, `↓` / `↑` | Move the selection in the tree, or scroll the focused pane by a line. |
| `Enter` | Open the selected agent (focus its timeline). |
| `space` / `←` / `→` | Collapse / expand the selected node. The only root cannot be collapsed. |
| `Esc` | Back: close help, or reset the view (expand all, show done, feed: all, focus tree, follow). |
| `Tab` / `Shift-Tab` | Cycle pane focus: tree → timeline → feed. |
| `PgUp` / `PgDn` | Scroll the focused pane by a page (the timeline when the tree has focus). |
| `Ctrl-u` / `Ctrl-d` | Scroll by half a page. |
| `G` / `End` | Jump to the tail and resume auto-follow. |
| `g` / `Home` | Jump to the top. |
| `f` | Feed filter: all messages ↔ only those involving the selected agent. |
| `h` | Hide or show agents that are done or killed. |
| `a` | Toggle auto-select of the most recently active agent. |
| `r` | Rescan for new agent files now. |
| `?` | Toggle the help overlay. |
| `q`, `Ctrl-c` | Quit. |

Scrolling up stops auto-follow. Scrolling back to the end or pressing `G` resumes it.
Changing the selection resets the timeline to follow. Moving the selection by hand turns
auto-select off.

## Console mode

`--mode tui` needs an interactive terminal. For pipes, CI, or logs, use `--mode console`.
It prints the agent tree, then one line per new timeline entry (prefixed with the agent),
then one line per feed entry, and it keeps printing as agents write:

```
$ agtrace watch --mode console --since 1d
watching project demo-project · since 1d (Ctrl-C to stop)
+ Demo project audit  idle 4%
+   [T] audit-A  killed 1%
+   [S] Count source files  done 0%
+   [F] docs-fork  done 1%
+ codex /root  done 8%
+   judge  done 4%
+   [F] forker  idle 6%
19:00 [Demo project audit] › user Audit the demo project with a small team.
19:00 [Demo project audit] ⇢ spawn audit-A (teammate, general-purpose, opus)
19:00 [Demo project audit] → audit-A MESSAGE "Please focus on error handling."
19:00 [Demo project audit] ← a0000000000000001 HANDBACK "There are 3 files in src."
19:01 [Demo project audit] ⟲ compact 973k→41k (auto)
...
19:00 [/root] → /root/judge NEW_TASK [encrypted]
19:00 [/root] ⇢ spawn judge (thread, gpt-5.6-sol)
19:00 [/root] ← /root/judge FINAL_ANSWER "The parser looks correct."
...
19:00 ✉ Demo project audit → audit-A  NEW_TASK  "Review the parser module."
19:00 ✉ /root/judge → /root  FINAL_ANSWER  "The parser looks correct."
19:00 ✉ audit-A  idle  available
19:00 ✉ Count source files → Demo project audit  HANDBACK  "There are 3 files in src."
```

## Try it

```bash
agtrace demo    # replays a synthetic multi-agent workspace; no logs of your own needed
```

## See Also

- [Multi-Agent Sessions](../multi-agent.md): the agent model, linking, and status rules
- [session](session.md): inspect historical sessions
- [FAQ: CWD-Scoped Monitoring](../faq.md#cwd-scoped-monitoring)
