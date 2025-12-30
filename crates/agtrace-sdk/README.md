# agtrace-sdk

**The Observability Platform for AI Agents (Public SDK)**

`agtrace-sdk` provides a high-level, stable API for building observability tools on top of agtrace. It abstracts away the internal complexity of providers, indexing, and runtime orchestration, exposing only the essential primitives for monitoring and analyzing AI agent behavior.

## Features

- **Universal Normalization**: Works with multiple AI agent providers (Claude, Codex, Gemini)
- **Real-time Monitoring**: Watch for live events as agents interact
- **Session Analysis**: Analyze agent sessions with built-in diagnostic lenses
- **Clean API**: Simple, type-safe interface for common observability tasks

## Architecture

This SDK acts as a facade over:
- `agtrace-types`: Core domain models (AgentEvent, etc.)
- `agtrace-providers`: Multi-provider log normalization
- `agtrace-engine`: Session analysis and diagnostics
- `agtrace-index`: Metadata storage and querying
- `agtrace-runtime`: Internal orchestration layer

### Stability Guarantee

- **agtrace-sdk**: Semantic Versioning (SemVer) strictly followed. Public API is stable.
- **Internal crates** (`agtrace-runtime`, `agtrace-engine`, etc.): Internal APIs, subject to breaking changes without notice.

If you're building tools on top of agtrace, use only the `agtrace-sdk` crate to ensure stability across updates.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
agtrace-sdk = "0.1.13"
```

## Usage

### Client-based API (Recommended)

For most use cases, use the Client-based API which provides stateful operations:

```rust
use agtrace_sdk::{Client, Lens};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the workspace
    let client = Client::connect("~/.agtrace")?;

    // 2. Get a specific session
    let session_handle = client.sessions().get("session_id_123")?;

    // 3. Analyze the session
    let report = session_handle.analyze()?
        .through(Lens::Failures)
        .through(Lens::Loops)
        .through(Lens::Bottlenecks)
        .report()?;

    println!("Health score: {}", report.score);
    for insight in &report.insights {
        println!("  Turn {}: {:?} - {}",
            insight.turn_index + 1,
            insight.severity,
            insight.message);
    }

    Ok(())
}
```

### Standalone API (for testing/simulations)

For scenarios where you don't need a Client (testing, simulations, custom pipelines):

```rust
use agtrace_sdk::{SessionHandle, types::AgentEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // When you have raw events without Client
    let events: Vec<AgentEvent> = vec![/* ... */];
    let handle = SessionHandle::from_events(events);

    let session = handle.assemble()?;
    println!("Session has {} turns", session.turns.len());

    Ok(())
}
```

### Real-time Monitoring

```rust
use agtrace_sdk::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;

    // Watch for live events from all providers
    let stream = client.watch().all_providers().start()?;

    // Use the Iterator trait for ergonomic event processing
    for event in stream {
        println!("New event: {:?}", event);
    }

    Ok(())
}
```

### Provider-specific Monitoring

```rust
use agtrace_sdk::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;

    // Watch only Claude events
    let stream = client
        .watch()
        .provider("claude")
        .start()?;

    for event in stream {
        println!("Claude event: {:?}", event);
    }

    Ok(())
}
```

## Diagnostic Lenses

The SDK provides several built-in lenses for analyzing agent behavior:

- **Failures**: Detects tool execution failures
- **Loops**: Detects repetitive patterns that might indicate infinite loops
- **Bottlenecks**: Identifies slow tool executions (>10 seconds)

## Use Cases

### Building a Monitoring Dashboard

```rust
use agtrace_sdk::{Client, Lens};

fn check_session_health(session_id: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;
    let session_handle = client.sessions().get(session_id)?;

    let report = session_handle.analyze()?
        .through(Lens::Failures)
        .through(Lens::Loops)
        .report()?;

    Ok(report.score)
}
```

### Dead Man's Switch (vital-checker)

```rust
use agtrace_sdk::Client;
use std::time::{Duration, Instant};

fn monitor_activity() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;
    let stream = client.watch().all_providers().start()?;

    let mut last_activity = Instant::now();
    let timeout = Duration::from_secs(180); // 3 minutes

    loop {
        if let Some(_event) = stream.try_next() {
            last_activity = Instant::now();
        }

        if last_activity.elapsed() > timeout {
            println!("WARNING: No agent activity for 3 minutes!");
            // Send alert...
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
```

## API Reference

### Client-based API (Recommended)

The primary way to interact with agtrace is through the Client-based API, which manages state (database connections, configuration) for you.

#### Client

- `Client::connect(path)`: Connect to an agtrace workspace
- `.sessions()`: Access session operations
- `.watch()`: Create a watch builder for real-time monitoring
- `.projects()`: Access project operations
- `.insights()`: Access insights/analysis operations
- `.system()`: Access system operations

#### SessionClient

- `.list(filter)`: List sessions with optional filtering
- `.get(id)`: Get a handle to a specific session

#### SessionHandle

- `SessionHandle::from_events(events)`: Create a handle from raw events (standalone use)
- `.events()`: Get all normalized events for the session
- `.assemble()`: Assemble events into a structured session
- `.summarize()`: Get a summary of the session
- `.analyze()`: Analyze session with diagnostic lenses
- `.export(strategy)`: Export session with specified strategy

### WatchBuilder

- `.provider(name)`: Filter events by provider
- `.all_providers()`: Watch all configured providers
- `.start()`: Start monitoring and return a live stream

### LiveStream

Implements the `Iterator` trait for ergonomic event processing.

- `.next_blocking()`: Block until next event arrives
- `.try_next()`: Try to get next event without blocking

#### SessionAnalyzer

Created by calling `session_handle.analyze()`:

- `.through(lens)`: Apply a diagnostic lens
- `.report()`: Generate analysis report

### Low-level Utilities (Power User API)

For advanced use cases like building custom TUIs or implementing custom event processing logic, the SDK exposes stateless utility functions through the `utils` module.

#### Event Processing

- `utils::extract_state_updates(&event)`: Extract state changes (tokens, turns, model) from a single event

**Example:**

```rust
use agtrace_sdk::{Client, utils};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;
    let stream = client.watch().all_providers().start()?;

    for event in stream.take(10) {
        let updates = utils::extract_state_updates(&event);
        if updates.is_new_turn {
            println!("New turn started!");
        }
        if let Some(usage) = updates.usage {
            println!("Token usage: {:?}", usage);
        }
    }
    Ok(())
}
```

#### Project Management

- `utils::discover_project_root(explicit_root)`: Discover project root from flag/env/cwd
- `utils::project_hash_from_root(path)`: Compute canonical project hash
- `utils::resolve_effective_project_hash(hash, all_flag)`: Resolve project scope for commands

**Example:**

```rust
use agtrace_sdk::utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = utils::discover_project_root(None)?;
    let hash = utils::project_hash_from_root(&project_root.to_string_lossy());
    println!("Project hash: {}", hash);
    Ok(())
}
```

## Contributing

Contributions are welcome! Please see the main [agtrace repository](https://github.com/lanegrid/agtrace) for contribution guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
