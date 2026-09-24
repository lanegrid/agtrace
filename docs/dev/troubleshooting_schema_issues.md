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

Run `agtrace doctor run` to identify files with parsing errors across **all files**:

```bash
$ agtrace doctor run

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

```

**Key Information:**
- Which providers have problems
- Total number of files checked (comprehensive, not sampled)
- Error types and examples
- File paths for investigation

**Note:** `diagnose` checks **all files** to ensure no issues are missed. This is critical for catching version-specific format changes across your entire log history.

### Step 2: Inspect Actual Data

Use `agtrace doctor inspect` to view the raw content of a problematic file:

```bash
$ agtrace doctor inspect /Users/.../rollout-2025-12-04...jsonl --lines 20
```

Compare the raw records with the schema structs in
`crates/agtrace-providers/src/<provider>/schema.rs` to identify the gap
(missing field, changed type, new record kind, etc.).

### Step 3: Validate Specific Files

Use `agtrace doctor check` to get detailed error information and suggestions:

```bash
$ agtrace doctor check /Users/.../rollout-2025-12-04...jsonl
```

### Step 4: Fix the Schema

Update the provider's schema definitions (`schema.rs`) and, if needed, the
normalization logic (`parser.rs` / `io.rs`). See [Common Patterns](#common-patterns)
below and the [Codex SandboxPolicy example](#example-fixing-codex-sandboxpolicy)
for a complete walkthrough.

### Step 5: Verify the Fix

After updating the schema, rebuild and re-run the diagnosis:

```bash
$ cargo build --release
$ agtrace doctor check /Users/.../rollout-2025-12-04...jsonl
$ agtrace doctor run --provider codex
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
agtrace doctor run                           # 1. Find problems (checks ALL files)
agtrace doctor inspect <file> --lines 30     # 2. See actual data
agtrace provider schema <provider>           # 3. See expected format
agtrace doctor check <file>                  # 4. Get detailed error
# (edit schema code)                         # 5. Fix the schema
cargo build --release                        # 6. Rebuild
agtrace doctor check <file>                  # 7. Test fix
agtrace doctor run --provider <name>         # 8. Verify all files

# For verbose output showing all problematic files:
agtrace doctor run --verbose
```

## Example: Fixing Codex SandboxPolicy

This example shows the complete process of fixing a real schema issue.

### Problem Discovery
```bash
$ agtrace doctor run --provider codex

Provider: Codex
  Parse failures: 5 (50.0%)

  ✗ missing_field (network_access): 2 files
    Example: /Users/.../rollout-2025-12-04...jsonl
```

### Investigation
```bash
$ agtrace doctor inspect /Users/.../rollout-2025-12-04...jsonl --lines 10

     5  ...{"type":"turn_context","payload":{..,"sandbox_policy":{"type":"read-only"},...

$ agtrace doctor inspect /Users/.../rollout-2025-11-03...jsonl --lines 10

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
$ agtrace doctor run --provider codex

Provider: Codex
  Successfully parsed: 10 (100.0%)

All files parsed successfully!
```

## Summary

The diagnostic workflow eliminates the need for manual file inspection with UNIX tools:

1. **`doctor run`** finds all problems deterministically by checking **every file** (no sampling)
2. **`doctor inspect`** shows raw file content with line numbers
3. **`provider schema`** displays expected format
4. **`doctor check`** gives detailed errors with suggestions
5. Fix code, rebuild, validate

**Key principle:** `doctor run` checks **all files comprehensively** to ensure no issues are missed. This is critical because:
- Schema changes can occur at any point in log history
- Sampling might miss older format versions
- Complete coverage ensures production-ready schema definitions

This creates a reproducible, deterministic debugging loop that's easy to follow and document.
