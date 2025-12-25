# Release Procedure

This document describes the standard release process for agtrace.

## Prerequisites

- Ensure `cargo login` is configured with crates.io token
- On `main` branch with clean working directory
- Tools installed: `cargo install git-cliff cargo-release`

## Release Steps

### 1. Dry-run (Verification)

Check what will happen without making changes (dry-run is the default):

```bash
cargo release --workspace patch
```

Replace `patch` with `minor` or `major` as needed.
The `--workspace` flag is required to publish all 6 crates in dependency order.

### 2. Update CHANGELOG

Generate CHANGELOG from commit history:

```bash
git cliff -o CHANGELOG.md
```

Review the generated `CHANGELOG.md`. The `## [unreleased]` section should contain recent changes.
Edit manually if needed.

### 3. Commit CHANGELOG

Commit the updated CHANGELOG before running the release:

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for vX.Y.Z"
```

Replace `X.Y.Z` with the actual version number.

### 4. Execute Release

This command will:
- Update version in all `Cargo.toml` files
- Create git commit
- Publish to crates.io (in dependency order: types → providers → index → engine → runtime → cli)
- Create and push git tag

```bash
cargo release --workspace patch --execute
```

### 5. Verify GitHub Actions

After tag push (e.g., `v0.1.2`), the workflow `.github/workflows/release.yml` automatically triggers:

- **crates.io**: Already published in step 4
- **GitHub Release**: CI builds binaries for all platforms and uploads artifacts

Check the Actions tab to ensure successful completion.

## Notes

- First release reserves package names on crates.io
- If a name is already taken, update `name` in `Cargo.toml`
- Binary distribution uses `cargo-dist` (handled by CI)
- crates.io publishing uses `cargo-release` (handled locally)
