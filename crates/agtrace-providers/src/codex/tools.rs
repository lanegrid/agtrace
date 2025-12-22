/// Codex provider-specific tool argument types
///
/// These structs represent the exact schema that Codex uses, before normalization
/// to the domain model in agtrace-types.
use agtrace_types::{ExecuteArgs, FileReadArgs};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Codex apply_patch tool arguments
///
/// Raw patch format used by Codex for both file creation and modification.
///
/// # Format
/// ```text
/// *** Begin Patch
/// *** Add File: path/to/file.rs
/// +content line 1
/// +content line 2
/// *** End Patch
/// ```
///
/// or
///
/// ```text
/// *** Begin Patch
/// *** Update File: path/to/file.rs
/// @@
///  context line
/// -old line
/// +new line
/// @@
///  another context
/// -another old
/// +another new
/// *** End Patch
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchArgs {
    /// Raw patch content including Begin/End markers, file path header, and diff hunks
    pub raw: String,
}

/// Parsed patch structure extracted from ApplyPatchArgs
///
/// This represents the structured view of Codex's patch format after parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPatch {
    /// Operation type (Add or Update)
    pub operation: PatchOperation,
    /// Target file path
    pub file_path: String,
    /// Original raw patch for preservation
    pub raw_patch: String,
}

/// Patch operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchOperation {
    /// File creation (*** Add File:)
    Add,
    /// File modification (*** Update File:)
    Update,
}

impl ApplyPatchArgs {
    /// Parse the raw patch into structured format
    ///
    /// # Errors
    /// Returns error if:
    /// - No file path header found (neither "Add File:" nor "Update File:")
    /// - Invalid patch format
    pub fn parse(&self) -> Result<ParsedPatch, String> {
        let raw = &self.raw;

        // Find operation and file path
        for line in raw.lines() {
            if let Some(path) = line.strip_prefix("*** Add File: ") {
                return Ok(ParsedPatch {
                    operation: PatchOperation::Add,
                    file_path: path.trim().to_string(),
                    raw_patch: raw.clone(),
                });
            }
            if let Some(path) = line.strip_prefix("*** Update File: ") {
                return Ok(ParsedPatch {
                    operation: PatchOperation::Update,
                    file_path: path.trim().to_string(),
                    raw_patch: raw.clone(),
                });
            }
        }

        Err("Failed to parse patch: no file path header found".to_string())
    }
}

/// Codex shell tool arguments
///
/// Codex uses array command format instead of string format.
///
/// # Format
/// ```json
/// {
///   "command": ["bash", "-lc", "ls"],
///   "timeout_ms": 10000,
///   "workdir": "/path/to/dir"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellArgs {
    /// Command as array of strings (e.g., ["bash", "-lc", "ls"])
    pub command: Vec<String>,
    /// Timeout in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Working directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
}

impl ShellArgs {
    /// Convert Codex shell args to standard ExecuteArgs
    ///
    /// - Joins command array into a single string
    /// - Converts timeout_ms to timeout
    /// - Preserves workdir in extra field
    pub fn to_execute_args(&self) -> ExecuteArgs {
        let command_str = self.command.join(" ");

        let mut extra = json!({});
        if let Some(workdir) = &self.workdir {
            extra["workdir"] = json!(workdir);
        }

        ExecuteArgs {
            command: Some(command_str),
            description: None,
            timeout: self.timeout_ms,
            extra,
        }
    }
}

/// Codex read_mcp_resource tool arguments
///
/// Codex uses MCP protocol to read resources via a server.
///
/// # Format
/// ```json
/// {
///   "server": "local",
///   "uri": "/path/to/file"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMcpResourceArgs {
    /// MCP server name (e.g., "local")
    pub server: String,
    /// Resource URI (file path for local server)
    pub uri: String,
}

impl ReadMcpResourceArgs {
    /// Convert Codex read_mcp_resource args to standard FileReadArgs
    ///
    /// - Maps uri → file_path
    /// - Preserves server in extra field
    pub fn to_file_read_args(&self) -> FileReadArgs {
        let mut extra = json!({});
        extra["server"] = json!(&self.server);

        FileReadArgs {
            file_path: Some(self.uri.clone()),
            path: None,
            pattern: None,
            extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add_file_patch() {
        let args = ApplyPatchArgs {
            raw: r#"*** Begin Patch
*** Add File: docs/test.md
+# Test Document
+
+This is a test.
*** End Patch"#
                .to_string(),
        };

        let parsed = args.parse().unwrap();
        assert_eq!(parsed.operation, PatchOperation::Add);
        assert_eq!(parsed.file_path, "docs/test.md");
    }

    #[test]
    fn test_parse_update_file_patch() {
        let args = ApplyPatchArgs {
            raw: r#"*** Begin Patch
*** Update File: src/lib.rs
@@
 fn example() {
-    old_code()
+    new_code()
 }
*** End Patch"#
                .to_string(),
        };

        let parsed = args.parse().unwrap();
        assert_eq!(parsed.operation, PatchOperation::Update);
        assert_eq!(parsed.file_path, "src/lib.rs");
    }

    #[test]
    fn test_parse_invalid_patch() {
        let args = ApplyPatchArgs {
            raw: "*** Begin Patch\nno header\n*** End Patch".to_string(),
        };

        assert!(args.parse().is_err());
    }

    #[test]
    fn test_shell_args_to_execute_args() {
        let shell_args = ShellArgs {
            command: vec!["bash".to_string(), "-lc".to_string(), "ls".to_string()],
            timeout_ms: Some(10000),
            workdir: Some("/path/to/dir".to_string()),
        };

        let execute_args = shell_args.to_execute_args();
        assert_eq!(execute_args.command, Some("bash -lc ls".to_string()));
        assert_eq!(execute_args.timeout, Some(10000));
        assert_eq!(
            execute_args.extra.get("workdir"),
            Some(&json!("/path/to/dir"))
        );
    }

    #[test]
    fn test_shell_args_without_optional_fields() {
        let shell_args = ShellArgs {
            command: vec!["echo".to_string(), "hello".to_string()],
            timeout_ms: None,
            workdir: None,
        };

        let execute_args = shell_args.to_execute_args();
        assert_eq!(execute_args.command, Some("echo hello".to_string()));
        assert_eq!(execute_args.timeout, None);
        assert_eq!(execute_args.extra, json!({}));
    }

    #[test]
    fn test_read_mcp_resource_args_to_file_read_args() {
        let mcp_args = ReadMcpResourceArgs {
            server: "local".to_string(),
            uri: "/Users/zawakin/go/src/github.com/lanegrid/agtrace/AGENTS.md".to_string(),
        };

        let file_read_args = mcp_args.to_file_read_args();
        assert_eq!(
            file_read_args.file_path,
            Some("/Users/zawakin/go/src/github.com/lanegrid/agtrace/AGENTS.md".to_string())
        );
        assert_eq!(file_read_args.path, None);
        assert_eq!(file_read_args.pattern, None);
        assert_eq!(file_read_args.extra.get("server"), Some(&json!("local")));
    }
}
