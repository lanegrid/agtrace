# Troubleshooting Schema Compatibility Issues

This guide shows how to diagnose and fix schema compatibility issues using agtrace's diagnostic commands.

## Overview

When provider log formats change between versions, agtrace may no longer understand some of their records. This guide demonstrates a systematic workflow to:
1. Discover problems
2. Inspect actual data
3. Compare with expected schema
4. Fix the schema definition
5. Verify the fix

### How decoding fails (lenient per line)

Providers decode each file **line by line** and leniently. A line that is not JSON, has an
unknown record kind, or fails the typed deserialize is **counted** in the file's
`ParseDiagnostics` (`invalid_json`, `schema_mismatch[kind]`, `unknown_kinds[kind]`) and
skipped. The rest of the file still decodes; only I/O errors fail a file. In `doctor`
output, a "failure" is a file with invalid-JSON or schema-mismatch lines. The reason names
the count and the first offending line.

Only the supported formats are modeled: Claude Code ≥ 2.1.24x and Codex ≥ 0.153. Lines from
older formats show up as diagnostics. That is expected, and there is no fix for them.

The record structs live in `crates/agtrace-providers/src/<provider>/records.rs`, and the
per-line logic in `decoder.rs`. Claude has two more files: `tags.rs` for XML-ish user-text
tags and `sidecar.rs` for team config, the registry and meta files. Codex has two as well:
`collab.rs` for multi-agent tools and `exec.rs` for exec sub-actions.

> The example outputs below predate lenient decoding and show whole-file failures; the
> workflow is the same.

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
    Example: ~/.claude/projects/-work-demo-project/00000000-0000-4000-8000-000000000001.jsonl
    Reason: No events extracted from file

    ... and 15 more files

Provider: Codex
  Total files scanned: 81
  Successfully parsed: 48 (59.3%)
  Parse failures: 33 (40.7%)

  Failure breakdown:
  ✗ missing_field (model_provider): 19 files
    Example: ~/.codex/sessions/.../rollout-2025-10-28T16-24-01-01900000-0000-7000-8000-000000000001.jsonl
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
$ agtrace doctor inspect ~/.codex/sessions/.../rollout-2025-12-04...jsonl --lines 20
```

Compare the raw records with the typed structs in
`crates/agtrace-providers/src/<provider>/records.rs` to identify the gap
(missing field, changed type, new record kind, etc.).

### Step 3: Validate Specific Files

Use `agtrace doctor check` to get detailed error information and suggestions:

```bash
$ agtrace doctor check ~/.codex/sessions/.../rollout-2025-12-04...jsonl
```

### Step 4: Fix the Schema

Update the provider's record definitions (`records.rs`) and, if needed, the
decoding logic (`decoder.rs`). Add a failing test first; a small line in the synthetic
fixture tree or a `synth` builder is usually enough (see
[Testing](testing_with_testworld.md#synthetic-fixtures)). See [Common Patterns](#common-patterns)
below and the [Codex SandboxPolicy example](#example-fixing-codex-sandboxpolicy)
for a complete walkthrough.

### Step 5: Verify the Fix

After updating the schema, rebuild and re-run the diagnosis:

```bash
$ cargo build --release
$ agtrace doctor check ~/.codex/sessions/.../rollout-2025-12-04...jsonl
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

### Pattern 3: New Record Kind

**Symptom:** `unknown_kinds["<kind>"]` grows (visible with `doctor check`, and in the
`watch` status bar diagnostic count).

**Solution:** Decide whether the kind carries information worth an event:
- **Yes:** Add a typed struct in `records.rs` and a decoder branch that emits the
  matching `EventPayload`.
- **No:** Add it to the decoder's ignored kinds. It is then counted in `ignored_kinds`, not
  `unknown_kinds`.

Old formats are not supported side by side. When a provider changes its format
incompatibly, the decoder follows the new format and the minimum supported version is raised.

## Decision-Making Framework

When you encounter a schema issue, ask:

### 1. Is this a one-off corrupted line?
- **Yes:** Nothing to do; lenient decoding already skips and counts it
- **No:** Fix the record definition

### 2. Is the file from a supported version?
- **No:** (Claude Code < 2.1.24x, Codex < 0.153) Not supported; leave it
- **Yes:** Update `records.rs` / `decoder.rs`, using `Option` and `#[serde(default)]` for
  fields that are not needed to identify the record

### 3. Can metadata be recovered?
- **Yes:** Extract from file path or synthesize reasonable defaults
- **No:** Use placeholder values like `"unknown"` or `None`

### 4. Is backwards compatibility important?
Within the supported versions, yes: keep fields optional. Across the minimum-version
boundary, no: older formats are not decoded.

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
# (compare with <provider>/records.rs)       # 3. See expected format
agtrace doctor check <file>                  # 4. Get detailed error
# (edit records.rs / decoder.rs)             # 5. Fix the decoder
cargo build --release                        # 6. Rebuild
agtrace doctor check <file>                  # 7. Test fix
agtrace doctor run --provider <name>         # 8. Verify all files

# For verbose output showing all problematic files:
agtrace doctor run --verbose
```

## Example: Fixing Codex SandboxPolicy

This historical example (from Codex 0.53 / 0.63, both of which are no longer supported)
shows the complete process of fixing a schema issue. The `Detailed` variant below has since
been removed.

### Problem Discovery
```bash
$ agtrace doctor run --provider codex

Provider: Codex
  Parse failures: 5 (50.0%)

  ✗ missing_field (network_access): 2 files
    Example: ~/.codex/sessions/.../rollout-2025-12-04...jsonl
```

### Investigation
```bash
$ agtrace doctor inspect ~/.codex/sessions/.../rollout-2025-12-04...jsonl --lines 10

     5  ...{"type":"turn_context","payload":{..,"sandbox_policy":{"type":"read-only"},...

$ agtrace doctor inspect ~/.codex/sessions/.../rollout-2025-11-03...jsonl --lines 10

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
3. **`records.rs`** holds the expected format
4. **`doctor check`** gives detailed errors with suggestions
5. Fix code, rebuild, validate

**Key principle:** `doctor run` checks **all files comprehensively** to ensure no issues are missed. This is critical because:
- Schema changes can occur at any point in log history
- Sampling might miss older format versions
- Complete coverage ensures production-ready schema definitions

This creates a reproducible, deterministic debugging loop that's easy to follow and document.
