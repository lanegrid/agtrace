# Publishing Preparation Guide

This document outlines the steps required to prepare `agtrace` for automatic publishing to npm and crates.io registries.

## Publishing Strategy

agtrace uses **automatic publishing** to both npm and crates.io on every tagged release:

| Registry | Method | Required Setup |
|----------|--------|----------------|
| **npm** | OIDC Trusted Publishing | Configure on npmjs.com (no token) |
| **crates.io** | Token-based | Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets |

**Both registries publish automatically when you push a version tag (e.g., `v0.1.0`).**

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
2. Click "Select your publisher" → **GitHub Actions**
3. Fill in:
   - **Organization**: `lanegrid`
   - **Repository**: `agtrace`
   - **Workflow filename**: `release.yml`
   - **Environment name**: (leave empty)
4. Save

**That's it! No npm token needed.**

The GitHub Actions workflow already has the required OIDC configuration.

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
- [ ] `@lanegrid` organization created
- [ ] Trusted Publisher configured on npmjs.com
- [ ] Repository is PUBLIC

### crates.io (Token)
- [ ] crates.io account created (GitHub login)
- [ ] API token generated with correct scopes
- [ ] `CARGO_REGISTRY_TOKEN` added to GitHub Secrets

### Repository
- [ ] Repository is PUBLIC (required for OIDC)
- [ ] All secrets verified (none leaked)

---

## Troubleshooting

### npm publish fails

**Error: "Unable to authenticate"**

Check:
- Trusted Publisher settings match exactly (case-sensitive)
- Repository is PUBLIC
- Workflow filename is `release.yml` (exact match)

### crates.io publish fails

**Error: "failed to authenticate"**

Check:
- GitHub Secret name is exactly `CARGO_REGISTRY_TOKEN`
- Token has `publish-new` and `publish-update` scopes
- Token hasn't expired

---

## Security Notes

**npm OIDC:**
- ✅ No long-lived tokens
- ✅ Automatic provenance attestations
- ✅ Workflow-scoped credentials

**crates.io Token:**
- ⚠️ Long-lived token (rotate periodically)
- ✅ Minimal scopes (publish only)
- ⚠️ Store securely in GitHub Secrets only

---

## Learn More

- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
