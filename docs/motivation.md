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

## The Solution: agtrace as a Platform

**agtrace** is the observability platform for AI coding agent sessions — powered by a normalized event timeline.

It reads local provider logs, normalizes them into a consistent model, and provides both:
- **A CLI application** (`agtrace-cli`): A reference implementation with live `watch` TUI and session analysis commands
- **A public SDK** (`agtrace-sdk`): Stable APIs for building custom monitoring tools

### The Platform Approach

Rather than being "just a CLI tool," agtrace is architected as a **platform** with multiple consumers:

**For End Users (CLI)**:
- Live "top" view (`watch`) for context pressure and activity
- History queries (`session` / `lab`) to understand what happened and when

**For Developers (SDK)**:
- Build vital-checkers (dead man's switches for agent activity)
- Create IDE integrations (VS Code, Neovim plugins)
- Build team dashboards and analytics
- Integrate with existing observability stacks

### Why a Platform?

Different use cases demand different interfaces:
- **Solo developers** want a fast, local TUI (`agtrace watch`)
- **Teams** want centralized dashboards and alerts
- **IDE users** want inline session replay and cost tracking
- **Researchers** want programmatic access to event streams

By providing a stable SDK layer, agtrace enables an **ecosystem** of tools rather than being a monolithic application.

### Core Loop

1. **Capture** local provider logs (no cloud)
2. **Normalize** them into a single event model across providers
3. **Index** metadata without duplicating large logs (pointer-based)
4. **Analyze** sessions into turns/steps/metrics (schema-on-read)
5. **Expose** via CLI (for humans) and SDK (for developers)
