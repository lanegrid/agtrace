# Release Procedure

This document describes the standard release process for agtrace.

## Prerequisites

- Ensure `cargo login` is configured with crates.io token
- On `main` branch with clean working directory
- Tools installed: `cargo install git-cliff cargo-release`

## Release Steps

### 1. Dry-run (Verification)

Check what will happen without making changes:

```bash
cargo release patch --dry-run
```

Replace `patch` with `minor` or `major` as needed.

### 2. Generate CHANGELOG

Generate CHANGELOG from commit history:

```bash
git cliff -o CHANGELOG.md
```

Review the generated `CHANGELOG.md`. Edit manually if needed, then:

```bash
git add CHANGELOG.md
```

### 3. Execute Release

This command will:
- Update version in `Cargo.toml`
- Create git commit
- Publish to crates.io (in dependency order: types → engine → ... → cli)
- Create and push git tag

```bash
cargo release patch --execute
```

### 4. Verify GitHub Actions

After tag push (e.g., `v0.1.2`), the workflow `.github/workflows/release.yml` automatically triggers:

- **crates.io**: Already published in step 3
- **GitHub Release**: CI builds binaries for all platforms and uploads artifacts

Check the Actions tab to ensure successful completion.

## Notes

- First release reserves package names on crates.io
- If a name is already taken, update `name` in `Cargo.toml`
- Binary distribution uses `cargo-dist` (handled by CI)
- crates.io publishing uses `cargo-release` (handled locally)
