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

The repository is a Rust Workspace organized into modular crates, following a distinct Layered Architecture (Presentation → Runtime → Core Logic → Data Access).

### Directory Structure

#### 1. Interface Layer

* `crates/agtrace-cli/`
* Role: The main entry point for the user.
* Key Files:
* `args.rs`: Defines the CLI structure (subcommands: `session`, `index`, `doctor`, `lab`, `watch`, etc.) using `clap`.
* `commands.rs` & `handlers/`: Dispatches CLI commands to the runtime layer.
* `presentation/`: Handles TUI (Terminal User Interface) and console output formatting.




* `crates/agtrace-debug/`
* Role: A standalone developer tool.
* Key Files: `main.rs`.
* Function: specialized utility to watch and debug raw event streams from providers in real-time.



#### 2. Orchestration Layer

* `crates/agtrace-runtime/`
* Role: The "glue" that binds components together.
* Key Files:
* `init.rs`: Handles the `agtrace init` workflow (DB creation, config detection).
* `config.rs`: Manages `config.toml` (provider settings).
* `client.rs` (exported facade): Public API for workspace operations.


* Function: Manages the application lifecycle, configuration loading, and high-level operations.



#### 3. Core Logic Layer

* `crates/agtrace-engine/`
* Role: The "brain" of the application.
* Key Files:
* `session.rs`: Reconstructs linear conversation history (`turns`, `steps`) from raw event streams.
* `state_updates.rs`: Tracks context window usage and state changes.
* `export.rs`: Handles data export strategies (Raw vs Clean vs Reasoning).


* Function: Processes normalized events into meaningful session insights and statistics.



#### 4. Data & Ingestion Layer

* `crates/agtrace-index/`
* Role: Metadata storage (SQLite).
* Key Files: `db.rs`.
* Function: Maintains a lightweight SQLite pointer database (`agtrace.db`) to track projects, sessions, and files without duplicating raw log content (Schema-on-Read approach).


* `crates/agtrace-providers/`
* Role: Data normalization adapters.
* Key Files:
* `traits.rs`: Defines generic `LogDiscovery`, `SessionParser`, and `ToolMapper` interfaces.
* `normalization.rs`: Maps provider-specific JSON to standard domain models.
* `claude/`, `codex/`, `gemini/`: Specific implementations for different AI providers.


* Function: Reads raw logs from specific providers and converts them into standardized `AgentEvent`s.



#### 5. Shared Domain Layer

* `crates/agtrace-types/`
* Role: Common type definitions.
* Key Files:
* `models.rs`: Defines the core `AgentEvent`, `EventPayload` (User, ToolCall, Reasoning), and `StreamId`.
* `util.rs`: Hashing and path utility helpers.


* Function: The shared "language" used by all other crates to ensure type safety across the system.

### Data Flow Summary

1. Read: `agtrace-providers` reads raw log files from disk.
2. Normalize: It converts them into `AgentEvent`s (defined in `types`).
3. Index: `agtrace-index` stores metadata about these sessions in SQLite.
4. Process: `agtrace-engine` consumes events to calculate tokens, reconstruct session flow, and analyze behavior.
5. Display: `agtrace-cli` presents the processed data to the user via TUI or JSON output.
