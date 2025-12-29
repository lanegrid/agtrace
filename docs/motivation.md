# Why agtrace?

## The Problem: Session Telemetry Exists — But It's Fragmented and Hard to Use

AI coding agents behave like stateful, long-running programs. They accumulate context, call tools, read/write files, and make decisions turn by turn.

Most agents *do* provide signals like "X% remaining before compaction" — but in practice those signals are:
- **inconsistent** (different wording, thresholds, and timing across vendors)
- **easy to miss** (buried in chat output, not always surfaced at the right moment)
- **hard to compare** (no shared units, no shared event model, no shared history)

So when something goes wrong — instruction loss, constraint violations, sudden behavior drift, cost spikes — it's painful to answer basic questions:

- **What changed?** (model, context pressure, tool usage, files touched)
- **When did it start?** (the boundary where behavior shifted)
- **Why this run?** (what was different compared to previous sessions)

In short: we're operating stateful systems with no `top`, no `tail -f`, and no cross-provider trace.

## The Solution: agtrace

**agtrace** is `top` + `tail -f` for AI coding agent sessions — powered by a normalized event timeline.

It reads local provider logs, normalizes them into a consistent model, and turns them into views you can actually operate with:
- a live "top" view (`watch`) for context pressure and activity
- a history you can query (`session` / `lab`) to understand what happened and when

agtrace's core loop:

1. **Capture** local provider logs (no cloud)
2. **Normalize** them into a single event model across providers
3. **Index** metadata without duplicating large logs (pointer-based)
4. **Analyze** sessions into turns/steps/metrics (schema-on-read)
5. **Visualize** live and historical runs in human-friendly views

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

*Live demo of `agtrace watch` — a `top`-like view for your session*

![agtrace watch TUI dashboard](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/watch-screenshot-claude.png)

*The dashboard showing context usage, current turn, and token costs*
