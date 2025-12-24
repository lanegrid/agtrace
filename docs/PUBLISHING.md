# Publishing Preparation Guide

This document outlines the steps required to prepare `agtrace` for publishing to npm and crates.io registries.

## Package Name Availability

**Status: All names are AVAILABLE** ✓

### crates.io
- `agtrace` - AVAILABLE
- `agtrace-cli` - AVAILABLE
- `agtrace-types` - AVAILABLE
- `agtrace-providers` - AVAILABLE
- `agtrace-engine` - AVAILABLE
- `agtrace-index` - AVAILABLE
- `agtrace-runtime` - AVAILABLE

### npm
- `agtrace-cli` - AVAILABLE
- `@lanegrid/agtrace` - AVAILABLE (if org created)

## 1. npm Registry Setup

### A. Account & Organization Setup

1. **Create npm account** (if not exists)
   ```bash
   npm adduser
   ```

2. **Create organization** (optional but recommended)
   - Visit: https://www.npmjs.com/org/create
   - Organization name: `lanegrid`
   - Benefit: Use scoped package `@lanegrid/agtrace` instead of global `agtrace-cli`

3. **Verify organization access**
   ```bash
   npm org ls lanegrid
   ```

### B. Access Token Generation

1. **Create Automation Token**
   - Visit: https://www.npmjs.com/settings/YOUR_USERNAME/tokens
   - Click "Generate New Token" → "Automation"
   - Token scope: Automation (for CI/CD)
   - Copy token (shown only once)

2. **Store token in GitHub Secrets**
   - Repository → Settings → Secrets and variables → Actions
   - New repository secret:
     - Name: `NPM_TOKEN`
     - Value: `npm_xxxxxxxxxxxxxxxxxxxxxxxxxx`

### C. Package Configuration Decision

**Current config:**
```json
{
  "name": "agtrace-cli",
  "version": "0.1.0"
}
```

**Option 1: Global namespace (current)**
- Package name: `agtrace-cli`
- Install: `npm install -g agtrace-cli`
- No organization needed
- ✓ Simpler setup
- ✗ Name collision risk

**Option 2: Scoped package (recommended)**
- Package name: `@lanegrid/agtrace`
- Install: `npm install -g @lanegrid/agtrace`
- Requires `lanegrid` organization
- ✓ Professional branding
- ✓ Name protection
- ✗ Requires org setup

**To switch to scoped package:**

Edit `dist-workspace.toml`:
```toml
[dist]
npm-scope = "lanegrid"
npm-package = "agtrace"
```

Then regenerate:
```bash
dist generate
```

### D. Enable npm Publishing in CI

**Current status:** npm package is BUILT but NOT PUBLISHED

To enable auto-publishing to npm on release:

Edit `dist-workspace.toml`:
```toml
[dist]
publish-jobs = ["npm"]
```

Regenerate workflow:
```bash
dist generate
```

This adds a publish job to `.github/workflows/release.yml` that:
- Runs after GitHub Release is created
- Publishes npm package using `NPM_TOKEN` secret
- Only runs on tagged releases (not PRs)

## 2. crates.io Registry Setup

### A. Account Setup

1. **Create crates.io account**
   - Visit: https://crates.io/
   - Sign in with GitHub account

2. **Generate API Token**
   - Visit: https://crates.io/settings/tokens
   - Click "New Token"
   - Token name: `github-actions-agtrace`
   - Scopes: `publish-new` and `publish-update`
   - Copy token (shown only once)

3. **Store token in GitHub Secrets**
   - Repository → Settings → Secrets and variables → Actions
   - New repository secret:
     - Name: `CARGO_REGISTRY_TOKEN`
     - Value: `crates-io_xxxxxxxxxxxxxxxxxxxxxxxxxx`

### B. Crate Publishing Strategy

**Decision needed:** Which crates to publish?

#### Option 1: CLI Only (recommended for initial release)
Publish only `agtrace-cli` to crates.io.

**Pros:**
- Simple dependency management
- Users get working CLI immediately
- Internal crates remain private

**Cons:**
- Cannot be used as Rust library

**Implementation:**
Mark internal crates as unpublishable in their `Cargo.toml`:
```toml
[package]
publish = false
```

#### Option 2: Full Library Suite
Publish all crates for Rust ecosystem reuse.

**Pros:**
- Other Rust projects can use agtrace as library
- Community contributions easier

**Cons:**
- Maintain public API stability
- More release coordination

**Implementation:**
All crates must have complete metadata:
```toml
[package]
description = "..."
license.workspace = true
repository.workspace = true
documentation = "https://docs.rs/agtrace-types"
readme = "README.md"
keywords = ["agent", "tracing", "observability"]
categories = ["development-tools", "command-line-utilities"]
```

### C. Enable crates.io Publishing in CI

Edit `dist-workspace.toml`:
```toml
[dist]
publish-jobs = ["crates-io"]
```

Or enable both npm and crates.io:
```toml
[dist]
publish-jobs = ["npm", "crates-io"]
```

Regenerate workflow:
```bash
dist generate
```

### D. Pre-Publish Validation

Before first publish, verify crate metadata:

```bash
# Check what will be published
cargo package --list -p agtrace-cli

# Dry run publish
cargo publish --dry-run -p agtrace-cli

# Check for missing fields
cargo package -p agtrace-cli
```

## 3. Release Workflow with Publishing

Once configured, the release process becomes:

```bash
# 1. Update version in Cargo.toml
# 2. Commit changes
git add .
git commit -m "release: bump to v0.2.0"

# 3. Create and push tag
git tag v0.2.0
git push origin main
git push origin v0.2.0

# 4. GitHub Actions automatically:
#    - Builds binaries for all platforms
#    - Creates GitHub Release
#    - Publishes to npm (if enabled)
#    - Publishes to crates.io (if enabled)
```

## 4. Verification Checklist

Before enabling auto-publish:

### npm
- [ ] npm account created
- [ ] Organization created (if using scoped package)
- [ ] `NPM_TOKEN` added to GitHub Secrets
- [ ] Package name decision made (scoped vs global)
- [ ] `dist-workspace.toml` configured
- [ ] `dist generate` executed
- [ ] Manual test: `npm publish --dry-run` on built package

### crates.io
- [ ] crates.io account created (linked to GitHub)
- [ ] `CARGO_REGISTRY_TOKEN` added to GitHub Secrets
- [ ] Publishing strategy decided (CLI-only vs full suite)
- [ ] Unpublishable crates marked with `publish = false`
- [ ] All publishable crates have complete metadata
- [ ] Manual test: `cargo publish --dry-run -p agtrace-cli`

### GitHub
- [ ] Repository is PUBLIC (required for crates.io)
- [ ] Secrets configured: `NPM_TOKEN`, `CARGO_REGISTRY_TOKEN`
- [ ] Release workflow tested with dry-run

## 5. Testing Before Going Live

### Test npm package locally
```bash
# Build npm package
dist build --artifacts=global --tag=v0.1.0

# Extract and test
cd /tmp
tar -xzf /path/to/agtrace-cli-npm-package.tar.gz
cd package
npm install
./run-agtrace.js --version
```

### Test crates.io publish (dry-run)
```bash
cargo publish --dry-run -p agtrace-cli
```

### Test GitHub Actions without publishing
```bash
# Create a test tag (don't push)
git tag v0.1.0-test

# Check what dist would do
dist plan --tag=v0.1.0-test

# Build all artifacts locally
dist build --tag=v0.1.0-test
```

## 6. Initial Release Recommendations

For the first public release (v0.1.0):

1. **Start conservative:**
   - GitHub Releases only (already working)
   - No auto-publish to npm/crates.io yet

2. **Validate with manual publish:**
   ```bash
   # After GitHub Release is created
   cd target/distrib
   tar -xzf agtrace-cli-npm-package.tar.gz
   cd package
   npm publish  # Manual first time

   # For crates.io
   cargo publish -p agtrace-cli
   ```

3. **Enable automation after v0.1.1+:**
   - Once manual publish succeeds
   - Configure `publish-jobs` in dist-workspace.toml
   - Future releases fully automated

## Current Configuration Summary

**Status: Pre-Release Setup**

| Registry | Package Name | Status | Auto-Publish |
|----------|--------------|--------|--------------|
| GitHub Releases | agtrace-cli-v0.1.0 | ✓ Ready | ✓ Enabled |
| npm | `agtrace-cli` | ⚠️ Needs token | ✗ Disabled |
| crates.io | `agtrace-cli` | ⚠️ Needs token | ✗ Disabled |

**Next Steps:**
1. Decide npm package naming (global vs scoped)
2. Create registry accounts and tokens
3. Add secrets to GitHub repository
4. Test manual publish workflow
5. Enable auto-publish for v0.2.0+
