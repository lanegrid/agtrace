# Testing with TestWorld Pattern

This document explains how to use the `agtrace-testing` crate and the `TestWorld` pattern for writing robust integration tests.

## Overview

The `agtrace-testing` crate provides a declarative, fluent interface for setting up test environments. It helps prevent common testing bugs by:

1. **Explicit CWD management**: Forces tests to explicitly declare the working directory
2. **Sample data isolation**: Provides utilities to create isolated test sessions with unique IDs
3. **Reusable assertions**: Custom assertions for common validation patterns
4. **Type-safe configuration**: Reduces copy-paste errors in test setup

## Quick Start

### Basic Example

```rust
use agtrace_testing::{assertions, TestWorld};

#[test]
fn test_session_list() {
    // Create isolated test environment
    let world = TestWorld::new();

    // Setup provider
    world
        .run(&[
            "provider",
            "set",
            "claude_code",
            "--log-root",
            world.log_root().to_str().unwrap(),
            "--enable",
        ])
        .expect("Failed to setup provider");

    // Copy sample data with isolation
    world
        .copy_sample_to_project_with_cwd(
            "claude_session.jsonl",
            "session1.jsonl",
            "/Users/test_user/project-a",
        )
        .expect("Failed to copy sample");

    // Index
    world
        .run(&["index", "update", "--all-projects"])
        .expect("Failed to index");

    // Query and verify
    let result = world
        .run(&["session", "list", "--format", "json"])
        .expect("Failed to list sessions");

    let json = result.json().expect("Failed to parse JSON");
    assertions::assert_session_count(&json, 1)
        .expect("Should have 1 session");
}
```

## Key Concepts

### TestWorld

`TestWorld` is the main entry point for creating test environments. It manages:

- Temporary directories (`.agtrace`, `.claude`)
- Working directory state
- Environment variables
- Sample file management

#### Creating a TestWorld

```rust
let world = TestWorld::new();
```

This creates a temporary directory structure:
```
temp_dir/
  .agtrace/     # Data directory (agtrace.db)
  .claude/      # Log root (sample files go here)
```

#### Working Directory Management

`TestWorld` provides three ways to manage the current working directory:

##### 1. `enter_dir()` - Builder Pattern (Immutable)

Use this when setting up the test environment:

```rust
let world = TestWorld::new()
    .with_project("my-project")
    .enter_dir("my-project");

// The command will execute with CWD = temp_dir/my-project
```

##### 2. `set_cwd()` - Mutable Changes

Use this when you need to change directories multiple times during a test:

```rust
let mut world = TestWorld::new()
    .with_project("project-a")
    .with_project("project-b");

// Move to project-a
world.set_cwd("project-a");
let result = world.run(&["session", "list"])?;

// Move to project-b
world.set_cwd("project-b");
let result = world.run(&["session", "list"])?;
```

##### 3. `run_in_dir()` - Temporary Context

Use this when you want to run a command in a specific directory without permanently changing the CWD:

```rust
let mut world = TestWorld::new()
    .with_project("project-a")
    .with_project("project-b");

// Run in project-a (temporarily)
let result_a = world.run_in_dir(&["session", "list"], "project-a")?;

// Run in project-b (cwd is preserved from before)
let result_b = world.run_in_dir(&["session", "list"], "project-b")?;

// Original cwd is still active
```

**When to use which:**
- `enter_dir()`: Initial setup in builder pattern
- `set_cwd()`: Explicitly testing "move to directory X" scenarios
- `run_in_dir()`: Testing the same command in multiple directories without state management

This is crucial for testing CWD-dependent logic like project detection.

### Sample File Management

The `TestWorld` provides three methods for copying sample files:

#### 1. `copy_sample` - Basic copy

```rust
world.copy_sample("claude_session.jsonl", "session.jsonl")?;
```

Copies to: `.claude/session.jsonl`

#### 2. `copy_sample_to_project` - Claude-encoded project directory

```rust
world.copy_sample_to_project(
    "claude_session.jsonl",
    "session.jsonl",
    "/Users/test/project-a",
)?;
```

Copies to: `.claude/-Users-test-project-a/session.jsonl`

#### 3. `copy_sample_to_project_with_cwd` - Isolated sessions (Recommended)

```rust
world.copy_sample_to_project_with_cwd(
    "claude_session.jsonl",
    "session.jsonl",
    "/Users/test/project-a",
)?;
```

This method:
1. Copies to: `.claude/-Users-test-project-a/session.jsonl`
2. Replaces `cwd` field with `/Users/test/project-a`
3. Generates a unique `sessionId` based on project dir + filename

**Why this matters:**
Sample files contain embedded `cwd` and `sessionId` fields. Without replacement, all tests would share the same session, causing false positives/negatives.

### Command Execution

There are two ways to execute commands in `TestWorld`:

#### 1. Using `run()` - Recommended for most cases

The `run()` method is a convenience wrapper that handles command creation and configuration:

```rust
// ✅ Simplest way - use run()
let result = world.run(&["session", "list", "--format", "json"])?;
assert!(result.success());

let json = result.json()?;
assertions::assert_session_count(&json, 1)?;
```

This automatically:
- Finds the `agtrace` binary (via `cargo_bin`)
- Configures `--data-dir`, `--format`, CWD, and environment variables
- Returns a `CliResult` with typed access to stdout/stderr

#### 2. Using `configure_command()` - For advanced cases

For more control (e.g., background processes), use `configure_command()`:

```rust
// ✅ Advanced: configure a custom command
let mut cmd = cargo_bin_cmd!("agtrace");
world.configure_command(&mut cmd)
    .arg("watch")
    .arg("--mode")
    .arg("console");

let output = cmd.output()?;
```

This automatically adds:
- `--data-dir <world.data_dir>`
- `--format plain`
- `current_dir(<world.cwd>)`
- Environment variables from `world.with_env(...)`

### Long-running commands (`watch`)

`watch` does not use the index or the configured log roots; it reads provider homes
directly. To test it end to end, point it at a writable copy of the synthetic fixture
workspace with `LiveFixture` (see [Synthetic fixtures](#synthetic-fixtures)) and the
`AGTRACE_CLAUDE_HOME` / `AGTRACE_CODEX_HOME` overrides. Then run it in console mode and read
stdout until the expected lines appear. `crates/agtrace-cli/tests/watch_command.rs` shows the
full pattern:

```rust
use agtrace_testing::live_fixture::{LiveFixture, PROJECT_ROOT};

let fx = LiveFixture::new(chrono::Local::now().date_naive())?; // Codex rollouts in "today"
let mut cmd = Command::cargo_bin("agtrace")?;
cmd.env("AGTRACE_CLAUDE_HOME", fx.claude_home())
    .env("AGTRACE_CODEX_HOME", fx.codex_home())
    .args(["--project", PROJECT_ROOT, "watch", "--mode", "console", "--since", "1d"]);
// spawn with piped stdout, read lines until e.g. "[T] audit-A" and "FINAL_ANSWER" appear, then kill
```

`--mode tui` refuses to start without a TTY. The TUI itself is tested without a terminal:
- presenter snapshots of the `WatchScreenVm`;
- ratatui `TestBackend` buffer snapshots (`crates/agtrace-cli/tests/watch_tui.rs`).

### Custom Assertions

The `assertions` module provides high-level validation:

```rust
use agtrace_testing::assertions;

// Assert session count
assertions::assert_session_count(&json, 2)?;

// Assert project count
assertions::assert_project_count(&json, 1)?;

// Assert all sessions belong to a project
assertions::assert_sessions_belong_to_project(&json, "abc123")?;

// Assert project list contains specific hashes
assertions::assert_projects_contain(&json, &["hash1", "hash2"])?;
```

## Migration Guide

### From `common::TestFixture` to `TestWorld`

**Old pattern:**
```rust
mod common;
use common::TestFixture;

#[test]
fn test_example() {
    let fixture = TestFixture::new();
    fixture.setup_provider("claude_code").unwrap();

    let mut cmd = fixture.command();
    cmd.arg("session").arg("list");
    // ...
}
```

**New pattern:**
```rust
use agtrace_testing::TestWorld;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn test_example() {
    let world = TestWorld::new();

    // Provider setup
    let mut cmd = cargo_bin_cmd!("agtrace");
    world.configure_command(&mut cmd)
        .arg("provider")
        .arg("set")
        .arg("claude_code")
        .arg("--log-root")
        .arg(world.log_root())
        .arg("--enable");
    cmd.output().expect("Failed to setup provider");

    // Use the world
    let mut cmd = cargo_bin_cmd!("agtrace");
    world.configure_command(&mut cmd)
        .arg("session")
        .arg("list");
    // ...
}
```

### Key Differences

1. **Command creation**: Use `cargo_bin_cmd!` in tests, not in `TestWorld`
2. **Provider setup**: Explicit command execution instead of builder method
3. **Sample files**: Use `copy_sample_to_project_with_cwd` for proper isolation

## Best Practices

### 1. Prefer `run()` over `configure_command()` when possible

```rust
// ✅ Simple and readable
let result = world.run(&["session", "list", "--format", "json"])?;
assert!(result.success());

// ❌ More verbose (only use for advanced cases like background processes)
let mut cmd = cargo_bin_cmd!("agtrace");
world.configure_command(&mut cmd)
    .arg("session")
    .arg("list")
    .arg("--format")
    .arg("json");
let output = cmd.output()?;
```

### 2. Always use `copy_sample_to_project_with_cwd` for session isolation

```rust
// ❌ Will cause session ID collisions
world.copy_sample_to_project("claude_session.jsonl", "s1.jsonl", "/proj/a")?;
world.copy_sample_to_project("claude_session.jsonl", "s2.jsonl", "/proj/b")?;
// Both sessions have the same sessionId!

// ✅ Each session gets a unique ID
world.copy_sample_to_project_with_cwd("claude_session.jsonl", "s1.jsonl", "/proj/a")?;
world.copy_sample_to_project_with_cwd("claude_session.jsonl", "s2.jsonl", "/proj/b")?;
```

### 3. Use custom assertions for readability

```rust
// ❌ Manual JSON parsing is verbose
let sessions = json["content"]["sessions"].as_array().unwrap();
assert_eq!(sessions.len(), 2);

// ✅ Custom assertion is clearer
assertions::assert_session_count(&json, 2)?;
```

### 4. Test CWD-dependent logic explicitly

Use `run_in_dir()` when testing the same logic across multiple directories:

```rust
#[test]
fn test_project_isolation_across_directories() {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    // Setup data for both projects
    world.copy_sample_to_project_with_cwd(..., "project-a")?;
    world.copy_sample_to_project_with_cwd(..., "project-b")?;
    world.run(&["index", "update", "--all-projects"])?;

    // Test project isolation without manual cwd management
    let result_a = world.run_in_dir(&["session", "list"], "project-a")?;
    let result_b = world.run_in_dir(&["session", "list"], "project-b")?;

    // Verify each directory only sees its own sessions
    assertions::assert_session_count(&result_a.json()?, 1)?;
    assertions::assert_session_count(&result_b.json()?, 1)?;
}
```

Or use `set_cwd()` when explicitly testing directory navigation:

```rust
#[test]
fn test_directory_navigation() {
    let mut world = TestWorld::new().with_project("project-a");

    // Test that changing to project directory works
    world.set_cwd("project-a");
    assert!(world.cwd().ends_with("project-a"));

    let result = world.run(&["session", "list"])?;
    // Should only see sessions from project-a
}
```

## Multi-Provider Testing

`TestWorld` provides first-class support for testing multiple providers (Claude Code, Codex).

### Provider Testing Goals

These tests verify that the CLI correctly:
1. Reads provider configuration from `config.toml`
2. Routes to the correct log directories
3. Selects the appropriate provider adapter
4. Aggregates data from multiple providers

### Using TestProvider

```rust
use agtrace_testing::providers::TestProvider;
use agtrace_testing::TestWorld;

#[test]
fn test_multi_provider_setup() -> anyhow::Result<()> {
    let world = TestWorld::new();

    // Enable multiple providers
    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Codex)?;

    // Add sessions from different providers
    world.add_session(TestProvider::Claude, "claude_work.jsonl")?;
    world.add_session(TestProvider::Codex, "rollout-codex-review.jsonl")?;

    // Index all providers
    world.run(&["index", "update", "--all-projects"])?;

    // Verify aggregation
    let result = world.run(&["session", "list", "--format", "json", "--all-projects"])?;
    let json = result.json()?;

    assertions::assert_session_count(&json, 2)?;

    Ok(())
}
```

### Provider-Specific Assertions

```rust
use agtrace_testing::assertions;

// Assert a specific session is from Claude
assertions::assert_session_provider(&json, 0, "claude_code")?;

// Assert all sessions are from Codex
assertions::assert_all_sessions_from_provider(&json, "codex")?;
```

### What These Tests Guarantee

1. **Configuration Routing**: `provider set` correctly updates `config.toml` and `session list` reads that configuration
2. **Provider Selection**: The indexer instantiates the correct `ProviderAdapter` based on file paths and config
3. **Data Aggregation**: Sessions from different providers are aggregated into a unified list
4. **Filtering**: The `--provider` option correctly filters by provider

## Synthetic fixtures

Test data is **synthetic**: no real user text, paths, e-mails, account ids, session ids,
tokens or encrypted blobs. The formats are copied field for field from real logs of the
supported versions (Claude Code ≥ 2.1.24x, Codex ≥ 0.153); the values are invented. Ids look
like `00000000-0000-4000-8000-00000000000N` (Claude) and `01900000-…` (Codex), the project is
`/work/demo-project`, and encrypted bodies are `gAAAA_SYNTHETIC_…`.

### The fixture tree: `crates/agtrace-testing/fixtures/v2026_09/`

```
claude/home/projects/-work-demo-project/
  00000000-…-000000000001.jsonl                        lead: spawns (teammate, subagent, fork), SendMessage,
                                                       teammate-message, task-notification, handback, compaction, …
  00000000-…-000000000001/subagents/agent-a0000000000000001.{jsonl,meta.json}   async subagent
  00000000-…-000000000001/subagents/agent-a0000000000000002.{jsonl,meta.json}   fork
  00000000-…-000000000002.jsonl                        teammate audit-A
claude/home/teams/session-00000001/config.json         team config (leadSessionId, members)
claude/home/sessions/4242.json                         session registry entry
codex/home/sessions/2026/09/20/rollout-…-01900000-…-000000000001.jsonl   root (spawn_agent, exec, compaction, …)
codex/home/sessions/2026/09/20/rollout-…-01900000-…-000000000002.jsonl   child /root/judge (thread_spawn)
codex/home/sessions/2026/09/20/rollout-…-01900000-…-000000000003.jsonl   fork (copied prefix)
codex/home/session_index.jsonl
```

It is the integration corpus: decoder snapshots, index tests, the SDK
`watch_workspace` test, and the `watch --mode console` CLI test all run on it. Keep every
file small (under 64 KiB).

### `LiveFixture`: a writable copy for watcher tests

`agtrace_testing::live_fixture::LiveFixture::new(day)` copies the tree into a temp directory:
Claude files under `<tmp>/claude` and the Codex rollouts under the given day in
`<tmp>/codex`. Pass today so the watcher's today/yesterday discovery sees them. It provides:
- paths: `claude_home()`, `codex_home()`, `lead_file()`, `teammate_file()`,
  `subagent_file(id)`, `codex_file(thread)`, `registry_file()`
- helpers that simulate live changes: `set_registry_pid(pid)` (make the registry entry live),
  `stash` / `restore` (make a file appear later), and `age_all(secs)` (age mtimes)

### `synth`: builders for unit tests

`agtrace_testing::synth` builds `AgentRef`s and `AgentEvent`s directly, for unit tests that
need precise variants without writing provider logs:

```rust
use agtrace_testing::synth::{AgentBuilder, EventLog};

let lead = AgentBuilder::claude_main("00000000-0000-4000-8000-000000000001").name("lead").build();
let mut log = EventLog::new(&lead.id);
log.at(10).user("review the parser");
log.at(12).bash("mise run test");
log.at(20).usage(42_000, Some("claude-opus-5-5"));
log.turn_end();
```

### Sanitization guard

`mise run fixtures:check` (`scripts/check-fixtures.sh`, part of `mise run verify`) fails on:
- absolute home paths (`/Users/…`, `/home/…`)
- e-mail addresses other than `@example.com` / `@example.org`
- `sk-` / `cse_` / `toolu_` values that are not marked synthetic
- fixture files larger than 64 KiB

## Examples

- `crates/agtrace-cli/tests/provider_filtering.rs` and `init_configuration.rs`: `TestWorld`
  with sample sessions
- `crates/agtrace-cli/tests/watch_command.rs`: `watch --mode console` on `LiveFixture`
- `crates/agtrace-cli/tests/watch_tui.rs`: TUI presenter and buffer snapshots
- `crates/agtrace-runtime/src/workspace/tests.rs`: watcher discovery and tailing on `LiveFixture`

## Next Steps

The next phase of testing improvements will include:

1. **RuntimeContext trait**: Abstract environment dependencies (CWD, time) for unit testing

For more details, see the full test strategy document.
