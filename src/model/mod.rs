use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A single agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    /// Unique identifier (from source or generated)
    pub id: String,

    /// Which agent produced this execution
    pub agent: Agent,

    /// Working directory where the agent was executed
    pub working_dir: PathBuf,

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
#[serde(tag = "type")]
pub enum Agent {
    #[serde(rename = "claude-code")]
    ClaudeCode {
        model: String,   // e.g., "claude-sonnet-4-5-20250929"
        version: String, // e.g., "2.0.28"
    },
    #[serde(rename = "codex")]
    Codex {
        model: String, // e.g., "gpt-5-codex"
    },
}

/// Events that occur during an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// Human input to the agent
    #[serde(rename = "user_message")]
    UserMessage {
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Agent's internal reasoning (extended thinking)
    #[serde(rename = "thinking")]
    Thinking {
        content: String,
        duration_ms: Option<u64>,
        timestamp: DateTime<Utc>,
    },

    /// Agent's visible response
    #[serde(rename = "assistant_message")]
    AssistantMessage {
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Agent calling a tool
    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,             // "Read", "Write", "shell", etc.
        input: serde_json::Value, // Tool-specific arguments
        call_id: Option<String>,  // For matching with results
        timestamp: DateTime<Utc>,
    },

    /// Result from a tool call
    #[serde(rename = "tool_result")]
    ToolResult {
        call_id: Option<String>,
        output: String,
        exit_code: Option<i32>, // For shell commands
        duration_ms: Option<u64>,
        timestamp: DateTime<Utc>,
    },

    /// File state snapshot
    #[serde(rename = "file_snapshot")]
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

impl Execution {
    /// Compute metrics from events
    pub fn compute_metrics(&mut self) {
        let mut metrics = ExecutionMetrics::default();

        // Calculate duration
        if let Some(ended_at) = self.ended_at {
            let duration = ended_at.signed_duration_since(self.started_at);
            metrics.duration_seconds = Some(duration.num_seconds() as u64);
        }

        // Process events
        for event in &self.events {
            match event {
                Event::UserMessage { .. } => {
                    metrics.user_message_count += 1;
                }
                Event::AssistantMessage { .. } => {
                    metrics.assistant_message_count += 1;
                }
                Event::Thinking { .. } => {
                    metrics.thinking_count += 1;
                }
                Event::ToolCall { name, input, .. } => {
                    metrics.tool_call_count += 1;
                    *metrics.tool_calls_by_name.entry(name.clone()).or_insert(0) += 1;

                    // Track file operations
                    match name.as_str() {
                        "Read" | "Glob" | "Grep" => {
                            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                                metrics.files_read.push(PathBuf::from(path));
                            } else if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                                metrics.files_read.push(PathBuf::from(path));
                            }
                        }
                        "Write" | "Edit" => {
                            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                                metrics.files_written.push(PathBuf::from(path));
                            }
                        }
                        "Bash" | "shell" => {
                            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                                metrics.shell_commands.push(cmd.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        self.metrics = metrics;
    }
}
