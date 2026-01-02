use agtrace_sdk::types::{ExecuteArgs, FileReadArgs, ToolCallPayload};

use super::truncate_string;

/// Summarizes tool executions into human-readable one-liners
pub struct ToolSummarizer;

impl ToolSummarizer {
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
