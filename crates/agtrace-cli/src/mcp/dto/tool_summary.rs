use agtrace_sdk::types::{
    EventPayload, ExecuteArgs, FileReadArgs, ToolCallPayload, ToolResultPayload,
};

use super::common::truncate_string;

/// Summarizes tool executions into human-readable one-liners
pub struct ToolSummarizer;

impl ToolSummarizer {
    /// Generate a one-line summary of a tool execution
    pub fn summarize_execution(
        call: &ToolCallPayload,
        result: Option<&ToolResultPayload>,
        is_error: bool,
    ) -> String {
        let base = Self::summarize_call(call);
        let status = if is_error {
            "failed"
        } else if result.is_some() {
            "ok"
        } else {
            "pending"
        };

        format!("{} ({})", base, status)
    }

    /// Summarize just the tool call (without result)
    pub fn summarize_call(call: &ToolCallPayload) -> String {
        use ToolCallPayload::*;

        match call {
            FileRead {
                name, arguments, ..
            } => Self::summarize_file_read(name, arguments),
            FileWrite {
                name, arguments, ..
            } => {
                let content_len = arguments.content.len();
                let lines = arguments.content.lines().count();
                let path = Self::truncate_path(&arguments.file_path, 40);
                format!("{} {} ({} lines, {} bytes)", name, path, lines, content_len)
            }
            FileEdit {
                name, arguments, ..
            } => {
                let path = Self::truncate_path(&arguments.file_path, 40);
                format!("{} {}", name, path)
            }
            Execute {
                name, arguments, ..
            } => Self::summarize_execute(name, arguments),
            Search {
                name, arguments, ..
            } => {
                // Extract pattern from arguments
                let args_str = serde_json::to_string(arguments).unwrap_or_default();
                let pattern_preview = truncate_string(&args_str, 30);
                format!("{} '{}'", name, pattern_preview)
            }
            Mcp {
                name, arguments, ..
            } => {
                let args_str = serde_json::to_string(arguments).unwrap_or_default();
                let preview = truncate_string(&args_str, 40);
                format!("{} {}", name, preview)
            }
            Generic {
                name, arguments, ..
            } => {
                let args_str = serde_json::to_string(arguments).unwrap_or_default();
                let preview = truncate_string(&args_str, 40);
                format!("{} {}", name, preview)
            }
        }
    }

    fn summarize_file_read(name: &str, args: &FileReadArgs) -> String {
        let path = if let Some(ref fp) = args.file_path {
            Self::truncate_path(fp, 40)
        } else if let Some(ref p) = args.path {
            Self::truncate_path(p, 40)
        } else if let Some(ref pattern) = args.pattern {
            format!("pattern '{}'", truncate_string(pattern, 30))
        } else {
            "(unknown)".to_string()
        };

        format!("{} {}", name, path)
    }

    fn summarize_execute(name: &str, args: &ExecuteArgs) -> String {
        let cmd_str = if let Some(ref cmd) = args.command {
            truncate_string(cmd, 40)
        } else if let Some(ref desc) = args.description {
            truncate_string(desc, 40)
        } else {
            "(unknown)".to_string()
        };

        format!("{} '{}'", name, cmd_str)
    }

    /// Summarize a payload into a preview string
    pub fn summarize_payload(payload: &EventPayload) -> String {
        match payload {
            EventPayload::ToolCall(tc) => Self::summarize_call(tc),
            EventPayload::ToolResult(tr) => {
                let preview = serde_json::to_string(tr)
                    .unwrap_or_default()
                    .chars()
                    .take(100)
                    .collect::<String>();
                format!("Result: {}", preview)
            }
            EventPayload::User(u) => truncate_string(&u.text, 100),
            EventPayload::Message(m) => truncate_string(&m.text, 100),
            EventPayload::Reasoning(r) => truncate_string(&r.text, 100),
            EventPayload::TokenUsage(tu) => {
                format!("Tokens: {}in/{}out", tu.input.total(), tu.output.total())
            }
            EventPayload::Notification(n) => {
                let s = serde_json::to_string(n).unwrap_or_default();
                truncate_string(&s, 100)
            }
        }
    }

    /// Truncate a file path, keeping the filename visible
    fn truncate_path(path: &str, max_len: usize) -> String {
        if path.len() <= max_len {
            return path.to_string();
        }

        // Try to keep filename visible
        if let Some(filename) = path.split('/').next_back()
            && filename.len() < max_len - 3
        {
            return format!(".../{}", filename);
        }

        truncate_string(path, max_len)
    }
}
