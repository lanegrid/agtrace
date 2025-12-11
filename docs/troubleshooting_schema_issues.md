# Troubleshooting Schema Compatibility Issues

This guide shows how to diagnose and fix schema compatibility issues using agtrace's diagnostic commands.

## Overview

When provider log formats change between versions, agtrace may fail to parse them. This guide demonstrates a systematic workflow to:
1. Discover problems
2. Inspect actual data
3. Compare with expected schema
4. Fix the schema definition
5. Verify the fix

## Workflow

### Step 1: Discover Problems

Run `agtrace diagnose` to identify files with parsing errors across **all files**:

```bash
$ agtrace diagnose

=== Diagnose Results ===

Provider: Claude
  Total files scanned: 329
  Successfully parsed: 312 (94.8%)
  Parse failures: 17 (5.2%)

  Failure breakdown:
  ✗ empty_file: 16 files
    Example: /Users/.../a50cd2c1-d8df-4ae7-ae5d-887009d66940.jsonl
    Reason: No events extracted from file

    ... and 15 more files

Provider: Codex
  Total files scanned: 81
  Successfully parsed: 48 (59.3%)
  Parse failures: 33 (40.7%)

  Failure breakdown:
  ✗ missing_field (model_provider): 19 files
    Example: /Users/.../rollout-2025-10-28T16-24-01-019a29b3-d031-7b31-9f2d-8970fd673604.jsonl
    Reason: Missing required field: model_provider

    ... and 18 more files

Provider: Gemini
  Total files scanned: 12
  Successfully parsed: 11 (91.7%)
  Parse failures: 1 (8.3%)

  Failure breakdown:
  ✗ empty_file: 1 files
    Example: /Users/.../a7e6a102cb8d98a9665a366914d81fc84cb6e3264be0970c66e14288b15761d7/logs.json
    Reason: No events extracted from file
```

**Key Information:**
- Which providers have problems
- Total number of files checked (comprehensive, not sampled)
- Error types and examples
- File paths for investigation

**Note:** `diagnose` checks **all files** to ensure no issues are missed. This is critical for catching version-specific format changes across your entire log history.

### Step 2: Inspect Actual Data

Use `agtrace inspect` to view the raw content of problematic files:

```bash
$ agtrace inspect /Users/.../logs.json --lines 20

File: /Users/.../logs.json
Lines: 1-20 (total: 23 lines)
────────────────────────────────────────
     1  [
     2    {
     3      "sessionId": "f0a689a6-b0ac-407f-afcc-4fafa9e14e8a",
     4      "messageId": 0,
     5      "type": "user",
     6      "message": "add myapp directory...",
     7      "timestamp": "2025-12-09T19:51:09.325Z"
     8    },
     9    {
    10      "sessionId": "f0a689a6-b0ac-407f-afcc-4fafa9e14e8a",
     ...
────────────────────────────────────────
```

**Observations:**
- File contains an **array** of messages
- Each message has `sessionId`, `messageId`, `type`, `message`, `timestamp`
- No root-level `session_id`, `project_hash`, or `messages` field

### Step 3: Compare with Expected Schema

Use `agtrace schema` to see what structure agtrace expects:

```bash
$ agtrace schema gemini

Provider: Gemini
Schema version: unknown

Root structure (JSON - single session object):
  GeminiSession:
    sessionId: String
    projectHash: String
    startTime: String
    lastUpdated: String
    messages: [GeminiMessage]

GeminiMessage (enum):
  - User:
      id: String
      timestamp: String
      content: String
  ...
```

**Gap Identified:**
- **Expected:** Root object with metadata + messages array
- **Actual:** Direct array of messages without session metadata

### Step 4: Validate Specific Files

Use `agtrace validate` to get detailed error information and suggestions:

```bash
$ agtrace validate /Users/.../logs.json

File: /Users/.../logs.json
Provider: gemini (auto-detected)
Status: ✗ Invalid

Parse error:
  Failed to parse Gemini JSON: invalid type: map, expected a string at line 2 column 2

Suggestion:
  The field type in the schema may not match the actual data format.
  Use 'agtrace inspect /Users/.../logs.json' to examine the actual structure.
  Use 'agtrace schema gemini' to see the expected format.

Next steps:
  1. Examine the actual data:
       agtrace inspect /Users/.../logs.json --lines 20
  2. Compare with expected schema:
       agtrace schema gemini
  3. Update schema definition if needed
```

### Step 5: Fix the Schema

Based on the investigation, update the schema definition in `src/providers/gemini/schema.rs` and `src/providers/gemini/io.rs`:

**Step 5.1: Add legacy format schema**

In `src/providers/gemini/schema.rs`, add a struct for the legacy format:

```rust
// Legacy format: array of simple messages
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGeminiMessage {
    pub session_id: String,
    pub message_id: u32,
    #[serde(rename = "type")]
    pub message_type: String,
    pub message: String,
    pub timestamp: String,
}
```

**Step 5.2: Update parser to handle both formats**

In `src/providers/gemini/io.rs`, update `normalize_gemini_file`:

```rust
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    // Try new format (session object) first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        return Ok(normalize_gemini_session(&session));
    }

    // Fallback: Try legacy format (array of messages)
    if let Ok(legacy_messages) = serde_json::from_str::<Vec<LegacyGeminiMessage>>(&text) {
        return normalize_legacy_format(path, legacy_messages);
    }

    anyhow::bail!("Failed to parse Gemini file in any known format: {}", path.display())
}

fn normalize_legacy_format(path: &Path, messages: Vec<LegacyGeminiMessage>) -> Result<Vec<AgentEventV1>> {
    // Extract session_id from first message
    let session_id = messages.first()
        .map(|m| m.session_id.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract project_hash from file path
    let project_hash = extract_project_hash_from_path(path)?;

    // Convert legacy messages to new format
    let converted_messages: Vec<GeminiMessage> = messages.iter().map(|msg| {
        GeminiMessage::User(UserMessage {
            id: msg.message_id.to_string(),
            timestamp: msg.timestamp.clone(),
            content: msg.message.clone(),
        })
    }).collect();

    // Create synthetic session
    let session = GeminiSession {
        session_id,
        project_hash,
        start_time: messages.first().map(|m| m.timestamp.clone()).unwrap_or_default(),
        last_updated: messages.last().map(|m| m.timestamp.clone()).unwrap_or_default(),
        messages: converted_messages,
    };

    Ok(normalize_gemini_session(&session))
}
```

### Step 6: Verify the Fix

After updating the schema, rebuild and test:

```bash
# Rebuild
$ cargo build --release

# Test the specific file
$ agtrace validate /Users/.../9126eddec7f67e038794657b4d517dd9cb5226468f30b5ee7296c27d65e84fde/logs.json

File: /Users/.../9126eddec7f67e038794657b4d517dd9cb5226468f30b5ee7296c27d65e84fde/logs.json
Provider: gemini (auto-detected)
Status: ✓ Valid

Parsed successfully:
  - Session ID: f0a689a6-b0ac-407f-afcc-4fafa9e14e8a
  - Events extracted: 3
  - Event breakdown:
      UserMessage: 3

# Re-run full diagnosis
$ agtrace diagnose --provider gemini

Provider: Gemini
  Total files scanned: 12
  Successfully parsed: 11 (91.7%)
  Parse failures: 1 (8.3%)

  Failure breakdown:
  ✗ empty_file: 1 files
    Example: /Users/.../a7e6a102cb8d98a9665a366914d81fc84cb6e3264be0970c66e14288b15761d7/logs.json
    Reason: No events extracted from file

# The remaining failure is a legitimately empty file, not a schema issue
```

## Common Patterns

### Pattern 1: Missing Optional Field

**Symptom:**
```
✗ missing_field (network_access): 2 files
```

**Solution:** Make the field optional with `#[serde(default)]`

```rust
#[derive(Debug, Deserialize)]
pub struct SandboxPolicy {
    #[serde(default)]  // ← Add this
    pub network_access: Option<bool>,
}
```

### Pattern 2: Type Changed Between Versions

**Symptom:**
```
✗ type_mismatch (source): 3 files
  Expected: String
  Found: {"subagent": "review"}
```

**Solution:** Use `serde_json::Value` or an enum

```rust
pub struct Payload {
    pub source: Value,  // Can be string or object
}
```

Or:

```rust
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Source {
    String(String),
    Object { subagent: String },
}
```

### Pattern 3: Multiple Format Versions

**Symptom:**
```
✗ parse_error: Files use different root structures
```

**Solution:** Try multiple parsing strategies

```rust
pub fn normalize_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)?;

    // Try format v2
    if let Ok(data) = serde_json::from_str::<FormatV2>(&text) {
        return Ok(normalize_v2(data));
    }

    // Fallback to format v1
    if let Ok(data) = serde_json::from_str::<FormatV1>(&text) {
        return Ok(normalize_v1(data));
    }

    anyhow::bail!("Unknown format")
}
```

## Decision-Making Framework

When you encounter a schema issue, ask:

### 1. Is this a one-off corrupted file?
- **Yes:** Skip it (use `continue` in scan)
- **No:** Fix the schema

### 2. Which format is more common?
- **New format is dominant:** Update schema, add fallback for old
- **Old format is dominant:** Keep schema, add support for new
- **Both common:** Use enum or untagged union

### 3. Can metadata be recovered?
- **Yes:** Extract from file path or synthesize reasonable defaults
- **No:** Use placeholder values like `"unknown"` or `None`

### 4. Is backwards compatibility important?
- **Yes:** Keep both formats working
- **No:** Update schema, accept that old files may fail

## Tips

1. **Start with specific files:** Use `validate` before touching code
2. **Use version control:** Make small, testable changes
3. **Document format changes:** Add comments explaining version differences
4. **Test with real data:** Always validate with actual problem files
5. **Run full diagnosis:** Ensure fix doesn't break other files

## Quick Reference

```bash
# Full workflow in order
agtrace diagnose                    # 1. Find problems (checks ALL files)
agtrace inspect <file> --lines 30   # 2. See actual data
agtrace schema <provider>           # 3. See expected format
agtrace validate <file>             # 4. Get detailed error
# (edit schema code)                # 5. Fix the schema
cargo build --release               # 6. Rebuild
agtrace validate <file>             # 7. Test fix
agtrace diagnose --provider <name>  # 8. Verify all files

# For verbose output showing all problematic files:
agtrace diagnose --verbose
```

## Example: Fixing Codex SandboxPolicy

This example shows the complete process of fixing a real schema issue.

### Problem Discovery
```bash
$ agtrace diagnose --provider codex

Provider: Codex
  Parse failures: 5 (50.0%)

  ✗ missing_field (network_access): 2 files
    Example: /Users/.../rollout-2025-12-04...jsonl
```

### Investigation
```bash
$ agtrace inspect /Users/.../rollout-2025-12-04...jsonl --lines 10

     5  ...{"type":"turn_context","payload":{..,"sandbox_policy":{"type":"read-only"},...

$ agtrace inspect /Users/.../rollout-2025-11-03...jsonl --lines 10

     5  ...{"type":"turn_context","payload":{..,"sandbox_policy":{"mode":"workspace-write","network_access":false},...
```

**Observation:** Two different formats!
- v0.63+: `{"type": "read-only"}`
- v0.53: `{"mode": "workspace-write", "network_access": false}`

### Schema Fix
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]  // ← Try each variant in order
pub enum SandboxPolicy {
    // New format (v0.63+)
    Simple {
        #[serde(rename = "type")]
        policy_type: String,
    },
    // Old format (v0.53)
    Detailed {
        mode: String,
        #[serde(default)]
        network_access: Option<bool>,
        #[serde(default)]
        exclude_tmpdir_env_var: bool,
        #[serde(default)]
        exclude_slash_tmp: bool,
    },
}
```

### Verification
```bash
$ cargo build --release
$ agtrace diagnose --provider codex

Provider: Codex
  Successfully parsed: 10 (100.0%)

All files parsed successfully!
```

## Summary

The diagnostic workflow eliminates the need for manual file inspection with UNIX tools:

1. **`diagnose`** finds all problems deterministically by checking **every file** (no sampling)
2. **`inspect`** shows raw file content with line numbers
3. **`schema`** displays expected format
4. **`validate`** gives detailed errors with suggestions
5. Fix code, rebuild, validate

**Key principle:** `diagnose` checks **all files comprehensively** to ensure no issues are missed. This is critical because:
- Schema changes can occur at any point in log history
- Sampling might miss older format versions
- Complete coverage ensures production-ready schema definitions

This creates a reproducible, deterministic debugging loop that's easy to follow and document.
