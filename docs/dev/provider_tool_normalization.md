# Provider Tool Normalization Pattern

## Overview

This document describes the pattern for defining provider-specific tool structures and mapping them to domain models in agtrace.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│ agtrace-types (Domain Layer)                                │
│ - FileEditArgs, FileWriteArgs, etc.                         │
│ - Provider-agnostic, semantic tool representations          │
└─────────────────────────────────────────────────────────────┘
                          ▲
                          │ normalization
                          │
┌─────────────────────────────────────────────────────────────┐
│ agtrace-providers/<provider>/tools.rs (Provider Layer)      │
│ - Provider-specific tool argument structs                   │
│ - Parsing logic for raw provider data                       │
│ - Exact representation of provider's schema                 │
└─────────────────────────────────────────────────────────────┘
                          ▲
                          │ deserialization
                          │
┌─────────────────────────────────────────────────────────────┐
│ Provider Raw Data (JSON/JSONL logs)                         │
│ - Exact format from provider (Claude Code, Codex, Gemini)   │
└─────────────────────────────────────────────────────────────┘
```

## Design Principles

1. **Provider Schema First**: Define structs that exactly match provider's raw data format
2. **Use `--raw` for Investigation**: Always use `lab grep --raw` to understand provider schemas
   - See both normalized data and original provider format side-by-side
   - Verify extraction/transformation logic is correct
   - Essential for reverse engineering provider-specific quirks
3. **Explicit Parsing**: Parse provider structs into domain models explicitly, handling all edge cases
4. **Preserve Raw Data**: Keep original raw format for debugging and future analysis
5. **Type Safety**: Use Rust's type system to catch incompatibilities at compile time
6. **Testability**: Test both parsing (provider → struct) and normalization (struct → domain model)

## Case Study: Codex `apply_patch`

### Step 1: Analyze Raw Data

**IMPORTANT**: Always use `--raw` flag for schema investigation. This is essential for reverse engineering provider schemas.

```bash
./target/release/agtrace lab grep '"name":"apply_patch"' --raw --limit 5
```

#### Why `--raw` is Critical

**`--raw` vs `--json`**:

- **`--json`**: Shows only normalized data (`content.arguments`)
  - You see the result after normalization
  - Provider-specific schema is hidden
  - ❌ Cannot verify if normalization is correct

- **`--raw`**: Shows complete AgentEvent with metadata
  - `content.arguments`: Normalized data (after mapping to domain model)
  - `metadata.payload`: Provider-specific raw data (before normalization)
  - ✅ Can compare before/after to verify normalization logic

**Example Output with `--raw`**:

```json
{
  "type": "tool_call",
  "content": {
    "name": "apply_patch",
    "arguments": {
      "file_path": "test.rs",           // ← Extracted by normalization
      "old_string": "",
      "new_string": "*** Begin Patch...", // ← Stored for reference
      "replace_all": false
    }
  },
  "metadata": {
    "payload": {
      "input": "*** Begin Patch\n*** Update File: test.rs\n@@...",  // ← Original raw string
      "name": "apply_patch",
      "type": "custom_tool_call"
    }
  }
}
```

**Key Observations**:
- `metadata.payload.input`: Provider's original format (just a string)
- `content.arguments.file_path`: Extracted during normalization
- Can verify: "Was `test.rs` correctly extracted from the patch header?"

#### Schema Analysis

From `--raw` output, we observe:

- Two operation types: `*** Add File:` (create) and `*** Update File:` (modify)
- Always has single `input` field containing the complete patch string
- File path is embedded in the patch header (not a separate field)
- Patch format is consistent: Begin → Header → Hunks → End

### Step 2: Define Provider-Specific Struct

`crates/agtrace-providers/src/codex/tools.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchArgs {
    /// Raw patch content including Begin/End markers
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPatch {
    pub operation: PatchOperation,
    pub file_path: String,
    pub raw_patch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchOperation {
    Add,    // *** Add File:
    Update, // *** Update File:
}

impl ApplyPatchArgs {
    pub fn parse(&self) -> Result<ParsedPatch, String> {
        // Parse raw patch to extract operation and file_path
        for line in self.raw.lines() {
            if let Some(path) = line.strip_prefix("*** Add File: ") {
                return Ok(ParsedPatch {
                    operation: PatchOperation::Add,
                    file_path: path.trim().to_string(),
                    raw_patch: self.raw.clone(),
                });
            }
            if let Some(path) = line.strip_prefix("*** Update File: ") {
                return Ok(ParsedPatch {
                    operation: PatchOperation::Update,
                    file_path: path.trim().to_string(),
                    raw_patch: self.raw.clone(),
                });
            }
        }
        Err("Failed to parse patch: no file path header found".to_string())
    }
}
```

### Step 3: Map to Domain Model

`crates/agtrace-providers/src/codex/normalize.rs`:

```rust
fn normalize_codex_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    match tool_name.as_str() {
        "apply_patch" => {
            if let Ok(patch_args) = serde_json::from_value::<ApplyPatchArgs>(arguments.clone()) {
                match patch_args.parse() {
                    Ok(parsed) => {
                        match parsed.operation {
                            PatchOperation::Add => {
                                // File creation → FileWrite
                                return ToolCallPayload::FileWrite {
                                    name: tool_name,
                                    arguments: FileWriteArgs {
                                        file_path: parsed.file_path,
                                        content: parsed.raw_patch,
                                    },
                                    provider_call_id,
                                };
                            }
                            PatchOperation::Update => {
                                // File modification → FileEdit
                                return ToolCallPayload::FileEdit {
                                    name: tool_name,
                                    arguments: FileEditArgs {
                                        file_path: parsed.file_path,
                                        old_string: String::new(),
                                        new_string: parsed.raw_patch.clone(),
                                        replace_all: false,
                                    },
                                    provider_call_id,
                                };
                            }
                        }
                    }
                    Err(_) => {
                        // Parsing failed, fall back to generic
                    }
                }
            }
        }
        _ => {
            // Not a provider-specific tool
        }
    }

    // Fallback to generic normalization
    normalize_tool_call(tool_name, arguments, provider_call_id)
}
```

### Step 4: Test Both Layers

Provider layer test (`crates/agtrace-providers/src/codex/tools.rs`):

```rust
#[test]
fn test_parse_add_file_patch() {
    let args = ApplyPatchArgs {
        raw: r#"*** Begin Patch
*** Add File: docs/test.md
+# Test Document
*** End Patch"#.to_string(),
    };

    let parsed = args.parse().unwrap();
    assert_eq!(parsed.operation, PatchOperation::Add);
    assert_eq!(parsed.file_path, "docs/test.md");
}
```

Normalization layer test (`crates/agtrace-providers/src/codex/normalize.rs`):

```rust
#[test]
fn test_normalize_apply_patch_add_file() {
    let raw_patch = r#"*** Begin Patch
*** Add File: docs/new_file.md
+# New File
*** End Patch"#;

    let arguments = serde_json::json!({ "raw": raw_patch });
    let payload = normalize_codex_tool_call(
        "apply_patch".to_string(),
        arguments,
        Some("call_456".to_string()),
    );

    match payload {
        ToolCallPayload::FileWrite { name, arguments, .. } => {
            assert_eq!(name, "apply_patch");
            assert_eq!(arguments.file_path, "docs/new_file.md");
        }
        _ => panic!("Expected FileWrite variant"),
    }
}
```

## Benefits

1. **Clear Separation**: Provider quirks isolated in provider layer
2. **Type Safety**: Compile-time guarantees for schema compatibility
3. **Debuggability**: Raw data preserved for inspection
4. **Extensibility**: Easy to add new providers or tools
5. **Testability**: Each layer can be tested independently

## Future Work

### Immediate Next Steps

1. Claude Code `Edit` tool
2. Gemini `replace` tool
3. Other file operation tools (Read, Glob, etc.)

### Long-term Goals

1. All providers, all tools
2. Automated schema extraction from real data
3. Schema compatibility verification tests
4. Provider-agnostic query interface over normalized data

## Verification

### 1. Unit Tests

```bash
cargo test -p agtrace-providers test_parse test_normalize_apply_patch
```

Expected: All tests passing (parsing + normalization)

### 2. Real Data Verification with `--raw`

**ALWAYS use `--raw` to verify normalization correctness**:

```bash
cargo build --release

# Verify Update File → FileEdit mapping
./target/release/agtrace lab grep '"name":"apply_patch"' --raw --limit 1
```

**What to check**:

```json
{
  "content": {
    "name": "apply_patch",
    "arguments": {
      "file_path": "crates/...",  // ← ✅ Extracted correctly?
      "old_string": "",
      "new_string": "*** Begin Patch\n*** Update File: crates/...",  // ← ✅ Raw patch preserved?
      "replace_all": false
    }
  },
  "metadata": {
    "payload": {
      "input": "*** Begin Patch\n*** Update File: crates/...",  // ← Original raw data
      "name": "apply_patch"
    }
  }
}
```

**Verification checklist**:
- ✅ `content.arguments.file_path` matches the file in `metadata.payload.input` header
- ✅ `content.arguments.new_string` contains the full raw patch
- ✅ `content.name` is still `apply_patch` (original tool name preserved)

**Check both operation types**:

```bash
# Update File → FileEdit
./target/release/agtrace lab grep 'Update File:' --raw --limit 1
# Expected: content.arguments has file_path, old_string, new_string, replace_all

# Add File → FileWrite
./target/release/agtrace lab grep 'Add File:' --raw --limit 1
# Expected: content.arguments has file_path, content (with full patch)
```

### 3. Compare Before/After

**Without `--raw` (only shows after)**:
```bash
./target/release/agtrace lab grep '"name":"apply_patch"' --json --limit 1
# ❌ Cannot see provider's original format
# ❌ Cannot verify extraction logic
```

**With `--raw` (shows both before & after)**:
```bash
./target/release/agtrace lab grep '"name":"apply_patch"' --raw --limit 1
# ✅ See metadata.payload (before normalization)
# ✅ See content.arguments (after normalization)
# ✅ Can verify: did we extract file_path correctly from the patch header?
```
