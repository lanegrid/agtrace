# agtrace-sdk

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
agtrace-sdk = "0.6"
```

<!-- cargo-rdme start -->

agtrace-sdk: SDK for AI agent observability.

**Note**: README.md is auto-generated from this rustdoc using `cargo-rdme`.
To update: `cargo rdme --workspace-project agtrace-sdk`

### Overview

`agtrace-sdk` provides a high-level, stable API for building tools on top of agtrace.
It powers agtrace's MCP server (letting agents query their execution history) and CLI tools,
and can be embedded in your own applications. The SDK normalizes logs from multiple providers
(Claude Code, Codex, Gemini) into a unified data model, enabling cross-provider analysis.

### Quickstart

```rust
use agtrace_sdk::{Client, types::SessionFilter};

// Connect to the local workspace
let client = Client::connect_default().await?;

// List sessions and browse the most recent one
let sessions = client.sessions().list(SessionFilter::all())?;
if let Some(summary) = sessions.first() {
    let handle = client.sessions().get(&summary.id)?;
    let session = handle.assemble()?;

    println!("Session: {} turns, {} tokens",
        session.turns.len(),
        session.stats.total_tokens);

    // Browse tool calls
    for turn in &session.turns {
        for step in &turn.steps {
            for tool in &step.tools {
                println!("  {} ({})",
                    tool.call.content.name(),
                    if tool.is_error { "failed" } else { "ok" });
            }
        }
    }
}
```

For complete examples, see the [`examples/`](https://github.com/lanegrid/agtrace/tree/main/crates/agtrace-sdk/examples) directory.

### Architecture

This SDK acts as a facade over:
- `agtrace-types`: Core domain models (AgentEvent, etc.)
- `agtrace-providers`: Multi-provider log normalization
- `agtrace-engine`: Session assembly and analysis
- `agtrace-index`: Metadata storage and querying
- `agtrace-runtime`: Internal orchestration layer

### Usage Patterns

#### Session Browsing

Access structured session data (Turn → Step → Tool hierarchy):

```rust
use agtrace_sdk::{Client, types::SessionFilter};

let client = Client::connect_default().await?;
let sessions = client.sessions().list(SessionFilter::all())?;

for summary in sessions.iter().take(5) {
    let handle = client.sessions().get(&summary.id)?;
    let session = handle.assemble()?;
    println!("{}: {} turns, {} tokens",
        summary.id,
        session.turns.len(),
        session.stats.total_tokens);
}
```

#### Real-time Monitoring

Watch for events as they happen:

```rust
use agtrace_sdk::Client;
use futures::stream::StreamExt;

let client = Client::connect_default().await?;
let mut stream = client.watch().all_providers().start()?;
while let Some(event) = stream.next().await {
    println!("Event: {:?}", event);
}
```

#### Diagnostics

Run diagnostic checks on sessions:

```rust
use agtrace_sdk::{Client, Diagnostic, types::SessionFilter};

let client = Client::connect_default().await?;
let sessions = client.sessions().list(SessionFilter::all())?;
if let Some(summary) = sessions.first() {
    let handle = client.sessions().get(&summary.id)?;
    let report = handle.analyze()?
        .check(Diagnostic::Failures)
        .check(Diagnostic::Loops)
        .report()?;

    println!("Health: {}/100", report.score);
    for insight in &report.insights {
        println!("  Turn {}: {}", insight.turn_index + 1, insight.message);
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
