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
agtrace-sdk = "0.1.12"
```

## Usage

### Basic Example

```rust
use agtrace_sdk::{Client, Lens, analyze_session, assemble_session};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the workspace
    let client = Client::connect("~/.agtrace")?;

    // 2. Get a specific session
    let session_handle = client.session("session_id_123");
    let events = session_handle.events()?;

    // 3. Assemble and analyze the session
    if let Some(session) = assemble_session(&events) {
        let report = analyze_session(session)
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
    }

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
use agtrace_sdk::{Client, analyze_session, assemble_session, Lens};

fn check_session_health(session_id: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let client = Client::connect("~/.agtrace")?;
    let events = client.session(session_id).events()?;

    if let Some(session) = assemble_session(&events) {
        let report = analyze_session(session)
            .through(Lens::Failures)
            .through(Lens::Loops)
            .report()?;

        Ok(report.score)
    } else {
        Ok(0)
    }
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

### Client

- `Client::connect(path)`: Connect to an agtrace workspace
- `.watch()`: Create a watch builder for real-time monitoring
- `.session(id)`: Get a handle to a specific session

### SessionHandle

- `.events()`: Get all normalized events for the session
- `.summary()`: Get a summary of the session

### WatchBuilder

- `.provider(name)`: Filter events by provider
- `.all_providers()`: Watch all configured providers
- `.start()`: Start monitoring and return a live stream

### LiveStream

Implements the `Iterator` trait for ergonomic event processing.

- `.next_blocking()`: Block until next event arrives
- `.try_next()`: Try to get next event without blocking

### Analysis

- `analyze_session(session)`: Create an analyzer for a session
- `.through(lens)`: Apply a diagnostic lens
- `.report()`: Generate analysis report

## Contributing

Contributions are welcome! Please see the main [agtrace repository](https://github.com/lanegrid/agtrace) for contribution guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
