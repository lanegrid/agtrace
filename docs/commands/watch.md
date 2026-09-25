# agtrace watch

Live multi-agent TUI for AI coding agent sessions.

## Overview

`agtrace watch` shows every live agent of the current project, Claude Code and Codex, as
one tree: main sessions, teammates, background subagents, forks, and Codex child threads.
For each agent it shows the status, context window usage, the current tool, and a timeline.
The messages the agents send each other appear in a shared feed.

It opens on an **overview** of every agent (status, context, an activity lane over time,
and what each one is doing now). From there you can drill into one agent's **detail**:
what it was asked to do, what it is doing, and what it produced.

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

## The TUI

The TUI has three screens:

| Key | Screen | Answers |
|---|---|---|
| `1` | **Overview** (start screen) | Which agents exist, which are running / idle / done over time, how full their context is, what each is doing now |
| `2` | **Agents** | The agent tree next to the selected agent's timeline and the message feed |
| `Enter` | **Agent detail** | What one agent was asked to do, what it is doing, what it produced |

`Enter` (or `→` / `l`) on an agent opens its detail, from the overview or the agents
screen; `i` / `n` / `r` / `t` open it directly at the Instructions / Now / Result /
Timeline section. `Esc` returns to the screen you came from, with the same agent
selected, and is always safe to press: it steps back one level at a time. `?` lists
every key.

To find an agent among many, press `/` and type part of its name (or id, or agent
type): the overview and the tree keep only the matches and their parents (folds are
ignored), and the first match is selected. `↑` / `↓` move between matches, `Enter`
opens the selected one's detail and keeps the filter, `Esc` clears it. The pane title
shows the filter and the number of matches (`/audit: 2 matches`).

### Overview (`1`)

```
┏ ▶ Overview · project demo-project · activity: last 15m ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  agent                     status context     activity 1m/cell                     now              ┃
┃ codex /root · idle · up 4m · gpt-5.6 · ctx █░░░░░░░░░ 12% of 258k [log] · ⟲1 · 4 agents (2 running) ┃
┃  codex /root               ○ idle █░░░░░  12%                                ▃·⟲   idle 2m          ┃
┃▶ ├ judge                   ● busy █████░  77%                                ▂▃▂·· ▸ exec cargo tes…┃
┃  │ └ x                     ● busy █░░░░░   5%                                 ···▂ "Reproducing th…┃
┃  └ scout                   ✓ done █░░░░░  15%                                ▂▂    result: "3 fla…┃
┃ s-lead · busy · up 5m · claude-opus-5-5[1m] · ctx ████░░░░░░ 42% of 1.0M [1m] · ⟲1 · 5 agents       ┃
┃  s-lead                    ● busy ███░░░  42%                                ▅⟲▂·▂ ▸ Bash mise run …┃
┃  ├ T audit-A               ○ idle █░░░░░  12%                                ▃▂    idle 3m          ┃
┃  ├ S explore call sites    ✓ done █░░░░░  15%                                ▃·▂   result: "found 2…┃
┃  └ F fork: bench           ● busy █████░  75%                                ▂···▂ ▸ Bash cargo ben…┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
┌ Messages ───────────────────────────────────────────────────────────────────────────────────────────┐
│12:02 explore call sites → s-lead TASK_NOTIFY  "found 2 call sites"                                  │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
 OVERVIEW  9 agents (4 running, 3 idle)         ↵ detail · / find · +/- window · space fold · ? help
```

Each root session gets a header line: its label, status and age, model, context bar with
the window size and its source, the number of compactions (`⟲N`), how many agents its
tree has (and how many are running), and its reasoning effort. Below it, one row per agent in tree order:

- **status** and **context** (bar and percent; the source of the window size is in the
  root header and in the agent detail);
- **activity**: one cell per time slice, oldest on the left, the current minute on the
  right. The glyph height (`▁▂▃▅▆`) is the event density (tool calls, messages, prompts,
  assistant text). The colour is the status at that time: green running, blue idle, grey
  ended. `⟲` marks a compaction. `·` means running without new events (a long tool call).
  A blank cell is idle, ended, or before the agent existed. The cell width adapts to the
  terminal width; `+` / `]` widen the window (15m → 60m → 4h → all) and `-` / `[` narrow
  it;
- **now**: the running tool and its elapsed time; between tools, the task in progress
  (`▸ Running the tests`, its active form) or else the latest assistant text; `idle Xm`, the result excerpt of a finished agent, or how long ago it
  was killed / failed (and why, when known).

`j` / `k` select a row, `Enter` opens its detail. When the list is longer than the
screen, the bottom border shows how many rows are above and below (`↑3 ↓10`). When no
agent had activity in the window (an old session), the activity column header says
`no activity — + widens`. `space` (fold), `d` (hide done), `f`
(feed filter) and `a` (auto-select) work as on the agents screen; the rows follow the same
fold and hide state. The bottom strip is the message feed (newest entries).

### Agent detail (`Enter`)

```
┏ ▶ Detail · explore call sites ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
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
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
 DETAIL · RESULT            i/n/r/t section · j/k scroll · J/K agent · G/g end/top · Esc back
```

- **Header.** The agent, how it relates to its parent (`teammate of lead (team t)`,
  `subagent of lead`, `thread of /root`, `main session`), its type, model and reasoning
  effort (`effort high`: the latest one in its own log, else the one its spawn call
  requested), and the status with the time spent in it.
- **Context line.** Context bar and window (with its source), a sparkline of the context
  occupancy over the agent's usage records (`⟲` = compaction), and totals: input and
  output tokens summed over requests (a request logged more than once counts once; `≥`
  marks output counts that are only stream-start snapshots), turns and tool calls.
- **Instructions.** What the agent was asked to do, in full (wrapped, not cut to one
  line): its initial task (a subagent's or fork's prompt, a teammate's first
  `NEW_TASK`, a main session's first prompt), then later prompts, prompts queued
  mid-turn, and messages addressed to it (follow-ups from its lead, peer messages).
  Reports from its own children are not instructions. If the child's own log has no task
  yet, the spawn call's prompt (or description) is shown. Codex encrypts inter-agent
  bodies: the task shows as `[encrypted by Codex]` with the sender, the agent path, the
  task name and the requested model.
- **Now.** The running tool with its elapsed time, the plan, and the latest assistant
  text (or reasoning when there is no text): what the agent is trying to do. The plan
  shows:
  - a Codex thread's goal (`Goal: <objective> (active)`);
  - the task list (Claude `TaskCreate` / `TaskUpdate`, legacy `TodoWrite`): `☐` pending,
    `▸` in progress (with its active form), `✓` completed. Agent Teams share one list: a
    teammate's change also shows in the lead's list, marked `[teammate]`, and a task a
    teammate only updated shows in its own detail too;
  - the latest plan-mode plan (Codex `Plan` item), wrapped.
- **Result.** The agent's `FINAL_ANSWER` / hand-back / task-notification result. A
  finished agent without one shows its last assistant message; a killed or failed one
  shows why, when known.
- **Timeline.** The same rows as the agents screen's timeline.

Each section heading names the key that focuses it: `i` Instructions, `n` Now, `r`
Result, `t` Timeline (`Tab` / `Shift-Tab` cycle through them). The focused section is
cyan, marked `▶`, named in the status bar, and gets the room it needs. The other
sections are shown in full when they fit (short ones always), partly when rows are
left, or else collapsed to a one-line summary ending in `… press <key>`: the latest
instruction, the running tool (or the task list, or the latest text), the result, the
newest timeline row. `j` / `k`, `PgUp` / `PgDn`, `Ctrl-u` / `Ctrl-d` scroll the focused
section; `G` / `g` jump to its end / top (the timeline follows its end again). Section
headings show the visible range (`1-10/40 ↓`). Texts are kept up to 16 KiB each; runs of
blank lines are shown as one.

The detail opens where the agent's state is: **Now** while it is running, **Result**
once it ended (done, killed, failed) with a result, else the **Timeline**, and never on
an empty section when another one has content. Once you pick a section (with a section
key or `Tab`), the details of the next agents you open start on it too, unless that
agent has nothing there. `J` / `K` step to the next / previous agent (in tree order)
without leaving the detail; `Esc` still returns to where you opened the first one.

### Agents screen (`2`)

```
┏ ▶ Agents · project demo-project ┓┌ s-lead · claude-opus-5-5[1m] · 42% of 1.0M [1m] ────────┐
┃▶ s-lead            ● busy  42%  ┃│now   ▸ Bash mise run test  (10s)                        │
┃  ├ T audit-A       ○ idle  12%  ┃│12:00 ⇢ spawn          audit-A (teammate, general-purpos│
┃  ├ T audit-B       ○ idle   9%  ┃│12:00 → audit-A        NEW_TASK "review parser"          │
┃  ├ S explore call  ✓ done  15%  ┃│12:00 ⇢ spawn          explore call sites (subagent)     │
┃  └ F fork: bench   ● busy  75%  ┃│12:00 ⏹ interrupted                                      │
┃  codex /root       ○ idle  12%  ┃│12:01 ← audit-A        MESSAGE "done, found 3 bugs"      │
┃  ├ judge           ● busy  77%  ┃│12:01 ◆ model          claude-opus-5 → claude-opus-5-5   │
┃  │ └ x             ● busy   5%  ┃│12:01 ⟲ compact        974k→112k (auto)                  │
┃  └ scout           ✓ done  15%  ┃│12:04 ▸ Bash           mise run test                     │
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛└─────────────────────────────────────────────────────────┘
┌ Messages ──────────────────────────────────────────────────────────────────────────────────┐
│12:01 audit-A → s-lead            MESSAGE      "done, found 3 bugs"                         │
│12:02 /root/scout → /root         FINAL_ANSWER "3 flaky tests, all in watch_command.rs"     │
│12:02 /root → /root/judge         MESSAGE      [encrypted]                                  │
│12:02 explore call sites → s-lead TASK_NOTIFY  "found 2 call sites"                         │
└────────────────────────────────────────────────────────────────────────────────────────────┘
 AGENTS  9 agents (4 running, 3 idle)  ↵ detail · / find · space fold · f msgs · 1 overview · ? help
```

Three panes answer three questions: **Agents** (who is there and in what state), the
**timeline** (what the selected agent is doing), and **Messages** (how the agents talk to
each other). `Enter` opens the selected agent's detail; `Tab` moves the focus to the
timeline or the feed, and `Esc` comes back to the tree. The focused pane has a thick cyan
border and a `▶` in its title; the other panes are dimmed.

#### Agents (left)

Each row is one agent, indented by depth:

- **Label.** A teammate's name, a Codex path relative to its parent row, or a subagent's
  description.
- **Kind badge.** `T` teammate, `S` subagent, `F` fork. Main sessions and Codex threads have
  no badge; Codex roots are prefixed with `codex`.
- **Status.** `● busy`, `○ idle`, `✓ done`, `✗ fail`, `⊘ kill`, or `·` (unknown).
- **Context %.** Yellow from 80%, red from 95%. It is blank while the window or the usage is
  unknown.

A collapsed node shows `▸+N`. The tree title names the scope (`Agents · session 1a2b3c4d`).
The selected row stays highlighted while another pane has focus (reversed when the tree
has focus, bold and underlined otherwise), so you can always see whose timeline is shown.

#### Focus pane (right)

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

#### Messages (bottom)

The workspace-wide inter-agent feed: `time from → to KIND text`.

Entries involving the selected agent have a bold cyan route. While the timeline has
focus, the other entries are dimmed (highlighted, not filtered; `f` filters).

- Kinds are `MESSAGE`, `NEW_TASK`, `FINAL_ANSWER`, `HANDBACK`, `TASK_NOTIFY`, `IDLE`,
  `INTERRUPT` and `PEER`, plus spawn and lifecycle entries.
- Codex's encrypted bodies are shown as `[encrypted]`; `FINAL_ANSWER` is plaintext.
- A message logged by both the sender and the recipient is shown once.

### Status bar

- **Left.** The mode: `OVERVIEW`, `AGENTS`, `TIMELINE · <agent>`, `MESSAGES`,
  `DETAIL · <SECTION>` (the agent is named in the detail's title), or `FIND` with the
  filter being typed and its number of matches. Then (except on the detail) the agent
  counts, hidden agents, the number of diagnostics (log lines agtrace could not decode),
  the last error and the active toggles. For about two seconds after a change, a short
  message replaces them: `▸ collapsed judge (+1 hidden)`, `done agents hidden (2) — d to
  show`, `messages: s-lead only — f for all`, `activity window: last 4h`, `view reset`,
  `filter cleared`, `rescanning…`, and so on. Keys
  that cannot act say why (`can't collapse the only session`, `x has no children`).
- **Right.** Key hints for the screen and the focused pane. The overview and the tree add
  `Esc:reset` while a view toggle is active (`Esc:clear filter` while a `/` filter is
  set). Hints are shortened on narrow terminals, keeping `↵ detail`, `? help` and
  `Esc back`.

### Keybindings

| Key | Action |
|---|---|
| `1` / `2` | Overview / agents screen (also from the detail). |
| `Enter` / `→` / `l` | Open the selected agent's detail. |
| `Esc` / `←` / `h` | Back one level: close help; leave the detail (to the screen it was opened from); return from the timeline / feed to the tree; clear the `/` filter; then (on the tree or the overview) reset the view: expand all, show done, feed: all, follow. The selection is kept. |
| `i` / `n` / `r` / `t` | Detail: focus the Instructions / Now / Result / Timeline section (remembered for the next agents). Overview, agents screen: open the selected agent's detail at that section. |
| `j` / `k`, `↓` / `↑` | Act on the focused pane: move the selection (overview, tree), or scroll the timeline / feed / detail section by a line. |
| `J` / `K` | Next / previous agent: in the detail, show that agent's detail; elsewhere, move the selection. |
| `/` | Find agents by name: type to filter, `↑` / `↓` select, `Enter` opens the selected match (the filter stays), `Esc` clears it. From the detail, returns to the screen it was opened from first. |
| `+` / `]`, `-` / `[` | Overview activity window: wider / narrower (15m, 60m, 4h, all). |
| `space` | Fold / unfold the selected node. The only root cannot be folded. |
| `Tab` / `Shift-Tab` | Agents screen: cycle pane focus (tree → timeline → feed). Detail: cycle sections. |
| `PgUp` / `PgDn` | Scroll the focused pane or section by a page (the timeline when the tree has focus; on the overview, move the selection). |
| `Ctrl-u` / `Ctrl-d` | Scroll by half a page. |
| `G` / `End` | Jump to the tail and resume auto-follow. |
| `g` / `Home` | Jump to the top. |
| `f` | Feed filter: all messages ↔ only those involving the selected agent. |
| `d` | Hide or show agents that are done or killed. |
| `a` | Toggle auto-select of the most recently active agent. |
| `R` | Rescan for new agent files now (it was `r`, now the Result section key). |
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
