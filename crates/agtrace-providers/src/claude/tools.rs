/// Claude Code provider-specific tool argument types
///
/// These structs represent the exact schema that Claude Code uses, before normalization
/// to the domain model in agtrace-types.
///
/// Note: Claude Code schemas happen to align closely with the domain model, but we document
/// them here for:
/// 1. Provider-specific validation
/// 2. Future schema divergence
/// 3. Claude-specific fields (e.g., timeout, dangerouslyDisableSandbox)
use agtrace_types::{ExecuteArgs, FileEditArgs, FileReadArgs, FileWriteArgs, SearchArgs};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Claude Code Bash tool arguments
///
/// Claude Code Bash supports additional fields beyond the domain model.
///
/// # Format
/// ```json
/// {
///   "command": "cargo build --release",
///   "description": "Build agtrace in release mode",
///   "timeout": 120000,
///   "dangerouslyDisableSandbox": false,
///   "run_in_background": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ClaudeBashArgs {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dangerouslyDisableSandbox: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
}

impl ClaudeBashArgs {
    /// Convert Claude Bash args to standard ExecuteArgs
    pub fn to_execute_args(&self) -> ExecuteArgs {
        let mut extra = json!({});
        if let Some(sandbox) = self.dangerouslyDisableSandbox {
            extra["dangerouslyDisableSandbox"] = json!(sandbox);
        }
        if let Some(bg) = self.run_in_background {
            extra["run_in_background"] = json!(bg);
        }

        ExecuteArgs {
            command: Some(self.command.clone()),
            description: self.description.clone(),
            timeout: self.timeout,
            extra,
        }
    }
}

/// Claude Code Read tool arguments
///
/// # Format
/// ```json
/// {
///   "file_path": "/path/to/file.rs",
///   "offset": 0,
///   "limit": 2000
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeReadArgs {
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

impl ClaudeReadArgs {
    /// Convert Claude Read args to standard FileReadArgs
    pub fn to_file_read_args(&self) -> FileReadArgs {
        let mut extra = json!({});
        if let Some(offset) = self.offset {
            extra["offset"] = json!(offset);
        }
        if let Some(limit) = self.limit {
            extra["limit"] = json!(limit);
        }

        FileReadArgs {
            file_path: Some(self.file_path.clone()),
            path: None,
            pattern: None,
            extra,
        }
    }
}

/// Claude Code Glob tool arguments
///
/// # Format
/// ```json
/// {
///   "pattern": "**/*.rs",
///   "path": "/path/to/search"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGlobArgs {
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ClaudeGlobArgs {
    /// Convert Claude Glob args to standard FileReadArgs
    pub fn to_file_read_args(&self) -> FileReadArgs {
        FileReadArgs {
            file_path: None,
            path: self.path.clone(),
            pattern: Some(self.pattern.clone()),
            extra: json!({}),
        }
    }
}

/// Claude Code Edit tool arguments
///
/// # Format
/// ```json
/// {
///   "file_path": "src/lib.rs",
///   "old_string": "old code",
///   "new_string": "new code",
///   "replace_all": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeEditArgs {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

impl ClaudeEditArgs {
    /// Convert Claude Edit args to standard FileEditArgs
    pub fn to_file_edit_args(&self) -> FileEditArgs {
        FileEditArgs {
            file_path: self.file_path.clone(),
            old_string: self.old_string.clone(),
            new_string: self.new_string.clone(),
            replace_all: self.replace_all,
        }
    }
}

/// Claude Code Write tool arguments
///
/// # Format
/// ```json
/// {
///   "file_path": "test.txt",
///   "content": "hello world"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeWriteArgs {
    pub file_path: String,
    pub content: String,
}

impl ClaudeWriteArgs {
    /// Convert Claude Write args to standard FileWriteArgs
    pub fn to_file_write_args(&self) -> FileWriteArgs {
        FileWriteArgs {
            file_path: self.file_path.clone(),
            content: self.content.clone(),
        }
    }
}

/// Claude Code Grep tool arguments
///
/// # Format
/// ```json
/// {
///   "pattern": "fn main",
///   "path": "src/",
///   "glob": "*.rs",
///   "type": "rust",
///   "output_mode": "content",
///   "-i": true,
///   "-n": true,
///   "-A": 5,
///   "-B": 5,
///   "-C": 5,
///   "head_limit": 100,
///   "multiline": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGrepArgs {
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<String>,
    #[serde(rename = "-i", default, skip_serializing_if = "Option::is_none")]
    pub case_insensitive: Option<bool>,
    #[serde(rename = "-n", default, skip_serializing_if = "Option::is_none")]
    pub line_numbers: Option<bool>,
    #[serde(rename = "-A", default, skip_serializing_if = "Option::is_none")]
    pub after_context: Option<u64>,
    #[serde(rename = "-B", default, skip_serializing_if = "Option::is_none")]
    pub before_context: Option<u64>,
    #[serde(rename = "-C", default, skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiline: Option<bool>,
}

impl ClaudeGrepArgs {
    /// Convert Claude Grep args to standard SearchArgs
    pub fn to_search_args(&self) -> SearchArgs {
        let mut extra = json!({});
        if let Some(glob) = &self.glob {
            extra["glob"] = json!(glob);
        }
        if let Some(t) = &self.r#type {
            extra["type"] = json!(t);
        }
        if let Some(mode) = &self.output_mode {
            extra["output_mode"] = json!(mode);
        }
        if let Some(ci) = self.case_insensitive {
            extra["-i"] = json!(ci);
        }
        if let Some(ln) = self.line_numbers {
            extra["-n"] = json!(ln);
        }
        if let Some(a) = self.after_context {
            extra["-A"] = json!(a);
        }
        if let Some(b) = self.before_context {
            extra["-B"] = json!(b);
        }
        if let Some(c) = self.context {
            extra["-C"] = json!(c);
        }
        if let Some(hl) = self.head_limit {
            extra["head_limit"] = json!(hl);
        }
        if let Some(ml) = self.multiline {
            extra["multiline"] = json!(ml);
        }

        SearchArgs {
            pattern: Some(self.pattern.clone()),
            query: None,
            input: None,
            path: self.path.clone(),
            extra,
        }
    }
}

/// Claude Code TodoWrite tool arguments
///
/// Claude uses `content` and `activeForm` for todo items.
///
/// # Format
/// ```json
/// {
///   "todos": [
///     {
///       "content": "Task description",
///       "activeForm": "Task in progress form",
///       "status": "pending"
///     }
///   ]
/// }
/// ```
///
/// # Normalization Note
/// This tool maps to ToolKind::Plan but currently falls back to Generic variant
/// since there's no unified Plan variant in ToolCallPayload yet.
/// The raw JSON is preserved in Generic.arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeTodoWriteArgs {
    pub todos: Vec<ClaudeTodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ClaudeTodoItem {
    /// Task description (Claude-specific field name)
    pub content: String,
    /// Active form of the task (Claude-specific, used for progress display)
    pub activeForm: String,
    /// Task status: "pending", "in_progress", "completed", "cancelled"
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_args_conversion() {
        let args = ClaudeBashArgs {
            command: "ls -la".to_string(),
            description: Some("List files".to_string()),
            timeout: Some(60000),
            dangerouslyDisableSandbox: Some(false),
            run_in_background: None,
        };

        let domain_args = args.to_execute_args();
        assert_eq!(domain_args.command, Some("ls -la".to_string()));
        assert_eq!(domain_args.description, Some("List files".to_string()));
        assert_eq!(domain_args.timeout, Some(60000));
        assert_eq!(
            domain_args.extra.get("dangerouslyDisableSandbox"),
            Some(&json!(false))
        );
    }

    #[test]
    fn test_read_args_conversion() {
        let args = ClaudeReadArgs {
            file_path: "src/main.rs".to_string(),
            offset: Some(100),
            limit: Some(500),
        };

        let domain_args = args.to_file_read_args();
        assert_eq!(domain_args.file_path, Some("src/main.rs".to_string()));
        assert_eq!(domain_args.extra.get("offset"), Some(&json!(100)));
        assert_eq!(domain_args.extra.get("limit"), Some(&json!(500)));
    }

    #[test]
    fn test_glob_args_conversion() {
        let args = ClaudeGlobArgs {
            pattern: "**/*.rs".to_string(),
            path: Some("src/".to_string()),
        };

        let domain_args = args.to_file_read_args();
        assert_eq!(domain_args.pattern, Some("**/*.rs".to_string()));
        assert_eq!(domain_args.path, Some("src/".to_string()));
    }

    #[test]
    fn test_edit_args_conversion() {
        let args = ClaudeEditArgs {
            file_path: "src/lib.rs".to_string(),
            old_string: "old".to_string(),
            new_string: "new".to_string(),
            replace_all: true,
        };

        let domain_args = args.to_file_edit_args();
        assert_eq!(domain_args.file_path, "src/lib.rs");
        assert_eq!(domain_args.old_string, "old");
        assert_eq!(domain_args.new_string, "new");
        assert!(domain_args.replace_all);
    }

    #[test]
    fn test_write_args_conversion() {
        let args = ClaudeWriteArgs {
            file_path: "test.txt".to_string(),
            content: "hello world".to_string(),
        };

        let domain_args = args.to_file_write_args();
        assert_eq!(domain_args.file_path, "test.txt");
        assert_eq!(domain_args.content, "hello world");
    }

    #[test]
    fn test_grep_args_conversion() {
        let args = ClaudeGrepArgs {
            pattern: "fn main".to_string(),
            path: Some("src/".to_string()),
            glob: Some("*.rs".to_string()),
            r#type: None,
            output_mode: Some("content".to_string()),
            case_insensitive: Some(true),
            line_numbers: Some(true),
            after_context: Some(3),
            before_context: None,
            context: None,
            head_limit: Some(100),
            multiline: None,
        };

        let domain_args = args.to_search_args();
        assert_eq!(domain_args.pattern, Some("fn main".to_string()));
        assert_eq!(domain_args.path, Some("src/".to_string()));
        assert_eq!(domain_args.extra.get("glob"), Some(&json!("*.rs")));
        assert_eq!(
            domain_args.extra.get("output_mode"),
            Some(&json!("content"))
        );
        assert_eq!(domain_args.extra.get("-i"), Some(&json!(true)));
        assert_eq!(domain_args.extra.get("-A"), Some(&json!(3)));
    }

    #[test]
    fn test_todo_write_args_parsing() {
        let json_value = json!({
            "todos": [
                {
                    "content": "Create tools.rs",
                    "activeForm": "Creating tools.rs",
                    "status": "in_progress"
                },
                {
                    "content": "Add tests",
                    "activeForm": "Adding tests",
                    "status": "pending"
                }
            ]
        });

        let args: ClaudeTodoWriteArgs = serde_json::from_value(json_value).unwrap();
        assert_eq!(args.todos.len(), 2);
        assert_eq!(args.todos[0].content, "Create tools.rs");
        assert_eq!(args.todos[0].activeForm, "Creating tools.rs");
        assert_eq!(args.todos[0].status, "in_progress");
    }
}
