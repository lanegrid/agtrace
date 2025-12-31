## Project Summary

Based on the codebase analysis, here are the project goals and non-goals for agtrace:

### Project Goals

* Universal Normalization: Unify diverse agent log formats (Claude, Codex, Gemini) into a standardized, type-safe `AgentEvent` timeline for consistent analysis.
* Fail-Safe Observability: Employ a "Schema-on-Read" architecture where raw logs are the source of truth, ensuring parsing errors or schema updates never cause data loss.
* Zero-Copy Indexing: maintain a lightweight SQLite pointer index that references original log files rather than duplicating content, minimizing storage bloat.
* Deep Diagnostics: Provide high-fidelity debugging tools (`doctor`, `lab`, `watch`) to inspect raw payloads, token usage, and complex reasoning chains without abstraction layers hiding details.
* Project Isolation: Enforce strict, hash-based project separation to ensure reliable session grouping across different providers' filesystem conventions.

### Non-Goals

* Real-Time Interception: It is not a proxy or middleware that sits between the user and the agent; it analyzes logs post-write (or via tailing).
* Schema-on-Write: It deliberately avoids normalizing data at ingestion time to prevent "baking in" parsing logic that might become obsolete.
* Hierarchical Organization: It does not support nested project structures (e.g., parent/child directories), opting for flat, exact-match isolation to avoid inconsistency.
* Centralized Storage: It does not aim to be a monolithic data store; the database is disposable and rebuildable from the raw log files at any time.

## Project Rules

- Keep minimal comments and documents.
- Write comments in English.
- Read `docs`.
- When you make a commit, the commit message must be oneline not multiline.
- Rather than rushing to complete tasks, please focus on a quality-driven approach: reviewing implementations, running lint and fmt checks, and committing with concise, one-line messages (messages like "Claude's co-author" are unnecessary—keep them one-line).
- Rules for snapshot tests: After running `cargo insta accept`, use `git diff` to check the differences. If there are issues, fix the implementation. If there are no issues, include it in the same commit as the implementation.
- Use `tree2md` command for full file tree.
- Design principle: Always choose the complete, unified solution over partial fixes. Never offer half-measures like "delete unused code" or "suppress warnings" without fixing the root cause. When facing implementation choices, default to the option that improves consistency and type safety across the codebase.
- Leave a TODO when you are consciously deferring a specific, necessary action due to immediate constraints like scope or dependencies.
- When investigating actual event/tool structures: Run `cargo build --release && ./target/release/agtrace lab grep "pattern" --json --limit 5` to see real data instead of reading raw files. `./target/release/agtrace lab grep -h` helps to learn how to use it.

### Test-Driven Bug Fixes

When fixing bugs, ensure tests actually validate the fix:

**Core Principle**: Tests should fail before the fix and pass after the fix.

**Recommended Approach**:
1. Write a test that reproduces the bug (should fail on current main)
2. Verify the test actually fails without your fix
3. Implement the fix
4. Verify the test now passes

**Commit Strategy** (choose based on context):
- **Separate commits** (preferred for complex bugs):
  - First commit: `test: add failing test for issue #N (documents bug behavior)`
  - Second commit: `fix: resolve issue #N (description)`
  - Keep both commits in the same PR for easier review
- **Single commit** (acceptable for simple fixes):
  - Include both test and fix in one commit
  - Ensure test would fail without the fix

**Verification**:
- Before finalizing: Temporarily revert your implementation changes and confirm tests fail
- This proves your test actually validates the fix
- If tests pass without your fix, the test isn't testing the right thing

**Anti-patterns to avoid**:
- Writing tests that always pass (even before the fix)
- Implementing fix without any test coverage
- Tests that check indirect effects instead of the actual bug

**Example** (issue #5):
- Test commit: Documents that `init` reports `session_count: 0` before indexing (current bug)
- Fix commit: Modifies `InitService::run()` to index before counting + updates test to assert `session_count: 1`

## Overview of agtrace

A Rust Workspace following layered architecture: CLI → SDK → Runtime → (Engine + Index + Providers) → (Core + Types).

### Crate Architecture & Design Principles

**1. Foundation Layer**

* `agtrace-types` (`crates/agtrace-types/`)
  - **Principle**: Minimal logic. Only stable type definitions and schemas that rarely change.
  - Dependencies: None

* `agtrace-core` (`crates/agtrace-core/`)
  - **Principle**: Handles file paths, environment variables, and workspace utilities.
  - Dependencies: `types`

* `agtrace-testing` (`crates/agtrace-testing/`)
  - **Principle**: Shared test utilities for `sdk` and `cli`.
  - Dependencies: Internal crates as needed

**2. Data Layer**

* `agtrace-engine` (`crates/agtrace-engine/`)
  - **Principle**: Provider-agnostic and environment-agnostic domain logic accumulation.
  - Dependencies: `types` only

* `agtrace-providers` (`crates/agtrace-providers/`)
  - **Principle**: Aggregates all provider-specific implementations (`claude/`, `codex/`, `gemini/`).
  - Dependencies: `types`, `core`

* `agtrace-index` (`crates/agtrace-index/`)
  - **Principle**: Provider-agnostic. Handles only local SQLite DB operations (zero-copy, pointer-based).
  - Dependencies: `types`

**3. Orchestration Layer**

* `agtrace-runtime` (`crates/agtrace-runtime/`)
  - **Principle**: Orchestrates `engine`, `index`, and `providers` without business logic.
  - Dependencies: `types`, `core`, `providers`, `index`, `engine`

**4. Public API Layer**

* `agtrace-sdk` (`crates/agtrace-sdk/`)
  - **Principle**: Public SDK for external observability tools.
  - Dependencies: All internal crates

**5. Presentation Layer**

* `agtrace-cli` (`crates/agtrace-cli/`)
  - **Principle**: Depends only on `sdk`. Acts as a sophisticated example of SDK usage.
  - Dependencies: `sdk` only

### Dependency Rules

`types` has no internal dependencies. `core`, `index`, and `engine` depend only on `types`. `providers` depends on `types` and `core`. `runtime` orchestrates all data layer crates (`engine`, `index`, `providers`) plus `types` and `core`. `sdk` wraps all internal crates. `cli` depends only on `sdk`.

### Data Flow

1. **Read**: `providers` discover and read raw log files
2. **Normalize**: Convert to `AgentEvent` (Schema-on-Read)
3. **Index**: `index` stores metadata pointers in SQLite
4. **Analyze**: `engine` reconstructs sessions, calculates tokens
5. **Present**: `cli` renders via TUI or JSON
