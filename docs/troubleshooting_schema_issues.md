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

Run `agtrace diagnose` to identify files with parsing errors:

```bash
$ agtrace diagnose

=== Diagnose Results ===

Provider: Codex
  Total files scanned: 10
  Successfully parsed: 5 (50.0%)
  Parse failures: 5 (50.0%)

  Failure breakdown:
  ✗ missing_field (network_access): 2 files
    Example: /Users/.../rollout-2025-12-04T22-23-36-019ae988-502c-7533-a763-5c796e30804c.jsonl
    Reason: Missing required field: network_access

Provider: Gemini
  Total files scanned: 10
  Successfully parsed: 7 (70.0%)
  Parse failures: 3 (30.0%)

  Failure breakdown:
  ✗ parse_error: 3 files
    Example: /Users/.../9126edde.../logs.json
    Reason: invalid type: map, expected a string at line 2 column 2
```

**Key Information:**
- Which providers have problems
- How many files are affected
- Error types and examples
- File paths for investigation

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

Based on the investigation, update the schema definition in `src/providers/gemini/schema.rs`:

**Before:**
```rust
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)?;

    // Expects single session object
    let session: GeminiSession = serde_json::from_str(&text)?;

    Ok(normalize_gemini_session(&session))
}
```

**After:**
```rust
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)?;

    // Try parsing as session object first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        return Ok(normalize_gemini_session(&session));
    }

    // Fallback: Try parsing as array of messages (older format)
    if let Ok(messages) = serde_json::from_str::<Vec<GeminiMessage>>(&text) {
        // Extract project hash from file path
        let project_hash = extract_project_hash_from_path(path)?;

        // Create synthetic session
        let session = GeminiSession {
            session_id: "unknown".to_string(),
            project_hash,
            start_time: messages.first()
                .map(|m| m.timestamp().to_string())
                .unwrap_or_default(),
            last_updated: messages.last()
                .map(|m| m.timestamp().to_string())
                .unwrap_or_default(),
            messages,
        };

        return Ok(normalize_gemini_session(&session));
    }

    anyhow::bail!("Failed to parse Gemini file in any known format")
}
```

### Step 6: Verify the Fix

After updating the schema, rebuild and test:

```bash
# Rebuild
$ cargo build --release

# Test the specific file
$ agtrace validate /Users/.../logs.json

File: /Users/.../logs.json
Provider: gemini (auto-detected)
Status: ✓ Valid

Parsed successfully:
  - Session ID: f0a689a6-b0ac-407f-afcc-4fafa9e14e8a
  - Events extracted: 15
  - Project: /Users/zawakin/...

# Re-run full diagnosis
$ agtrace diagnose --provider gemini

Provider: Gemini
  Total files scanned: 10
  Successfully parsed: 10 (100.0%)

All files parsed successfully!
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
agtrace diagnose                    # 1. Find problems
agtrace inspect <file> --lines 30   # 2. See actual data
agtrace schema <provider>           # 3. See expected format
agtrace validate <file>             # 4. Get detailed error
# (edit schema code)                # 5. Fix the schema
cargo build --release               # 6. Rebuild
agtrace validate <file>             # 7. Test fix
agtrace diagnose --provider <name>  # 8. Verify all files
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

1. **`diagnose`** finds all problems deterministically
2. **`inspect`** shows raw file content with line numbers
3. **`schema`** displays expected format
4. **`validate`** gives detailed errors with suggestions
5. Fix code, rebuild, validate

This creates a reproducible, deterministic debugging loop that's easy to follow and document.
