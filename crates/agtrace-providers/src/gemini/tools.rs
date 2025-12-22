/// Gemini provider-specific tool argument types
///
/// These structs represent the exact schema that Gemini uses, before normalization
/// to the domain model in agtrace-types.
use agtrace_types::{ExecuteArgs, FileEditArgs, FileReadArgs, FileWriteArgs, SearchArgs};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Gemini read_file tool arguments
///
/// Gemini uses the same schema as domain model for read_file.
///
/// # Format
/// ```json
/// {
///   "file_path": "src/main.rs"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiReadFileArgs {
    pub file_path: String,
}

impl GeminiReadFileArgs {
    /// Convert Gemini read_file args to standard FileReadArgs
    pub fn to_file_read_args(&self) -> FileReadArgs {
        FileReadArgs {
            file_path: Some(self.file_path.clone()),
            path: None,
            pattern: None,
            extra: json!({}),
        }
    }
}

/// Gemini write_file tool arguments
///
/// Gemini uses the same schema as domain model for write_file.
///
/// # Format
/// ```json
/// {
///   "content": "...",
///   "file_path": "test.txt"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiWriteFileArgs {
    pub content: String,
    pub file_path: String,
}

impl GeminiWriteFileArgs {
    /// Convert Gemini write_file args to standard FileWriteArgs
    pub fn to_file_write_args(&self) -> FileWriteArgs {
        FileWriteArgs {
            file_path: self.file_path.clone(),
            content: self.content.clone(),
        }
    }
}

/// Gemini replace tool arguments
///
/// Gemini includes an `instruction` field that explains the edit,
/// which is not present in the domain model.
///
/// # Format
/// ```json
/// {
///   "file_path": "src/lib.rs",
///   "instruction": "Update import statement...",
///   "old_string": "old code",
///   "new_string": "new code"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiReplaceArgs {
    pub file_path: String,
    /// Gemini-specific: explanation of the edit
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    pub old_string: String,
    pub new_string: String,
}

impl GeminiReplaceArgs {
    /// Convert Gemini replace args to standard FileEditArgs
    ///
    /// Note: FileEditArgs doesn't have an `extra` field, so the Gemini-specific
    /// `instruction` field is currently lost during normalization.
    /// TODO: Add `extra` field to FileEditArgs in agtrace-types to preserve this.
    ///
    /// - Sets replace_all to false (Gemini doesn't support this option)
    pub fn to_file_edit_args(&self) -> FileEditArgs {
        // TODO: Preserve instruction in extra field when FileEditArgs supports it
        FileEditArgs {
            file_path: self.file_path.clone(),
            old_string: self.old_string.clone(),
            new_string: self.new_string.clone(),
            replace_all: false, // Gemini doesn't support replace_all
        }
    }
}

/// Gemini run_shell_command tool arguments
///
/// Gemini uses the same schema as domain model for run_shell_command.
///
/// # Format
/// ```json
/// {
///   "command": "ls -la",
///   "description": "List files in directory"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiRunShellCommandArgs {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl GeminiRunShellCommandArgs {
    /// Convert Gemini run_shell_command args to standard ExecuteArgs
    pub fn to_execute_args(&self) -> ExecuteArgs {
        ExecuteArgs {
            command: Some(self.command.clone()),
            description: self.description.clone(),
            timeout: None,
            extra: json!({}),
        }
    }
}

/// Gemini google_web_search tool arguments
///
/// # Format
/// ```json
/// {
///   "query": "rust async programming"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGoogleWebSearchArgs {
    pub query: String,
}

impl GeminiGoogleWebSearchArgs {
    /// Convert Gemini google_web_search args to standard SearchArgs
    pub fn to_search_args(&self) -> SearchArgs {
        SearchArgs {
            pattern: None,
            query: Some(self.query.clone()),
            input: None,
            path: None,
            extra: json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_args_conversion() {
        let args = GeminiReadFileArgs {
            file_path: "src/main.rs".to_string(),
        };

        let domain_args = args.to_file_read_args();
        assert_eq!(domain_args.file_path, Some("src/main.rs".to_string()));
        assert_eq!(domain_args.path, None);
        assert_eq!(domain_args.pattern, None);
        assert_eq!(domain_args.extra, json!({}));
    }

    #[test]
    fn test_write_file_args_conversion() {
        let args = GeminiWriteFileArgs {
            content: "hello world".to_string(),
            file_path: "test.txt".to_string(),
        };

        let domain_args = args.to_file_write_args();
        assert_eq!(domain_args.file_path, "test.txt");
        assert_eq!(domain_args.content, "hello world");
    }

    #[test]
    fn test_replace_args_with_instruction() {
        let args = GeminiReplaceArgs {
            file_path: "src/lib.rs".to_string(),
            instruction: Some("Update import statement".to_string()),
            old_string: "old code".to_string(),
            new_string: "new code".to_string(),
        };

        let domain_args = args.to_file_edit_args();
        assert_eq!(domain_args.file_path, "src/lib.rs");
        assert_eq!(domain_args.old_string, "old code");
        assert_eq!(domain_args.new_string, "new code");
        assert_eq!(domain_args.replace_all, false);
        // Note: instruction is lost due to lack of extra field in FileEditArgs
    }

    #[test]
    fn test_replace_args_without_instruction() {
        let args = GeminiReplaceArgs {
            file_path: "src/lib.rs".to_string(),
            instruction: None,
            old_string: "old".to_string(),
            new_string: "new".to_string(),
        };

        let domain_args = args.to_file_edit_args();
        assert_eq!(domain_args.file_path, "src/lib.rs");
        assert_eq!(domain_args.old_string, "old");
        assert_eq!(domain_args.new_string, "new");
        assert_eq!(domain_args.replace_all, false);
    }

    #[test]
    fn test_run_shell_command_args_conversion() {
        let args = GeminiRunShellCommandArgs {
            command: "ls -la".to_string(),
            description: Some("List files".to_string()),
        };

        let domain_args = args.to_execute_args();
        assert_eq!(domain_args.command, Some("ls -la".to_string()));
        assert_eq!(domain_args.description, Some("List files".to_string()));
        assert_eq!(domain_args.timeout, None);
        assert_eq!(domain_args.extra, json!({}));
    }

    #[test]
    fn test_run_shell_command_args_without_description() {
        let args = GeminiRunShellCommandArgs {
            command: "pwd".to_string(),
            description: None,
        };

        let domain_args = args.to_execute_args();
        assert_eq!(domain_args.command, Some("pwd".to_string()));
        assert_eq!(domain_args.description, None);
    }

    #[test]
    fn test_google_web_search_args_conversion() {
        let args = GeminiGoogleWebSearchArgs {
            query: "rust async".to_string(),
        };

        let domain_args = args.to_search_args();
        assert_eq!(domain_args.query, Some("rust async".to_string()));
        assert_eq!(domain_args.extra, json!({}));
    }
}
