# agtrace watch

Live multi-agent TUI for AI coding agent sessions.

## Overview

`agtrace watch` shows the **sessions** of the current project, Claude Code and Codex,
live ones first. Each session is a tree: its main session or Codex root thread with its
teammates, background subagents, forks, and Codex child threads. For each agent it shows
the status, context window usage, the current tool, and a timeline. The messages the
agents send each other appear in a shared feed.

A **navigator** on the left is always visible: the scope, its sessions, and each
session's agent tree, finished agents folded into one node. `↑` / `↓` move through it,
`→` / `←` expand, collapse and walk up and down the hierarchy. The right side shows the
selected node: the **overview** of every session (status, context, an activity lane over
time, and what each agent is doing now), one session's overview, or an agent's **detail**
(what it was asked to do, what it is doing, what it produced), above the messages scoped
to that node.

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
`subagents/` for tracked sessions) and today's and yesterday's Codex date directories.
Older Codex date directories are listed every 30 s when the scope reaches them: the days
overlapping a wider `--since`, or, with `--session`, every day since the Codex root was
created (a long-running tree's threads live in the directories of their own start dates).
It never walks the whole provider tree (except once, to find an old `--session` root).

## Sessions

A **session** is one top-level tree: a Claude main session or a Codex root thread and
everything below it. Other transcripts of the same *logical* session are folded into it
instead of being listed as separate sessions:

| Transcript | Shown as |
|---|---|
| continued in another one (`continued-in`) | `earlier transcript · <name>` |
| its id is the runtime session id another transcript was written under (a resumed / respawned process) | `earlier transcript · <name>` |
| no conversation of its own (only local commands such as `/clear` or `/resume`), carrying the name (`agent-name` / `ai-title`) of another session | `/clear · <name>` |

A live Claude process that has not written a transcript yet (a background job waiting for
work) is listed as a session too, with `no transcript yet`.

**Liveness.** A Claude session is *live* while one of its processes is registered
(`~/.claude/sessions/<pid>.json`) with a live pid: `busy` when the registry says so or one
of its agents is running, else `idle`. A Codex session is `busy` while one of its agents
runs (a turn is open) and `idle` while its tree was written to in the last 30 minutes.
Other sessions have *ended*: **recent** when written to in the last hour, **older**
otherwise.

**Order.** Busy, idle, recent, older; within each group the most recently active first.
The navigator and the overview use the same order.

**Name.** The first of: the registry name (when it is not just an id or derived from the
directory, like `yohaku-studio-c8`), the session's own name (`agent-name`, the title; a
Codex thread's nickname), an excerpt of its first prompt, its first slash command with
the short id (`/resume · fa330341`), or the short id. `(bg)` marks background (daemon /
job) sessions. Session nodes and root rows carry the session's name (the console keeps the
agent labels).

## The TUI

The TUI has no screens to switch between. A **navigator** is always on the left; the
right side shows the **content** of the node selected in it, above the **messages**
scoped to that node:

```
┏ ▶ Navigator ━━━━━━━━━━━━━━━━━━━━━━━━━┓┌ Overview · project demo-project · activity: last 15m ──────────────────────────────┐
┃▶◆ demo-project  4 live               ┃│ Sessions · 4 live, 1 recent, 1 older  · pick one in the navigator                  │
┃ ▸ ● Audit the parser with a tea…  42%┃│   claude   Audit the parser with a tea… ● busy      5 (2 run)    42%  now ▸ Bash … │
┃ ▸ ● codex triage the flaky tests  12%┃│   codex    triage the flaky tests       ● busy      4 (2 run)    12%  now "Reprodu… │
┃ ▾ ● Ship the release              25%┃│  agent                 status context     activity 1m/cell   now                   │
┃  ├ ● S step r0                       ┃│ Audit the parser with a team of… · busy · up 5m · claude-opus-5-5[1m] · ctx ███…   │
┃  └ ▸ ⊘ 8 killed · ✓ 3 done · 1 earli…┃│  Audit the parser wit… ● busy ███░░░  42%              ▅⟲▂·▂ ▸ Bash mise run te…  │
┃   ○ f00dcafe                         ┃│  ├ T audit-A           ○ idle █░░░░░  12%              ▃▂    idle 3m              │
┃   ✓ rename the config keys           ┃│  └ F fork: bench       ● busy █████░  75%              ▂···▂ ▸ Bash cargo bench…  │
┃ ▸ ✓ 1 older session                  ┃│ Ship the release · idle (bg) · up 4m · claude-opus-5 · ctx ███░░░░░░░ 25% of 200k… │
┃                                      ┃│  Ship the release      ○ idle ██░░░░  25%              ▆··▆  idle 1m              │
┃                                      ┃│  └ S step r0           ● busy                          ▂···▂ ▸ Bash cargo publish…│
┃                                      ┃│    ✓ 3 finished · ⊘ 8 killed · 1 earlier transcript  (d to show)                   │
┃                                      ┃└────────────────────────────────────────────────────────────────────────────────────┘
┃                                      ┃┌ Messages · all ────────────────────────────────────────────────────────────────────┐
┃                                      ┃│12:03 step d0 done                                                                  │
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛└────────────────────────────────────────────────────────────────────────────────────┘
 demo-project  6 sessions (4 live) · 25 agents (5 run) · 12 folded       ↑↓ move · → open · ← back · ↵ read · / find · ? help
```

`↑` / `↓` move through the navigator and the content follows at once. `→` and `←` walk
the hierarchy: `→` expands a node, then goes to its first child, and on an agent without
children moves the focus to the content to read it; `←` collapses a node, else goes to its
parent. `Enter` focuses the content, `Esc` goes back. `?` lists every key.

### Navigator (left)

| Node | Shown as | Content (right) | Messages |
|---|---|---|---|
| **Scope** (top) | `◆ <project>  N live` (`all projects`, `session <id>`) | the overview of every session | all |
| **Session** | `▸ ● <name>` (`codex` prefix for Codex) with its state glyph and root context | that session's overview | the session's |
| **Agent** | `├ ○ T audit-A`: status glyph, kind badge, label, context | the agent's detail | the agent's |
| **Folded group** | `└ ▸ ⊘ 8 killed · ✓ 3 done · 1 earlier (d)` | the folded agents as overview rows | theirs |
| **Older sessions** | `▸ ✓ 2 older sessions` | the older sessions, one line each | theirs |

- **Sessions** are ordered busy, idle, recent (see [Sessions](#sessions)); sessions that
  ended more than an hour ago collapse into one *older sessions* node while something newer
  is in scope. A session node stands for its root agent too: its children are the root's
  children. Sessions start collapsed when there are several, expanded when there is one.
- **Agents** are indented by depth. Status: `● busy`, `○ idle`, `✓ done`, `✗ failed`,
  `⊘ killed`, `·` unknown. Kind badge: `T` teammate, `S` subagent, `F` fork (main sessions
  and Codex threads have none). Labels: a teammate's name, a subagent's description, a Codex
  path relative to its parent, `earlier transcript · <name>` / `/clear · <name>` for the
  session's other transcripts. The context percentage is yellow from 80%, red from 95%.
- **Folded groups.** A parent's finished children (done or killed, with everything below
  them finished too) and the session's earlier / `/clear` transcripts fold into one node,
  its last child, when there are at least two of them; a single one stays in place. Failed
  agents never fold. `→` on the group lists its items below it; `d` shows every finished
  agent in place instead (again to fold).
- `▶` marks the selection, `▸` / `▾` a collapsed / expanded node. The selection stays
  highlighted while another pane has focus. When the list is longer than the pane, the
  bottom border shows how many rows are above and below (`↑3 ↓10`).

`/` filters the navigator: type part of an agent's name (or id, or agent type); only the
matching agents and their ancestors remain (folds and collapses are ignored, so finished
agents are found), matches are highlighted and the first one is selected. `↑` / `↓` move,
`Enter` keeps the filter, `Esc` clears it. The pane title shows the filter and the number
of matches (`/audit: 2 matches`).

On terminals narrower than 80 columns, `s` hides the navigator so the content gets the
full width (`s` again shows it; the keys keep moving the selection, and the breadcrumb in
the status bar says where you are).

### Overview (scope and session nodes)

The scope's overview has, with more than one session, a summary of the live and recent
ones on top (a third of the pane at most; left out when the pane is narrow). Then per
session a header line and its agents in place; a session node shows the same for that
session alone.

- **Header line**: the session's name, the root's status (`(bg)` for background
  sessions) and age, model, context bar with the window size and its source, the number of
  compactions (`⟲N`), how many agents its tree has (and how many run), and its reasoning
  effort.
- **Rows**, in tree order: **status** and **context** (bar and percent);
  **activity**: one cell per time slice, oldest on the left, the current minute on the
  right. The glyph height (`▁▂▃▅▆`) is the event density (tool calls, messages, prompts,
  assistant text); the colour is the status at that time (green running, blue idle, grey
  ended); `⟲` marks a compaction; `·` means running without new events (a long tool call);
  blank is idle, ended, or before the agent existed. The cell width adapts to the pane;
  `+` / `]` widen the window (15m → 60m → 4h → all) and `-` / `[` narrow it. When no agent
  had activity in the window, the column header says `no activity — + widens`;
  **now**: the running tool and its elapsed time; between tools, the task in progress
  (`▸ Running the tests`, its active form) or else the latest assistant text; `idle Xm`,
  the result excerpt of a finished agent, or how long ago it was killed / failed (and why).
- After a session's rows, its folded agents in one line:
  `✓ 18 finished · ⊘ 3 killed · 1 earlier transcript  (d to show)`. Older sessions are
  left to the navigator (`▸ 2 older sessions — in the navigator`).

With the content focused, `↑` / `↓`, `PgUp` / `PgDn`, `Ctrl-u` / `Ctrl-d`, `g` / `G`
scroll the rows (the summary and the column header stay).

### Agent detail (agent nodes)

```
┏ ▶ explore call sites ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃explore call sites · subagent of s-lead · Explore · claude-sonnet-5 · ✓ done 2m          ┃
┃ctx ██░░░░░░░░ 15% of 200k [table]  ▂   in 30k / out 10 · 1 tools                        ┃
┃── [i] Instructions (1) ──────────────────────────────────────────────────────────────── ┃
┃[12:00 spawned by s-lead] Find call sites of parse_line                         … press i┃
┃── [n] Now ───────────────────────────────────────────────────────────────────────────── ┃
┃said: Two call sites: decoder.rs and lab.rs.                                    … press n┃
┃── ▶ [r] Result ──────────────────────────────────────────────────────────────────────── ┃
┃[12:02 TASK_NOTIFY]                                                                      ┃
┃  found 2 call sites                                                                     ┃
┃── [t] Timeline (3) ──────────────────────────────────────────────────────────────────── ┃
┃12:00 › user Find call sites of parse_line                                               ┃
┃12:00 ▸ Grep parse_line                                                                  ┃
┃12:02 •      Two call sites: decoder.rs and lab.rs.                                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

- **Header.** The agent, how it relates to its parent (`teammate of lead (team t)`,
  `subagent of lead`, `thread of /root`, `main session`), its type, model and reasoning
  effort (`effort high`: the latest one in its own log, else the one its spawn call
  requested), and the status with the time spent in it.
- **Context line.** Context bar and window with its source (`cfg`: `[context_window]` in
  `config.toml`; `log`: a number in the log (Codex); `1m`: a `[1m]` / "(1M context)"
  marker (Claude); `cache`: `~/.codex/models_cache.json`; `table`: the built-in model
  table; `obs`: bumped to fit the observed usage), a sparkline of the context occupancy
  over the agent's usage records (`⟲` = compaction), and totals: input and output tokens
  summed over requests (a request logged more than once counts once; `≥` marks output
  counts that are only stream-start snapshots), turns and tool calls.
- **Instructions.** What the agent was asked to do, in full (wrapped, not cut to one
  line): its initial task (a subagent's or fork's prompt, a teammate's first `NEW_TASK`, a
  main session's first prompt), then later prompts, prompts queued mid-turn, and messages
  addressed to it (follow-ups from its lead, peer messages). Reports from its own children
  are not instructions. If the child's own log has no task yet, the spawn call's prompt (or
  description) is shown. Codex encrypts inter-agent bodies: the task shows as
  `[encrypted by Codex]` with the sender, the agent path, the task name and the requested
  model.
- **Now.** The running tool with its elapsed time, the plan, and the latest assistant
  text (or reasoning when there is no text): what the agent is trying to do. The plan
  shows a Codex thread's goal (`Goal: <objective> (active)`); the task list (Claude
  `TaskCreate` / `TaskUpdate`, legacy `TodoWrite`: `☐` pending, `▸` in progress with its
  active form, `✓` completed; Agent Teams share one list, so a teammate's change also shows
  in the lead's list, marked `[teammate]`); and the latest plan-mode plan (Codex `Plan`
  item).
- **Result.** The agent's `FINAL_ANSWER` / hand-back / task-notification result. A finished
  agent without one shows its last assistant message; a killed or failed one shows why,
  when known.
- **Timeline.** Newest at the bottom, following the end until you scroll up: user prompts
  and slash commands, assistant text, tool calls and errors, Codex sub-actions, messages in
  and out, spawns, lifecycle changes, compactions (`pre→post`), model changes, turn ends and
  interrupts, prompts queued and absorbed mid-turn.

`i` / `n` / `r` / `t` show the Instructions / Now / Result / Timeline section and focus
the content, from any pane: on a session node they show its root agent's detail (the
navigator stays on the session), on the scope node the first session's root, on a folded
group its first item. The focused section is cyan, marked `▶`, named in the breadcrumb,
and gets the room it needs; the others are shown in full when they fit, partly when rows
are left, or collapsed to a one-line summary ending in `… press <key>`. With the content
focused, `↑` / `↓`, `PgUp` / `PgDn`, `Ctrl-u` / `Ctrl-d` scroll the section and `G` / `g`
jump to its end / top (the timeline follows its end again). Section headings show the
visible range (`1-10/40 ↓`). Texts are kept up to 16 KiB each; runs of blank lines are
shown as one.

The detail opens where the agent's state is: **Now** while it is running, **Result** once
it ended (done, killed, failed) with a result, else the **Timeline**, and never on an empty
section when another one has content. Once you pick a section with a section key, the next
agents you select start on it too, unless that agent has nothing there.

### Messages (bottom right)

Inter-agent messages, spawns and lifecycle changes scoped to the selection: everything
for the scope node, the session's for a session node, those an agent sent, received or
emitted for an agent node (`Messages · audit-A`). One row per entry:
`time from → to KIND text`.

- Kinds are `MESSAGE`, `NEW_TASK`, `FINAL_ANSWER`, `HANDBACK`, `TASK_NOTIFY`, `IDLE`,
  `INTERRUPT` and `PEER`, plus spawn and lifecycle entries.
- Codex's encrypted bodies are shown as `[encrypted]`; `FINAL_ANSWER` is plaintext.
- A message logged by both the sender and the recipient is shown once.

The newest entries are at the bottom; with the messages focused (`Tab`), scrolling up stops
following the end and `G` resumes it.

### Status bar

- **Left.** The breadcrumb of the selection, `project › session › agent · SECTION` (`FIND`
  with the filter being typed and its number of matches), then the sessions in scope and how
  many are live, the agents and how many run, how many finished agents are folded, the
  number of diagnostics (log lines agtrace could not decode), the last error and the active
  toggles (`done shown`, `auto`). For about two seconds after a change, a short message
  replaces them: `▾ expanded judge`, `finished agents shown in place — d to fold`,
  `activity window: last 4h`, `filter cleared`, `view reset`, `rescanning…`, and so on.
  Keys that cannot act say why (`audit-A has no children`).
- **Right.** Key hints for the focused pane, shortened on narrow terminals (`→ open`,
  `← nav` and `? help` always stay).

### Keybindings

| Key | Navigator (focused by default) | Content / messages |
|---|---|---|
| `↑` / `↓`, `k` / `j` | Move the selection; the content follows. | Scroll by a line. |
| `→` / `l` | Expand; if expanded, go to the first child; on an agent without children, focus the content. | — |
| `←` / `h` | Collapse; if collapsed (or a leaf), go to the parent. | Back to the navigator. |
| `Enter` | Focus the content (read / scroll). | — |
| `Esc` | Clear the `/` filter; else go to the scope node; there, reset the view (collapse to the defaults, fold finished agents). | Back to the navigator. |
| `i` / `n` / `r` / `t` | Show that section of the selected agent's detail (a session's root agent; see above) and focus the content. | Switch the section. |
| `Tab` / `Shift-Tab` | Cycle the focus: navigator → content → messages. | Same. |
| `PgUp` / `PgDn`, `Ctrl-u` / `Ctrl-d` | Move the selection by a page / half a page. | Scroll by a page / half a page. |
| `g` / `Home`, `G` / `End` | First / last row. | Top / end (the timeline and the messages follow the end again). |
| `/` | Filter the navigator by name (see above). | Same (focuses the navigator). |
| `d` | Show finished (done / killed) agents and earlier transcripts in place, or fold them again. | Same. |
| `space` | Expand / collapse the selected node. | — |
| `+` / `]`, `-` / `[` | Activity window of the overview: wider / narrower (15m, 60m, 4h, all). | Same. |
| `s` | Hide / show the navigator (below 80 columns). | Same. |
| `A` | Follow the most recently active agent shown in the navigator. | Same. |
| `R` | Rescan for new agent files now. | Same. |
| `?` | Toggle the help overlay. | Same. |
| `q`, `Ctrl-c` | Quit. | Same. |

Selecting another node starts its content over (the detail picks its section again,
scrolls go back to the top, the timeline and the messages follow their end). Moving the
selection by hand turns `A` off.

## Console mode

`--mode tui` needs an interactive terminal. For pipes, CI, or logs, use `--mode console`.
It prints the agent tree (every session, nothing folded), then one line per new timeline
entry (prefixed with the agent),
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
