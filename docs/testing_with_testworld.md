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

```rust
// Create a project directory and enter it
let world = TestWorld::new()
    .with_project("my-project")
    .enter_dir("my-project");

// The command will execute with CWD = temp_dir/my-project
```

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

### Background Processes

For testing long-running commands like `watch`, use `BackgroundProcess`:

```rust
use agtrace_testing::process::BackgroundProcess;
use std::io::{BufRead, BufReader};
use std::process::Command;

// Start watch in the background
let mut cmd = Command::cargo_bin("agtrace")?;
world.configure_command(&mut cmd)
    .arg("watch")
    .arg("--mode")
    .arg("console");

let mut proc = BackgroundProcess::spawn_piped(cmd)?;

// Read output line by line
if let Some(stdout) = proc.stdout() {
    let reader = BufReader::new(stdout);
    for line in reader.lines().take(10) {
        if let Ok(l) = line {
            if l.contains("Attached") {
                // Found what we're looking for
                break;
            }
        }
    }
}

// Clean up (or let Drop handle it)
proc.kill()?;
```

**Key methods:**
- `spawn_piped()` - Start process with stdout/stderr captured
- `stdout()` - Get mutable access to stdout for reading
- `stderr()` - Get mutable access to stderr
- `kill()` - Terminate the process
- `wait_timeout()` - Wait for process to exit

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

```rust
#[test]
fn test_project_detection_from_cwd() {
    let world = TestWorld::new()
        .with_project("project-a")
        .enter_dir("project-a");

    // Command will execute from project-a directory
    let result = world.run(&["session", "list"])?;
    // Should only see sessions from project-a
}
```

## Examples

See `crates/agtrace-cli/tests/testworld_example.rs` for complete examples.

## Next Steps

The next phase of testing improvements will include:

1. **RuntimeContext trait**: Abstract environment dependencies (CWD, time) for unit testing
2. **Watch logic extraction**: Move complex state management from handlers to runtime/ops
3. **Background process utilities**: Better support for long-running commands like `watch`

For more details, see the full test strategy document.
