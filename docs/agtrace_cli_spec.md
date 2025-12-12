# agtrace Design Rationale

## Why This Tool Exists

Agent behavior logs from Claude Code, Codex, and Gemini CLI contain valuable debugging information, but:
1. Each provider uses different schemas and file structures
2. Logs are scattered across multiple directories
3. No unified way to search, compare, or analyze sessions across providers
4. Understanding long sessions requires manual log diving

`agtrace` solves this by creating a lightweight, provider-agnostic index and analysis layer.

## Core Design Principles

### 1. Pointer-Based Architecture

**Decision:** Store metadata pointers in SQLite, never copy or convert raw logs.

**Why:**
- Provider schemas evolve frequently (Codex v0.53 → v0.63 had 3 breaking changes)
- Copying creates sync issues and storage bloat
- Original logs are the source of truth - preserve them
- Failed parses can be retried later without data loss

**Trade-off:** Reading sessions requires on-demand parsing (slightly slower), but enables fail-safe evolution.

### 2. Schema-on-Read

**Decision:** Normalize provider logs to `AgentEventV1` at read-time, not write-time.

**Why:**
- Provider schemas change without notice
- Can improve normalization logic without re-indexing
- Parsing errors don't block indexing - file is registered, parsing retried on view
- Enables "diagnose → fix schema → re-read" workflow without re-scanning

**Trade-off:** No pre-computed event statistics, but diagnostics become trivial.

### 3. Project Isolation by Exact Match

**Decision:** Projects are identified by exact `project_root` path or hash. Subdirectories are separate projects.

**Why:**
- Gemini uses `sha256(project_root)` - different hash for each directory level
- Path-based hierarchy (treating `/project/subdir` as child of `/project`) would be inconsistent across providers
- Simpler mental model: one directory = one project
- Avoids ambiguity about which sessions belong where

**Trade-off:** Can't view parent + child sessions together (use `--all-projects` if needed).

### 4. Fail-Safe Indexing

**Decision:** If a log file fails to parse, register it anyway with minimal metadata.

**Why:**
- Schema updates shouldn't break existing indexes
- User can run `doctor` to identify issues
- Parsing can be retried after schema fixes
- Never lose track of a session due to temporary schema incompatibility

**Trade-off:** Index may contain unparseable sessions, but `doctor` makes debugging explicit.

## Core Concepts

- **Provider:** Source of logs (Claude, Codex, Gemini). Each has different schemas and file structures.
- **Project:** A source code repository identified by `sha256(project_root)`. Sessions are grouped by project.
- **Session:** A logical unit of work (conversation). The primary browsing/analysis unit.

## Command Organization

**Why namespaces?** Flat command structures become unwieldy past ~10 commands. Namespaces (`session`, `provider`, `doctor`, `lab`) group related operations and improve `--help` discoverability.

For command details: `agtrace <command> --help`

## Compact View Philosophy

**Decision:** `session show --style compact` collapses tool chains into single lines, showing inputs (not outputs) and duration.

**Why:**
- Long sessions (100+ events) are hard to scan in timeline mode
- Bottlenecks and loops become immediately visible via duration highlights
- Shows *what was asked* (command, pattern, file), not results - preserves execution sequence while drastically reducing visual noise
- User interprets intent from facts: `Edit(schema.rs x4)` could be deliberate iteration or being stuck

**Trade-off:** Less readable for detailed debugging, but enables quick pattern recognition across 200+ event sessions.

## Related Documentation

- Database schema: `database_schema.md`
- Provider schemas: `agent_event_schema_v1.md`
- Schema debugging: `troubleshooting_schema_issues.md`
