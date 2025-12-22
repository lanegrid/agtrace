/// Codex provider-specific tool argument types
///
/// These structs represent the exact schema that Codex uses, before normalization
/// to the domain model in agtrace-types.
use serde::{Deserialize, Serialize};

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
}
