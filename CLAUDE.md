# CLAUDE.md

This file provides guidance for Claude Code when working in this repository.

## Required

- **Always load `/git-workflow` first** regardless of the conversation topic (before any other task)
- Before starting work -> `mise tasks` to see available tasks
- All commands MUST be run via `mise run` (works from any directory)
- Prefer root-cause fixes over shortcuts (step back, reflect, take the senior engineer approach)
- **All changes go through PRs via `/git-workflow`** (never commit directly to main)
- Commit messages must be one-line, Conventional Commits format
- Write everything in English (OSS project)

## Project Overview

Rust-based local-first observability platform for AI agents. Normalizes diverse agent log formats (Claude Code, Codex, Gemini) into a unified `AgentEvent` timeline.

```
./
├── crates/
│   ├── agtrace-types/       # Type definitions (no deps)
│   ├── agtrace-core/        # File paths, env, workspace utils
│   ├── agtrace-providers/   # Log parsers (claude/, codex/, gemini/)
│   ├── agtrace-index/       # SQLite pointer index
│   ├── agtrace-engine/      # Session assembly, token calc
│   ├── agtrace-runtime/     # Orchestration layer
│   ├── agtrace-sdk/         # Public SDK
│   ├── agtrace-cli/         # CLI (depends on sdk only)
│   └── agtrace-testing/     # Shared test utilities
└── docs/                    # Design documents
```

## Quick Reference

```bash
mise run verify              # fmt + clippy + test + build (full check)
mise run test                # run all tests
mise run clippy              # lint
mise run fmt                 # format code
mise run build:release       # release build
mise run git:status          # git workflow state + next action
mise tasks                   # list all tasks
```

## Skills

Detailed knowledge is separated into `.claude/skills/`. Loaded automatically or manually via `/skill-name`.

| Skill | Purpose |
|-------|---------|
| `/git-workflow` | Git workflow, branch management, PR creation |
| `/agtrace-provider-normalization` | Provider schema investigation and domain abstraction |

**Usage**: Type `/skill-name` in conversation, or auto-loaded when relevant.

## Project Rules

- Snapshot tests: `mise run snapshot:accept` to accept and review diff, then include in the same commit
- Bug fixes: write a test that fails before the fix and passes after
- Investigate event structures: `mise run lab:grep -- "pattern" --json --limit 5`

## Architecture

Layered architecture: CLI -> SDK -> Runtime -> (Engine + Index + Providers) -> (Core + Types).

**Dependency rules**: `types` has no internal deps. `core`, `index`, `engine` depend on `types` only. `providers` depends on `types` + `core`. `runtime` orchestrates all data layer crates. `sdk` wraps all. `cli` depends on `sdk` only.

**Data flow**: providers read raw logs -> normalize to `AgentEvent` -> index stores pointers in SQLite -> engine reconstructs sessions -> cli renders via TUI or JSON.

# CLAUDE.md

This file provides guidance for Claude Code when working in this repository.

## Required

- **Always load `/git-workflow` first** regardless of the conversation topic (before any other task)
- Before starting work -> `mise tasks` to see available tasks
- All commands MUST be run via `mise run` (works from any directory)
- Prefer root-cause fixes over shortcuts (step back, reflect, take the senior engineer approach)
- **All changes go through PRs via `/git-workflow`** (never commit directly to main)
- Commit messages must be one-line, Conventional Commits format
- Write everything in English (OSS project)

## Project Overview

Rust-based local-first observability platform for AI agents. Normalizes diverse agent log formats (Claude Code, Codex, Gemini) into a unified `AgentEvent` timeline.

```
./
├── crates/
│   ├── agtrace-types/       # Type definitions (no deps)
│   ├── agtrace-core/        # File paths, env, workspace utils
│   ├── agtrace-providers/   # Log parsers (claude/, codex/, gemini/)
│   ├── agtrace-index/       # SQLite pointer index
│   ├── agtrace-engine/      # Session assembly, token calc
│   ├── agtrace-runtime/     # Orchestration layer
│   ├── agtrace-sdk/         # Public SDK
│   ├── agtrace-cli/         # CLI (depends on sdk only)
│   └── agtrace-testing/     # Shared test utilities
└── docs/                    # Design documents
```

## Quick Reference

```bash
mise run verify              # fmt + clippy + test + build (full check)
mise run test                # run all tests
mise run clippy              # lint
mise run fmt                 # format code
mise run build:release       # release build
mise run git:status          # git workflow state + next action
mise tasks                   # list all tasks
```

## Skills

Detailed knowledge is separated into `.claude/skills/`. Loaded automatically or manually via `/skill-name`.

| Skill | Purpose |
|-------|---------|
| `/git-workflow` | Git workflow, branch management, PR creation |
| `/agtrace-provider-normalization` | Provider schema investigation and domain abstraction |

**Usage**: Type `/skill-name` in conversation, or auto-loaded when relevant.

## Project Rules

- Snapshot tests: `mise run snapshot:accept` to accept and review diff, then include in the same commit
- Bug fixes: write a test that fails before the fix and passes after
- Investigate event structures: `mise run lab:grep -- "pattern" --json --limit 5`

## Architecture

Layered architecture: CLI -> SDK -> Runtime -> (Engine + Index + Providers) -> (Core + Types).

**Dependency rules**: `types` has no internal deps. `core`, `index`, `engine` depend on `types` only. `providers` depends on `types` + `core`. `runtime` orchestrates all data layer crates. `sdk` wraps all. `cli` depends on `sdk` only.

**Data flow**: providers read raw logs -> normalize to `AgentEvent` -> index stores pointers in SQLite -> engine reconstructs sessions -> cli renders via TUI or JSON.
