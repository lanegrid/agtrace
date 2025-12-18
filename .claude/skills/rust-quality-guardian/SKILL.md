---
name: rust-quality-guardian
description: Enforce Rust code quality standards with cargo fmt, clippy, and tests. Use when making code changes, before commits, or when asked to check/verify quality. Automatically handles snapshot test workflow.
allowed-tools: Bash, Read, Grep
---

# Rust Quality Guardian

## Purpose

Automates the quality-driven approach defined in CLAUDE.md by running cargo fmt, clippy, and tests. Ensures code meets project standards before committing.

## When to use

- After making any code changes
- Before creating commits
- When explicitly asked to "check quality", "run tests", or "verify code"
- After modifying snapshot tests

## Quality check workflow

### 1. Format check

```bash
cargo fmt --check
```

If formatting issues found, run:
```bash
cargo fmt
```

### 2. Linting check

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Address all warnings - do not suppress them without fixing root cause.

### 3. Run tests

```bash
cargo test --workspace
```

### 4. Snapshot test handling

When tests involve snapshot updates (insta):

**Step 4.1:** Accept snapshots if tests show differences
```bash
cargo insta accept
```

**Step 4.2:** Review changes with git diff
```bash
git diff
```

**Step 4.3:** Decision making
- **If issues found**: Fix implementation, rerun tests
- **If no issues**: Include snapshot changes in same commit as implementation

## Output format

Provide results in this structure:

1. **Format Check**: Pass/Fail with details
2. **Clippy Check**: Pass/Fail with warning count
3. **Test Results**: Pass/Fail with failure details
4. **Snapshot Changes**: If applicable, summary of diff
5. **Recommendation**: Next steps or approval to commit

## CLAUDE.md compliance

This skill enforces:
- Quality-driven approach (fmt, clippy, tests before commit)
- Snapshot test rule: `cargo insta accept` → `git diff` → verify → commit together
- Complete solutions over partial fixes
- No warning suppression without root cause fix

## Build commands reference

```bash
# Full workspace build
cargo build --workspace

# Release build (for CLI testing)
cargo build --release

# Test with insta review mode
cargo insta test --review

# Specific crate test
cargo test -p agtrace-engine

# Clean build
cargo clean && cargo build
```

## Common issues

### Clippy warnings
- Do not use `#[allow(clippy::...)]` without justification
- Fix the root cause, not the symptom
- Consider if warning indicates design issue

### Test failures
- Check if snapshot needs update
- Verify test expectations match implementation
- Review error messages for clues

### Snapshot conflicts
- Review git diff carefully
- Ensure changes align with implementation intent
- Do not blindly accept all snapshots

## Best practices

1. Run checks frequently during development
2. Address issues immediately, not in batch
3. Read error messages completely before fixing
4. For workspace projects, test all crates
5. Keep commits atomic: implementation + snapshots together
