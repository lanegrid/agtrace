# agtrace Design Document

## 1. Overview

**agtrace** is a Rust library and CLI tool that unifies session histories from AI coding agents (Claude Code, Codex, and others) into a standardized format for analysis and visualization.

**Repository**: `github.com/lanegrid/agtrace`

**Status**: Part of the [lanegrid](https://github.com/lanegrid/lanegrid) project, but designed to be useful as a standalone tool.

---

## 2. Problem Statement

As AI coding agents become integral to development workflows, understanding what happened during agent sessions becomes critical:

- **No unified view**: Each agent stores history in its own format and location
- **No analysis capability**: Raw logs are hard to query or aggregate
- **No cross-agent comparison**: Impossible to compare behavior across different agents
- **Lost insights**: Patterns in agent behavior (exploration time, tool usage, failure modes) remain invisible

Developers have no way to answer questions like:
- "How long did the agent spend exploring before acting?"
- "Which files did it read across all my sessions?"
- "How does Claude Code compare to Codex for this type of task?"

---

## 3. Design Goals

1. **Unified Format**: One data model for all agents
2. **Zero Configuration**: Auto-detect agent data from standard locations
3. **Lossless Parsing**: Preserve all meaningful information from source logs
4. **Library First**: Usable as a Rust crate, CLI is a thin wrapper
5. **Extensible**: Easy to add new agent parsers
6. **Privacy Aware**: All data stays local, no network calls

---

## 4. Data Model

### 4.1 Core Types

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    /// Unique identifier (from source or generated)
    pub id: String,
    
    /// Which agent produced this execution
    pub agent: Agent,
    
    /// Project directory the agent was working in
    pub project_path: PathBuf,
    
    /// Git branch at time of execution (if available)
    pub git_branch: Option<String>,
    
    /// When the session started
    pub started_at: DateTime<Utc>,
    
    /// When the session ended (None if still running or unknown)
    pub ended_at: Option<DateTime<Utc>>,
    
    /// High-level summaries (from agent's own summarization)
    pub summaries: Vec<String>,
    
    /// Ordered list of events in the session
    pub events: Vec<Event>,
    
    /// Computed metrics
    pub metrics: ExecutionMetrics,
}

/// Supported agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Agent {
    ClaudeCode {
        model: String,      // e.g., "claude-sonnet-4-5-20250929"
        version: String,    // e.g., "2.0.28"
    },
    Codex,
}

/// Events that occur during an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// Human input to the agent
    UserMessage {
        content: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Agent's internal reasoning (extended thinking)
    Thinking {
        content: String,
        duration_ms: Option<u64>,
        timestamp: DateTime<Utc>,
    },
    
    /// Agent's visible response
    AssistantMessage {
        content: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Agent calling a tool
    ToolCall {
        name: String,               // "Read", "Write", "shell", etc.
        input: serde_json::Value,   // Tool-specific arguments
        call_id: Option<String>,    // For matching with results
        timestamp: DateTime<Utc>,
    },
    
    /// Result from a tool call
    ToolResult {
        call_id: Option<String>,
        output: String,
        exit_code: Option<i32>,     // For shell commands
        duration_ms: Option<u64>,
        timestamp: DateTime<Utc>,
    },
    
    /// File state snapshot
    FileSnapshot {
        message_id: String,
        timestamp: DateTime<Utc>,
    },
}

/// Aggregated metrics for an execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total session duration in seconds
    pub duration_seconds: Option<u64>,
    
    /// Token usage
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    
    /// Event counts
    pub user_message_count: u32,
    pub assistant_message_count: u32,
    pub thinking_count: u32,
    pub tool_call_count: u32,
    
    /// Tool usage breakdown
    pub tool_calls_by_name: HashMap<String, u32>,
    
    /// File operations
    pub files_read: Vec<PathBuf>,
    pub files_written: Vec<PathBuf>,
    
    /// Shell commands executed
    pub shell_commands: Vec<String>,
}
```

### 4.2 Design Decisions

- **Event-based**: Sessions are modeled as a stream of events, preserving temporal order
- **Denormalized metrics**: Pre-computed for fast queries, derived from events
- **Serde-friendly**: All types serialize cleanly to JSON for interop
- **Optional fields**: Graceful handling of missing data from different agent versions

---

## 5. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        agtrace                              │
├─────────────────────────────────────────────────────────────┤
│  CLI Layer (src/cli/)                                       │
│    - import, list, show, stats commands                     │
│    - Thin wrapper over library                              │
├─────────────────────────────────────────────────────────────┤
│  Library (src/lib.rs)                                       │
│    - Public API for external consumers                      │
├─────────────────────────────────────────────────────────────┤
│  Parsers (src/parser/)                                      │
│    ├── claude_code.rs   ~/.claude/projects/*/*.jsonl        │
│    ├── codex.rs         ~/.codex/sessions/*.json            │
│    └── (future agents)                                      │
├─────────────────────────────────────────────────────────────┤
│  Model (src/model/)                                         │
│    - Execution, Event, Agent, ExecutionMetrics              │
├─────────────────────────────────────────────────────────────┤
│  Storage (src/storage/)                                     │
│    - Read/write executions to ~/.agtrace/                   │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Source Formats

### 6.1 Claude Code

**Location**: `~/.claude/projects/<encoded-path>/*.jsonl`

**Format**: JSONL (one JSON object per line)

**Key fields**:
```json
{
  "type": "user" | "assistant" | "file-history-snapshot" | "summary",
  "parentUuid": "...",
  "sessionId": "...",
  "timestamp": "2025-11-22T12:45:22.800Z",
  "cwd": "/path/to/project",
  "gitBranch": "main",
  "message": {
    "role": "user" | "assistant",
    "content": [
      { "type": "text", "text": "..." },
      { "type": "thinking", "thinking": "..." },
      { "type": "tool_use", "name": "Read", "input": {...} }
    ],
    "usage": {
      "input_tokens": 10,
      "output_tokens": 977,
      "cache_read_input_tokens": 12135
    }
  }
}
```

**Parsing strategy**:
1. Scan `~/.claude/projects/` for directories
2. For each project dir, find all `.jsonl` files
3. Parse each file as a separate session
4. Convert events to unified format
5. Compute metrics from events

### 6.2 Codex

**Location**: `~/.codex/sessions/rollout-*.json`

**Format**: Single JSON file per session

**Key fields**:
```json
{
  "session": {
    "timestamp": "2025-04-18T13:30:43.105Z",
    "id": "81c29260-6bf6-401a-a377-cc0c84f53123",
    "instructions": ""
  },
  "items": [
    { "role": "user", "content": [...], "type": "message" },
    { "type": "reasoning", "duration_ms": 4034 },
    { "type": "function_call", "name": "shell", "arguments": "..." },
    { "type": "function_call_output", "output": "..." }
  ]
}
```

**Parsing strategy**:
1. Scan `~/.codex/sessions/` for `rollout-*.json` files
2. Parse each file as a session
3. Map `items` array to Event enum
4. Compute metrics

---

## 7. CLI Interface

```bash
# List executions (reads directly from agent directories)
agtrace list                        # All executions from all agents
agtrace list --agent claude-code    # Only Claude Code sessions
agtrace list --agent codex          # Only Codex sessions
agtrace list --path /custom/path    # Read from custom location
agtrace list --project ./my-repo    # Filter by project
agtrace list --since 2025-01-01     # Filter by date
agtrace list --limit 20             # Limit results

# Show execution details
agtrace show <agent> <execution-id>
agtrace show claude-code <session-id> --events  # Include event timeline
agtrace show codex <session-id> --json          # Output as JSON

# Statistics (computed on-the-fly from agent directories)
agtrace stats                       # Overall statistics
agtrace stats --agent claude-code   # Only Claude Code
agtrace stats --by-agent            # Grouped by agent
agtrace stats --by-project          # Grouped by project
agtrace stats --by-day              # Grouped by day

# Export (reads directly and exports)
agtrace export <agent> <execution-id> --format json
agtrace export --all --format jsonl
agtrace export --agent claude-code --format jsonl
```

---

## 8. Storage

agtrace does not persist parsed data. It reads directly from agent
data directories (`~/.claude`, `~/.codex`) on each invocation.

**Rationale**:
- Source directories are the single source of truth
- No sync issues between original and cached data
- Simpler implementation
- Consumers (like lanegrid) can implement their own persistence if needed

---

## 9. Library API

```rust
// Importing
use agtrace::{parser, Execution, Agent};

// Parse Claude Code sessions
let executions = parser::claude_code::parse_dir("~/.claude")?;

// Parse Codex sessions  
let executions = parser::codex::parse_dir("~/.codex")?;

// Parse from custom path
let executions = parser::claude_code::parse_dir("/custom/path")?;

// Access execution data
for exec in executions {
    println!("Session: {}", exec.id);
    println!("Project: {:?}", exec.project_path);
    println!("Duration: {:?}s", exec.metrics.duration_seconds);
    println!("Tool calls: {}", exec.metrics.tool_call_count);
    
    for event in &exec.events {
        match event {
            Event::ToolCall { name, timestamp, .. } => {
                println!("  {} at {}", name, timestamp);
            }
            _ => {}
        }
    }
}
```

---

## 10. Integration with lanegrid

agtrace is designed to be consumed by lanegrid:

```rust
// In lanegrid
use agtrace::{parser, Execution};

impl TraceManager {
    pub fn import_from_agents(&mut self) -> Result<()> {
        // Import Claude Code sessions
        let claude_execs = parser::claude_code::parse_default_dir()?;
        
        // Convert to lanegrid's internal format
        for exec in claude_execs {
            let trace_entry = self.convert_execution(exec)?;
            self.store(trace_entry)?;
        }
        
        Ok(())
    }
}
```

lanegrid will add:
- **Objective linking**: Associate executions with Objectives
- **Lane tracking**: Record which Lane an execution ran under
- **Drift detection**: Compare executions against current project state

---

## 11. Roadmap

### Phase 1: MVP (Week 1-2)
- [x] Project setup (Cargo.toml, lib.rs, main.rs)
- [x] Data model implementation
- [x] Claude Code parser
- [x] Codex parser
- [x] Basic CLI (list, show, stats, export)
- [x] Tests and documentation

### Phase 2: Extended (Future)
- [ ] More agent parsers (Cursor, Copilot, etc.)
- [ ] Export formats (CSV, HTML report)
- [ ] Performance optimization for large session histories
- [ ] Incremental parsing (cache file metadata to avoid re-parsing)

---

## 12. Non-Goals

- **Real-time monitoring**: agtrace works on historical data
- **Agent control**: agtrace only observes, never modifies agent behavior
- **Cloud sync**: All data stays local
- **Agent-specific features**: Focus on common denominator across agents
- **Data persistence**: agtrace does not cache or store parsed data

---

## 13. Open Questions

1. **Session boundaries**: How to handle sessions that span multiple files?
2. **Privacy**: Should we redact sensitive content (API keys, etc.)?
3. **Performance**: For very large session histories, should we implement lazy loading or pagination?

---

## 14. References

- Claude Code data: `~/.claude/projects/`
- Codex data: `~/.codex/sessions/`
- lanegrid: `github.com/lanegrid/lanegrid`
