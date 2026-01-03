# agtrace-sdk

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
agtrace-sdk = "0.4"
```

<!-- cargo-rdme start -->

agtrace-sdk: SDK for AI agent observability and memory.

**Note**: README.md is auto-generated from this rustdoc using `cargo-rdme`.
To update: `cargo rdme --workspace-project agtrace-sdk`

### Overview

`agtrace-sdk` provides a high-level, stable API for building tools on top of agtrace.
It powers agtrace's MCP server (giving agents memory) and CLI tools (for debugging),
and can be embedded in your own applications. It abstracts away the internal complexity
of providers, indexing, and runtime orchestration, exposing only the essential
primitives for monitoring and analyzing AI agent behavior.

### Quickstart

```rust
use agtrace_sdk::{Client, Lens, types::SessionFilter};

// Connect to the local workspace (uses system data directory)
let client = Client::connect_default().await?;

// List sessions and analyze the most recent one
let sessions = client.sessions().list(SessionFilter::all())?;
if let Some(summary) = sessions.first() {
    let handle = client.sessions().get(&summary.id)?;
    let report = handle.analyze()?
        .through(Lens::Failures)
        .report()?;
    println!("Health: {}/100", report.score);
}
```

For complete examples, see the [`examples/`](https://github.com/lanegrid/agtrace/tree/main/crates/agtrace-sdk/examples) directory.

### Architecture

This SDK acts as a facade over:
- `agtrace-types`: Core domain models (AgentEvent, etc.)
- `agtrace-providers`: Multi-provider log normalization
- `agtrace-engine`: Session analysis and diagnostics
- `agtrace-index`: Metadata storage and querying
- `agtrace-runtime`: Internal orchestration layer

### Usage Patterns

#### Real-time Monitoring

```rust
use agtrace_sdk::Client;
use futures::stream::StreamExt;

let client = Client::connect_default().await?;
let mut stream = client.watch().all_providers().start()?;
let mut count = 0;
while let Some(event) = stream.next().await {
    println!("New event: {:?}", event);
    count += 1;
    if count >= 10 { break; }
}
```

#### Session Analysis

```rust
use agtrace_sdk::{Client, Lens, types::SessionFilter};

let client = Client::connect_default().await?;
let sessions = client.sessions().list(SessionFilter::all())?;
if let Some(summary) = sessions.first() {
    let handle = client.sessions().get(&summary.id)?;
    let report = handle.analyze()?
        .through(Lens::Failures)
        .through(Lens::Loops)
        .report()?;

    println!("Health score: {}", report.score);
    for insight in &report.insights {
        println!("Turn {}: {:?} - {}",
            insight.turn_index + 1,
            insight.severity,
            insight.message);
    }
}
```

#### Standalone API (for testing/simulations)

```rust
use agtrace_sdk::{SessionHandle, types::AgentEvent};

// When you have raw events without Client (e.g., testing, simulations)
let events: Vec<AgentEvent> = vec![/* ... */];
let handle = SessionHandle::from_events(events);

let session = handle.assemble()?;
println!("Session has {} turns", session.turns.len());
```

<!-- cargo-rdme end -->

## Contributing

Contributions are welcome! Please see the main [agtrace repository](https://github.com/lanegrid/agtrace) for contribution guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
