# Publishing Preparation Guide

This document outlines the steps required to prepare `agtrace` for automatic publishing to npm.

## Publishing Strategy

agtrace uses **automatic publishing** to npm on every tagged release:

| Registry | Method | Required Setup |
|----------|--------|----------------|
| **npm** | OIDC Trusted Publishing | Configure on npmjs.com (no token needed) |

**The npm package publishes automatically when you push a version tag (e.g., `v0.1.0`).**

### Architecture

- **cargo-dist**: Builds cross-platform binaries and generates the npm package tarball
- **Custom OIDC job**: Publishes the npm package using OIDC (no long-lived tokens)

### Note on crates.io

The workspace crates (agtrace-types, agtrace-providers, etc.) are **not published to crates.io**. The project is distributed as pre-built binaries via npm.

---

## Package Names

| Component | Name |
|-----------|------|
| **Binary name** | `agtrace` |
| **npm package** | `@lanegrid/agtrace` |

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

## 2. Release Workflow

Once npm OIDC is configured, releasing is simple:

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
1. ✅ Build binaries for 5 platforms (macOS, Linux, Windows)
2. ✅ Generate installers (shell, npm, homebrew)
3. ✅ Create GitHub Release with all artifacts
4. ✅ Publish to npm (via OIDC with provenance)

---

## 3. Verify Installation

After the release workflow completes:

```bash
# npm (recommended)
npm install -g @lanegrid/agtrace
agtrace --version

# Homebrew (after tap setup)
brew install lanegrid/tap/agtrace
agtrace --version

# Shell script (curl)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/lanegrid/agtrace/releases/latest/download/agtrace-installer.sh | sh
```

---

## 4. Setup Checklist

### npm (OIDC)
- [ ] npm account created
- [ ] `@lanegrid` organization created on npmjs.com
- [ ] Trusted Publisher configured at https://www.npmjs.com/settings/@lanegrid/agtrace/publishing
  - Organization: `lanegrid`
  - Repository: `agtrace`
  - Workflow: `release.yml`

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

---

## Security Notes

### npm OIDC Publishing
- ✅ **No long-lived tokens** - OIDC tokens are issued per-workflow and expire immediately
- ✅ **Automatic provenance** - Sigstore attestations prove the package was built in GitHub Actions
- ✅ **Workflow-scoped** - Credentials only valid for the specific workflow run
- ✅ **Transparent supply chain** - Users can verify the package origin with `npm audit signatures`

### Binary Distribution
- All binaries are built in GitHub Actions with full transparency
- GitHub Artifact Attestations provide cryptographic proof of build provenance
- No manual build steps or local compilation required

---

## Learn More

- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
- [cargo-dist Documentation](https://opensource.axo.dev/cargo-dist/)
