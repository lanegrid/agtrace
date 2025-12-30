---
name: releaser
description: Release Management - Automates safe release preparation with validation, testing, and rollback procedures. Activates when preparing releases or bumping versions.
---

# Releaser Skill

**Trigger**: When the user requests to prepare a release, create a release, or bump version.

**Objective**: Safely and robustly prepare a new release following strict validation and rollback procedures.

---

## Core Principles

1. **Atomicity**: All changes (version bump, CHANGELOG, commit, tag) must succeed or fail together
2. **Validation First**: Never proceed if any validation step fails
3. **Idempotency**: Script can be safely retried after fixing issues
4. **Transparency**: Always show what will be done before doing it
5. **Rollback Safety**: Always provide clear rollback instructions

---

## Pre-Release Checklist (MANDATORY)

Before starting ANY release preparation, verify:

- [ ] Currently on `main` branch
- [ ] Local `main` is up-to-date with `origin/main`
- [ ] Working directory is clean (or user explicitly approves proceeding with uncommitted changes)
- [ ] All tests pass locally
- [ ] No open PR blockers or failing CI on main

**Command to verify:**
```bash
git checkout main
git pull origin main
git status
cargo test --workspace
```

---

## Release Preparation Process

### Step 1: Determine Version Number

**Rules:**
- Version MUST follow SemVer: `MAJOR.MINOR.PATCH`
- Increment rules:
  - `PATCH`: Bug fixes, documentation updates, internal refactoring
  - `MINOR`: New features, backwards-compatible API additions
  - `MAJOR`: Breaking changes (0.x.y is pre-1.0, so breaking changes increment MINOR)

**Anti-pattern:**
```bash
# ❌ NEVER skip versions
0.1.14 → 0.1.16  # Missing 0.1.15

# ❌ NEVER release same version twice
0.1.14 → 0.1.14  # Must be different
```

**Validation:**
```bash
# Version must be newer than current
CURRENT=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "agtrace") | .version')
# NEW_VERSION must be > CURRENT
```

### Step 2: Run Automated Release Script

**ONLY use the official script:**
```bash
./scripts/prepare-release.sh <NEW_VERSION>
```

**The script performs these steps (DO NOT do manually):**

1. **Validate environment** (git status, version format)
2. **Run full test suite** (`cargo test --workspace`)
3. **Validate README examples** (`./scripts/check-readme-sync.sh`)
4. **Update version numbers** (Cargo.toml workspace and all crates)
5. **Generate CHANGELOG** (using git-cliff from last tag to HEAD)
6. **Run code quality checks** (`cargo fmt`, `cargo clippy`)
7. **Create atomic commit and tag**

**Dry-run first (RECOMMENDED):**
```bash
./scripts/prepare-release.sh --dry-run <NEW_VERSION>
```

### Step 3: Verification Before Push

**CRITICAL: Review the commit before pushing**

```bash
# 1. Check the commit content
git show HEAD

# 2. Verify CHANGELOG entries are correct
git diff HEAD~1 CHANGELOG.md

# 3. Verify version numbers updated correctly
git diff HEAD~1 Cargo.toml | grep version

# 4. Ensure tag matches version
git tag -l | tail -1  # Should be v<NEW_VERSION>
```

**Checklist:**
- [ ] Commit message: `chore: bump version to X.Y.Z and update CHANGELOG`
- [ ] Tag created: `vX.Y.Z`
- [ ] CHANGELOG has new version section with today's date
- [ ] All workspace versions updated consistently
- [ ] No unintended changes in the commit

### Step 4: Push to Trigger Release

**Two-step push (ATOMIC):**
```bash
# Push commit and tag together to ensure atomicity
git push origin main && git push origin v<NEW_VERSION>
```

**If either push fails:**
```bash
# Undo both (rollback procedure below)
git reset --hard HEAD~1
git tag -d v<NEW_VERSION>
git push origin main  # Try again
```

### Step 5: Monitor CI/CD Pipeline

**After pushing, immediately check:**
```bash
gh run list --limit 1
gh run watch <RUN_ID>
```

**What to watch for:**
- [ ] All platform builds succeed (Linux, macOS, Windows - x86_64, ARM64)
- [ ] crates.io publish succeeds for all crates
- [ ] GitHub Release created with artifacts
- [ ] No errors in workflow logs

**Typical duration:** 8-12 minutes

### Step 6: Post-Release Validation

**Verify on crates.io:**
```bash
# Check that new version is published
cargo search agtrace --limit 1
# Output should show: agtrace = "X.Y.Z"
```

**Verify GitHub Release:**
```bash
gh release view v<NEW_VERSION>
```

**Checklist:**
- [ ] crates.io shows new version
- [ ] GitHub Release page has binaries for all platforms
- [ ] Release notes (CHANGELOG excerpt) are present
- [ ] All checksums/signatures present

---

## Error Handling & Rollback

### If Script Fails During Preparation

**Scenario: `prepare-release.sh` fails at any step**

```bash
# The script is designed to fail-fast
# Review error message and fix the issue
# Example: Test failures
./scripts/prepare-release.sh 0.1.15
# → "Step 2/7: Running tests" fails

# Fix the issue
cargo fix
cargo test --workspace

# Retry from scratch (script is idempotent)
./scripts/prepare-release.sh 0.1.15
```

**Common failure points:**
- Tests fail → Fix code, retry
- README doctest fails → Update README examples, retry
- Clippy warnings → Fix warnings, retry
- Working directory dirty → Commit or stash changes, retry

### If You Need to Undo After Commit/Tag Created

**Before pushing:**
```bash
# Safe: Just remove local commit and tag
git reset --hard HEAD~1
git tag -d v<NEW_VERSION>

# Now you can retry or fix issues
./scripts/prepare-release.sh <NEW_VERSION>
```

**After pushing (DANGEROUS - use with extreme caution):**
```bash
# Only if absolutely necessary and no one has pulled yet
git push origin :refs/tags/v<NEW_VERSION>  # Delete remote tag
git push --force-with-lease origin main~1:main  # Revert commit

# This will break CI and anyone who pulled
# Better approach: Release a patch version with fixes
```

### If CI/CD Fails After Push

**Scenario: Push succeeded, but GitHub Actions fails**

1. **Check failure logs:**
   ```bash
   gh run view <FAILED_RUN_ID> --log-failed
   ```

2. **Common issues:**
   - **Build failure**: Platform-specific compilation issue
     → Fix in new commit, release patch version
   - **crates.io publish auth**: Token expired
     → Regenerate token in GitHub secrets, re-run workflow
   - **crates.io rate limit**: Too many publishes
     → Wait, then trigger manual publish: `gh workflow run release.yml`

3. **DO NOT delete the tag** - CI can be re-run:
   ```bash
   gh run rerun <FAILED_RUN_ID>
   ```

---

## Security & Safety Rules

### NEVER Do These:

❌ **Manual version editing**
```bash
# WRONG: Editing Cargo.toml by hand
vim Cargo.toml  # Easy to miss crates, create inconsistency
```
✅ Use: `./scripts/prepare-release.sh` (updates all crates atomically)

---

❌ **Force pushing to main**
```bash
# WRONG: Rewriting published history
git push --force origin main
```
✅ Use: Release a new patch version to fix issues

---

❌ **Deleting published releases**
```bash
# WRONG: Removing tags after crates.io publish
git push origin :refs/tags/v0.1.14
```
✅ Use: `cargo yank` for broken releases, publish new version

---

❌ **Skipping validation steps**
```bash
# WRONG: Bypassing tests
git commit --no-verify
```
✅ Use: Fix the issues that tests/lints are catching

---

❌ **Publishing from non-main branch**
```bash
# WRONG: Releasing from feature branch
git checkout feature-x
./scripts/prepare-release.sh 0.1.15
```
✅ Use: Only release from `main` after PR merge

---

## Rollback Procedures

### Complete Rollback Decision Tree

```
Release state?
│
├─ NOT PUSHED YET (commit/tag only local)
│   └─ SAFE: git reset --hard HEAD~1 && git tag -d v<VERSION>
│
├─ PUSHED BUT CI NOT STARTED
│   └─ RISKY: Delete remote tag, force-push main~1 (coordination required)
│
├─ CI RUNNING / FAILED
│   ├─ Not published to crates.io yet?
│   │   └─ SAFE: Delete tag, cancel workflow, force-push rollback
│   │
│   └─ Published to crates.io?
│       └─ PERMANENT: Cannot delete. Options:
│           ├─ cargo yank (if critically broken)
│           └─ Publish patch version (recommended)
│
└─ FULLY RELEASED & ANNOUNCED
    └─ PERMANENT: Release new version with fixes
```

### Rollback Script (for pre-push only)

```bash
#!/bin/bash
# scripts/rollback-release.sh
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <VERSION>"
    exit 1
fi

VERSION=$1

# Safety check: Ensure tag wasn't pushed
if git ls-remote --tags origin | grep -q "refs/tags/v$VERSION"; then
    echo "ERROR: Tag v$VERSION already pushed to remote!"
    echo "Cannot rollback safely. Consider 'cargo yank' or new release."
    exit 1
fi

# Safety check: Ensure last commit is version bump
LAST_MSG=$(git log -1 --pretty=%B)
if [[ ! $LAST_MSG =~ "bump version to $VERSION" ]]; then
    echo "ERROR: Last commit doesn't look like version bump"
    echo "Last commit: $LAST_MSG"
    exit 1
fi

# Rollback
git reset --hard HEAD~1
git tag -d "v$VERSION" 2>/dev/null || true

echo "✅ Rolled back v$VERSION (local only)"
```

---

## Emergency Procedures

### If a Critical Bug is Found in Released Version

**DO NOT delete the release. Instead:**

1. **Yank the broken version from crates.io** (prevents new installs):
   ```bash
   cargo yank --vers X.Y.Z agtrace
   cargo yank --vers X.Y.Z agtrace-sdk
   # ... for each published crate
   ```

2. **Immediately release a patch version** with the fix:
   ```bash
   # Fix the bug
   git checkout main
   # ... make fixes ...
   git commit -m "fix: critical bug in X"

   # Release patch
   ./scripts/prepare-release.sh X.Y.Z+1
   ```

3. **Update GitHub Release with warning**:
   ```bash
   gh release edit vX.Y.Z --notes "⚠️ DEPRECATED: Use vX.Y.Z+1 instead. This version has [critical bug]."
   ```

### If CI/CD Credentials Are Compromised

1. **Immediately rotate secrets in GitHub:**
   - Settings → Secrets → Actions → Update `CARGO_REGISTRY_TOKEN`

2. **Cancel any running workflows:**
   ```bash
   gh run list --status in_progress --json databaseId -q '.[].databaseId' | xargs -I {} gh run cancel {}
   ```

3. **Audit recent releases for tampering:**
   ```bash
   gh release list
   # Verify checksums match your local builds
   ```

---

## Checklist Template (Copy for Each Release)

```markdown
## Release vX.Y.Z Checklist

### Pre-Release
- [ ] On `main` branch, synced with remote
- [ ] Working directory clean
- [ ] All tests passing locally
- [ ] No failing CI on main

### Preparation
- [ ] Ran `./scripts/prepare-release.sh --dry-run X.Y.Z`
- [ ] Dry-run succeeded
- [ ] Ran `./scripts/prepare-release.sh X.Y.Z`
- [ ] Reviewed commit: `git show HEAD`
- [ ] Reviewed CHANGELOG: `git diff HEAD~1 CHANGELOG.md`
- [ ] Tag created: `git tag -l | tail -1` == vX.Y.Z

### Release
- [ ] Pushed: `git push origin main && git push origin vX.Y.Z`
- [ ] CI workflow started: `gh run list --limit 1`
- [ ] Monitored workflow: `gh run watch <ID>`
- [ ] All builds succeeded
- [ ] crates.io publish succeeded

### Validation
- [ ] Verified on crates.io: `cargo search agtrace`
- [ ] Verified GitHub Release: `gh release view vX.Y.Z`
- [ ] Binaries present for all platforms
- [ ] Checksums/signatures present

### Post-Release
- [ ] Announced release (if applicable)
- [ ] Updated docs.rs links (automatic)
- [ ] Closed milestone (if exists)
```

---

## Integration with AI Assistant

When user requests a release:

1. **Always start with confirmation:**
   ```
   I'll prepare release vX.Y.Z. This will:
   - Run full test suite
   - Validate README examples
   - Update all version numbers
   - Generate CHANGELOG
   - Create commit and tag

   Proceed? (y/n)
   ```

2. **Use dry-run first:**
   ```bash
   ./scripts/prepare-release.sh --dry-run X.Y.Z
   ```

3. **Show output and ask for confirmation:**
   ```
   Dry-run succeeded. Ready to create actual release?
   Review: <show dry-run output>

   Proceed? (y/n)
   ```

4. **Execute and monitor:**
   ```bash
   ./scripts/prepare-release.sh X.Y.Z
   # ... show results ...
   git push origin main && git push origin vX.Y.Z
   gh run watch <ID>
   ```

5. **Report final status:**
   ```
   ✅ Release vX.Y.Z completed successfully!
   - Published to crates.io: https://crates.io/crates/agtrace
   - GitHub Release: https://github.com/lanegrid/agtrace/releases/tag/vX.Y.Z
   ```

---

## Maintenance Notes

This skill should be updated when:
- [ ] Release process changes
- [ ] New CI/CD steps added
- [ ] New validation requirements
- [ ] Security procedures updated

Last updated: 2025-12-31
