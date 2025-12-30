<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>The Observability Platform for AI Agents.</strong></p>
  <p>Local-first OpenTelemetry for Claude, Codex, and Gemini.</p>

  [![npm version](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace.svg)](https://crates.io/crates/agtrace)
</div>

---

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

**agtrace** provides a unified timeline and analysis layer for fragmented AI agent logs.
Use the **CLI** for instant visualization, or build custom monitoring tools with the **SDK**.

## 🌟 Core Value

1. **Universal Normalization**: Converts diverse provider logs (Claude, Gemini, etc.) into a standard `AgentEvent` model.
2. **Schema-on-Read**: Resilient to provider updates. No database migrations needed.
3. **Local-First**: 100% offline. Privacy by design.

## 🚀 Quick Start (CLI)

The reference implementation of the agtrace platform.

```bash
npm install -g @lanegrid/agtrace
cd my-project
agtrace init      # once
agtrace watch     # live dashboard
```

## 🛠️ Building with the SDK

Embed agent observability into your own tools (vital-checkers, IDE plugins, dashboards).

```toml
[dependencies]
agtrace-sdk = "0.1.12"
```

```rust
use agtrace_sdk::{Client, Lens, analyze_session, assemble_session};

// Connect to the local workspace
let client = Client::connect("~/.agtrace")?;

// 1. Real-time Monitoring
let stream = client.watch().all_providers().start()?;
while let Some(event) = stream.next_blocking() {
    println!("Activity: {:?}", event);
}

// 2. Session Diagnosis
let events = client.session("session_123").events()?;
if let Some(session) = assemble_session(&events) {
    let report = analyze_session(session)
        .through(Lens::Failures)
        .through(Lens::Loops)
        .report()?;
    println!("Health: {}", report.score);
}
```

## 📚 Documentation

- [Why agtrace?](docs/motivation.md) - Understanding the problem and solution
- [Getting Started](docs/getting-started.md) - Detailed installation and usage guide
- [Architecture](docs/architecture.md) - Platform design and principles
- [SDK Documentation](crates/agtrace-sdk/README.md) - Building custom tools
- [Full Documentation](docs/README.md) - Commands, FAQs, and more

## 🔌 Supported Providers

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini** (Google)

## 📦 Architecture

- **Core SDK**: `agtrace-sdk`, `agtrace-engine`, `agtrace-providers`
- **Applications**: `agtrace-cli` (Reference Implementation)

## License

Dual-licensed under the MIT and Apache 2.0 licenses.
