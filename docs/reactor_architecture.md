# Reactor Architecture

## Overview

The reactor architecture is an event-driven plugin system for the `watch` command that enables extensible monitoring and intervention capabilities. This design transforms agtrace from a passive log viewer into an active circuit breaker.

## Core Concepts

### Event-Driven Flow

```
AgentEvent (from SessionWatcher)
    ↓
SessionState (updated)
    ↓
ReactorContext (event + state)
    ↓
Reactors (process in order)
    ↓
Reactions (Continue, Warn, Intervene)
    ↓
Main Loop (handles reactions)
```

### Key Components

#### 1. Reactor Trait

All reactors implement the `Reactor` trait:

```rust
pub trait Reactor: Send {
    fn name(&self) -> &str;
    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction>;
}
```

#### 2. ReactorContext

The context passed to each reactor contains:
- **Event** (`&AgentEvent`): The current event trigger
- **State** (`&SessionState`): Session metadata snapshot

This dual-context design enables reactors to:
- React to immediate events (e.g., tool call detected)
- Consider historical context (e.g., 5th consecutive error)

#### 3. Reactions

Reactors return one of three reactions:

- **`Continue`**: Normal operation, proceed to next reactor
- **`Warn(String)`**: Log a warning message
- **`Intervene { reason, severity }`**: Request intervention
  - `Severity::Notification`: Alert user (desktop notification)
  - `Severity::Kill`: Terminate agent process (v0.2.0)

#### 4. SessionState

Lightweight metadata tracking:
- Session ID
- Token consumption (input/output)
- Error count
- Turn count
- Last activity timestamp

## Built-in Reactors

### TuiRenderer

**Purpose**: Display events to stdout

**Behavior**:
- Formats events with icons and colors
- Truncates long text
- Categorizes tools (read/write/exec)
- Always returns `Continue`

### StallDetector

**Purpose**: Detect when agent is idle (waiting for input)

**Configuration**:
- Idle threshold: 60 seconds
- Notification cooldown: 5 minutes

**Behavior**:
- Tracks time since last activity
- Emits `Intervene { Notification }` if idle
- Useful for "go to bathroom" notifications

### SafetyGuard

**Purpose**: Detect potentially dangerous operations

**Detections**:
- Path traversal (`..`)
- System directory access (`/etc/`, `/sys/`, `/`)
- Absolute paths outside user directories

**Configuration**:
- v0.1.0: `kill_on_danger = false` (monitoring only)
- v0.2.0: `kill_on_danger = true` (automatic termination)

## Creating a Custom Reactor

### Example: Token Budget Reactor

```rust
use crate::reactor::{Reaction, Reactor, ReactorContext, Severity};
use anyhow::Result;

pub struct TokenBudgetReactor {
    max_tokens: i32,
}

impl TokenBudgetReactor {
    pub fn new(max_tokens: i32) -> Self {
        Self { max_tokens }
    }
}

impl Reactor for TokenBudgetReactor {
    fn name(&self) -> &str {
        "TokenBudgetReactor"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        let total = ctx.state.total_input_tokens + ctx.state.total_output_tokens;

        if total > self.max_tokens {
            return Ok(Reaction::Intervene {
                reason: format!(
                    "Token budget exceeded: {} / {}",
                    total, self.max_tokens
                ),
                severity: Severity::Notification,
            });
        }

        Ok(Reaction::Continue)
    }
}
```

### Registration

Add to `handlers/watch.rs`:

```rust
let mut reactors: Vec<Box<dyn Reactor>> = vec![
    Box::new(TuiRenderer::new()),
    Box::new(StallDetector::new(60)),
    Box::new(SafetyGuard::new()),
    Box::new(TokenBudgetReactor::new(100_000)), // Add custom reactor
];
```

## Design Principles

### 1. Separation of Concerns

Each reactor has a single responsibility:
- TuiRenderer → Display only
- StallDetector → Time monitoring
- SafetyGuard → Security checks

### 2. Context over State

Reactors receive both:
- **Trigger** (current event): "What just happened?"
- **Background** (session state): "What's the overall situation?"

This enables smart decisions like:
- "Is this the 5th consecutive error?" (needs state)
- "Is this tool call dangerous?" (needs event)

### 3. Progressive Enhancement

v0.1.0 (current):
- Reactors monitor and warn
- No process control

v0.2.0 (future):
- Add `agtrace run -- <command>`
- Reactors can kill child process
- Same architecture, new capabilities

### 4. Testability

Reactors are:
- Isolated (single responsibility)
- Stateless (context is passed in)
- Mockable (trait-based)

See `reactor.rs` and `safety_guard.rs` tests for examples.

## Future Extensions

### v0.2.0: Process Control

```rust
// In handlers/watch.rs
fn handle_reaction(reaction: Reaction, child: Option<&Child>) -> Result<()> {
    match reaction {
        Reaction::Intervene { severity: Severity::Kill, .. } => {
            if let Some(child) = child {
                child.kill()?; // Terminate agent process
            }
        }
        _ => {}
    }
    Ok(())
}
```

### Potential Reactors

- **LoopDetector**: Detect repeated tool calls (stuck in loop)
- **CostOptimizer**: Suggest cheaper alternatives (e.g., use grep instead of reading entire file)
- **AuditLogger**: Record all events to audit log
- **MetricsCollector**: Send telemetry to observability platform
- **AIReviewer**: Use LLM to analyze session quality

## Testing Strategy

### Unit Tests

Test each reactor in isolation:

```rust
#[test]
fn test_safety_guard_detects_path_traversal() {
    let mut guard = SafetyGuard::new();
    let event = create_tool_call_event(json!({"path": "../../etc/passwd"}));
    let state = SessionState::new(...);
    let ctx = ReactorContext { event: &event, state: &state };

    let result = guard.handle(ctx).unwrap();
    assert!(matches!(result, Reaction::Intervene { .. }));
}
```

### Integration Tests

Test reactor chain coordination (TODO).

## References

- Core traits: `crates/agtrace-cli/src/reactor.rs`
- Built-in reactors: `crates/agtrace-cli/src/reactors/`
- Main loop integration: `crates/agtrace-cli/src/handlers/watch.rs`
