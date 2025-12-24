# Release Checklist

Quick reference for preparing OSS release of agtrace.

## Pre-Release Setup (One-time)

### GitHub Repository
- [ ] Make repository PUBLIC
- [ ] Verify all sensitive data removed (`.env`, API keys)
- [ ] LICENSE files present (MIT, Apache-2.0) ✓
- [ ] Add repository description and topics

### npm Registry
**Decision Point:** Choose package name strategy

#### Option A: Global namespace
- [ ] Package name: `agtrace-cli`
- [ ] Create npm account
- [ ] Generate Automation token
- [ ] Add `NPM_TOKEN` to GitHub Secrets

#### Option B: Scoped package (recommended)
- [ ] Package name: `@lanegrid/agtrace`
- [ ] Create npm account
- [ ] Create `lanegrid` organization on npm
- [ ] Generate Automation token
- [ ] Add `NPM_TOKEN` to GitHub Secrets
- [ ] Update `dist-workspace.toml`:
  ```toml
  npm-scope = "lanegrid"
  npm-package = "agtrace"
  ```
- [ ] Run `dist generate`

### crates.io Registry
**Decision Point:** What to publish?

#### Option A: CLI only (recommended)
- [ ] Create crates.io account (GitHub login)
- [ ] Generate API token with `publish-new` + `publish-update` scopes
- [ ] Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets
- [ ] Mark internal crates unpublishable:
  ```bash
  # Add to each internal crate's Cargo.toml
  [package]
  publish = false
  ```

#### Option B: Full library suite
- [ ] Create crates.io account
- [ ] Generate API token
- [ ] Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets
- [ ] Add metadata to all crate manifests:
  - description
  - documentation URL
  - keywords (5 max)
  - categories

### Documentation
- [ ] README.md (installation, quick start, examples)
- [ ] CONTRIBUTING.md (if accepting contributions)
- [ ] CHANGELOG.md (release notes)

## Release Process

### Phase 1: Initial Manual Release (v0.1.0)

**Goal:** Validate workflow without automation

1. **Pre-flight checks**
   ```bash
   cargo test --all
   cargo clippy --all-targets
   cargo fmt --check
   dist plan --tag=v0.1.0
   ```

2. **Create release**
   ```bash
   git tag v0.1.0
   git push origin main
   git push origin v0.1.0
   ```

3. **Wait for GitHub Actions**
   - Monitor workflow: https://github.com/lanegrid/agtrace/actions
   - Verify GitHub Release created
   - Download and test artifacts

4. **Manual publish to registries** (optional)
   ```bash
   # npm
   cd target/distrib
   tar -xzf agtrace-cli-npm-package.tar.gz
   cd package
   npm publish --dry-run  # test first
   npm publish            # actual publish

   # crates.io
   cargo publish --dry-run -p agtrace-cli
   cargo publish -p agtrace-cli
   ```

5. **Verify installations**
   ```bash
   # npm
   npm install -g agtrace-cli
   agtrace --version

   # crates.io
   cargo install agtrace-cli
   agtrace --version

   # Homebrew (after Tap setup)
   brew install lanegrid/tap/agtrace-cli
   agtrace --version
   ```

### Phase 2: Enable Automation (v0.2.0+)

**Goal:** Fully automated releases

1. **Enable auto-publish**

   Edit `dist-workspace.toml`:
   ```toml
   [dist]
   publish-jobs = ["npm", "crates-io"]  # or just one
   ```

2. **Regenerate workflow**
   ```bash
   dist generate
   git add .
   git commit -m "build: enable auto-publish to npm and crates.io"
   git push
   ```

3. **Test with v0.2.0**
   ```bash
   # Update version in Cargo.toml
   git tag v0.2.0
   git push origin v0.2.0

   # CI automatically publishes to all registries
   ```

## Homebrew Tap Setup

**Requires:** Separate public repository for formulas

1. **Create tap repository**
   ```bash
   # On GitHub, create: lanegrid/homebrew-tap
   git clone https://github.com/lanegrid/homebrew-tap
   cd homebrew-tap
   ```

2. **Copy formula from dist build**
   ```bash
   cp /path/to/agtrace/target/distrib/agtrace-cli.rb Formula/
   git add Formula/agtrace-cli.rb
   git commit -m "feat: add agtrace-cli formula"
   git push
   ```

3. **Users can now install**
   ```bash
   brew install lanegrid/tap/agtrace-cli
   ```

4. **Auto-update formula** (future)
   - Configure `dist` to update tap on release
   - Requires `HOMEBREW_TAP_TOKEN` GitHub secret

## Post-Release

- [ ] Announce on social media / blog
- [ ] Update documentation site
- [ ] Monitor issue tracker for bug reports
- [ ] Plan next release milestones

## Emergency: Yanking a Release

### npm
```bash
npm unpublish agtrace-cli@0.1.0
```

### crates.io
```bash
cargo yank --vers 0.1.0 agtrace-cli
```

Note: Yanking removes from default installs but keeps version published (SemVer guarantee).

## Quick Reference: Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| cargo-dist | ✓ Configured | v0.30.3 |
| GitHub Actions | ✓ Working | Release workflow tested |
| Binary builds | ✓ Tested | 5 platforms supported |
| npm package | ✓ Built | Not auto-published yet |
| crates.io | ⚠️ Not configured | Needs tokens + metadata |
| Package names | ✓ Available | All checked on registries |
| Repository | 🔒 Private | Make public before release |
