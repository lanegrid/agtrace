# Publishing Preparation Guide

This document outlines the steps required to prepare `agtrace` for automatic publishing to npm and crates.io registries.

## Publishing Strategy

agtrace uses **automatic publishing** to both npm and crates.io on every tagged release:

| Registry | Method | Required Setup |
|----------|--------|----------------|
| **npm** | OIDC Trusted Publishing | Configure on npmjs.com (no token needed) |
| **crates.io** | Token-based | Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets |

**Both registries publish automatically when you push a version tag (e.g., `v0.1.0`).**

### Architecture

- **cargo-dist**: Builds cross-platform binaries and generates the npm package tarball
- **Custom OIDC job**: Publishes the npm package using OIDC (no long-lived tokens)
- **Custom cargo publish job**: Publishes the Rust crate to crates.io

---

## Package Names

| Component | Name |
|-----------|------|
| **Crate name** | `agtrace-cli` |
| **Binary name** | `agtrace` |
| **npm package** | `@lanegrid/agtrace` |

**Availability:** All names are available ✓

---

## 1. npm Setup (OIDC - No Token Needed)

### Step 1: Create Organization

1. Visit: https://www.npmjs.com/org/create
2. Organization name: `lanegrid`

### Step 2: Configure Trusted Publisher

1. Go to: https://www.npmjs.com/settings/@lanegrid/agtrace/publishing
2. Click "Add trusted publisher" → Select **GitHub Actions**
3. Fill in the configuration:
   - **Organization**: `lanegrid`
   - **Repository**: `agtrace`
   - **Workflow filename**: `release.yml`
   - **Environment name**: (leave empty)
4. Save

**That's it! No npm token needed.**

### Benefits of OIDC Publishing

- ✅ **No long-lived tokens** - Eliminates secret management and rotation
- ✅ **Automatic provenance** - npm automatically attaches Sigstore attestations
- ✅ **Workflow-scoped** - Credentials only valid during workflow execution
- ✅ **Better security** - No risk of token leakage

---

## 2. crates.io Setup (Token-based)

### Step 1: Generate API Token

1. Visit: https://crates.io/settings/tokens
2. Click "New Token"
3. Settings:
   - **Name**: `github-actions-agtrace`
   - **Scopes**: Check `publish-new` and `publish-update`
4. **Copy the token** (starts with `crates-io_...`)

### Step 2: Add to GitHub Secrets

1. Go to: https://github.com/lanegrid/agtrace/settings/secrets/actions
2. Click "New repository secret"
3. Settings:
   - **Name**: `CARGO_REGISTRY_TOKEN`
   - **Secret**: (paste the token from Step 1)
4. Save

---

## 3. Release Workflow

Once both registries are configured, releasing is simple:

```bash
# 1. Run checks
cargo test --all
cargo clippy --all-targets
cargo fmt --check

# 2. Create and push tag
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

**GitHub Actions will automatically:**
1. ✅ Build binaries for 5 platforms
2. ✅ Generate installers (shell, npm, homebrew)
3. ✅ Create GitHub Release
4. ✅ Publish to npm (via OIDC)
5. ✅ Publish to crates.io (via token)

---

## 4. Verify Installation

After the release workflow completes:

```bash
# npm
npm install -g @lanegrid/agtrace
agtrace --version

# crates.io
cargo install agtrace-cli
agtrace --version

# Homebrew (after tap setup)
brew install lanegrid/tap/agtrace-cli
agtrace --version
```

---

## Setup Checklist

### npm (OIDC)
- [ ] npm account created
- [ ] `@lanegrid` organization created on npmjs.com
- [ ] Trusted Publisher configured at https://www.npmjs.com/settings/@lanegrid/agtrace/publishing
  - Organization: `lanegrid`
  - Repository: `agtrace`
  - Workflow: `release.yml`
- [ ] Repository is PUBLIC (required for npm OIDC)

### crates.io (Token)
- [ ] crates.io account created (GitHub login)
- [ ] API token generated with `publish-new` and `publish-update` scopes
- [ ] `CARGO_REGISTRY_TOKEN` added to GitHub Secrets

### Repository
- [ ] Repository is PUBLIC (required for npm OIDC and provenance)
- [ ] GitHub Actions enabled
- [ ] Workflow has correct permissions (`id-token: write` for OIDC)

---

## Troubleshooting

### npm publish fails

**Error: "Unable to verify the first certificate" or "OIDC token validation failed"**

Check:
- Trusted Publisher settings match **exactly** (case-sensitive):
  - Organization: `lanegrid` (not `Lanegrid`)
  - Repository: `agtrace` (not `lanegrid/agtrace`)
  - Workflow: `release.yml` (not `.github/workflows/release.yml`)
- Repository is PUBLIC (private repos don't support OIDC)
- Workflow has `permissions.id-token: write`
- npm CLI version ≥ 11.5.1 (check in workflow logs)

**Error: "Package name too similar to existing package"**

Check:
- Package name `@lanegrid/agtrace` is available
- Organization `@lanegrid` exists and you have publish permissions

### crates.io publish fails

**Error: "failed to authenticate"**

Check:
- GitHub Secret name is exactly `CARGO_REGISTRY_TOKEN`
- Token has `publish-new` and `publish-update` scopes
- Token hasn't expired (check at https://crates.io/settings/tokens)

**Error: "crate name already exists"**

- The crate name is `agtrace-cli`, not `agtrace`

---

## Security Notes

### npm OIDC Publishing
- ✅ **No long-lived tokens** - OIDC tokens are issued per-workflow and expire immediately
- ✅ **Automatic provenance** - Sigstore attestations prove the package was built in GitHub Actions
- ✅ **Workflow-scoped** - Credentials only valid for the specific workflow run
- ✅ **Transparent supply chain** - Users can verify the package origin with `npm audit signatures`

### crates.io Token-based Publishing
- ⚠️ **Long-lived token** - Rotate periodically (recommended: every 6 months)
- ✅ **Minimal scopes** - Token only has `publish-new` and `publish-update` permissions
- ⚠️ **Secret management** - Store securely in GitHub Secrets only, never commit to repo
- 💡 **Future**: crates.io added OIDC support in July 2025, may migrate in future

---

## Learn More

- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
