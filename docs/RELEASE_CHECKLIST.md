# Release Checklist

Quick reference for OSS release of agtrace.

## Pre-Release Setup (One-Time)

### 1. GitHub Repository
- [ ] Make repository PUBLIC
- [ ] Verify no sensitive data (`.env`, API keys, etc.)
- [ ] LICENSE files present ✓

### 2. npm (OIDC - No Token)
- [ ] Create npm account
- [ ] Create `@lanegrid` organization
- [ ] Configure Trusted Publisher on npmjs.com:
  - Organization: `lanegrid`
  - Repository: `agtrace`
  - Workflow: `release.yml`

### 3. crates.io (Token)
- [ ] Create crates.io account (GitHub login)
- [ ] Generate API token with `publish-new` + `publish-update` scopes
- [ ] Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets

---

## Release Process

### Every Release

1. **Pre-flight checks:**
   ```bash
   cargo test --all
   cargo clippy --all-targets
   cargo fmt --check
   dist plan --tag=v0.1.0
   ```

2. **Create and push tag:**
   ```bash
   git tag v0.1.0
   git push origin main
   git push origin v0.1.0
   ```

3. **GitHub Actions automatically:**
   - ✅ Builds binaries (5 platforms)
   - ✅ Creates GitHub Release
   - ✅ Publishes to npm
   - ✅ Publishes to crates.io

4. **Verify:**
   ```bash
   npm install -g @lanegrid/agtrace
   agtrace --version

   cargo install agtrace-cli
   agtrace --version
   ```

---

## Current Status

| Component | Status |
|-----------|--------|
| cargo-dist | ✓ v0.30.3 |
| npm package | `@lanegrid/agtrace` |
| crate | `agtrace-cli` |
| binary | `agtrace` |
| npm auto-publish | ✓ OIDC |
| crates.io auto-publish | ✓ Token |
| Repository | 🔒 Private → PUBLIC before release |

---

## Troubleshooting

**npm fails:** Check Trusted Publisher config on npmjs.com
**crates.io fails:** Check `CARGO_REGISTRY_TOKEN` in GitHub Secrets
