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
cargo release --workspace patch --no-verify
```

Replace `patch` with `minor` or `major` as needed.

**Flags explained**:
- `--workspace`: Required to publish all 6 crates in dependency order
- `--no-verify`: Skips local verification to avoid a known Cargo bug ([#14396](https://github.com/rust-lang/cargo/issues/14396)) that causes "no hash listed" errors during dry-run. The actual release with `--execute` verifies against crates.io and works correctly.

### 2. Update CHANGELOG

Generate CHANGELOG entries for changes since the last release:

```bash
# Get the latest tag
LAST_TAG=$(git describe --tags --abbrev=0)

# Generate changelog for commits since last tag
git cliff ${LAST_TAG}..HEAD --unreleased --prepend CHANGELOG.md
```

This prepends new entries to the existing CHANGELOG.md under the `[Unreleased]` section.

Review the generated changes:
- Check that only new commits are added
- Edit descriptions for clarity if needed
- The `[Unreleased]` section will be automatically renamed to the version number during release

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

### CHANGELOG Best Practices

- **Incremental releases**: Use `git cliff ${LAST_TAG}..HEAD --unreleased --prepend CHANGELOG.md` to add only new commits
- **Initial release**: Manually write a concise "Initial public release" summary instead of dumping all commits
- **Keep it readable**: Focus on user-visible changes (features, bug fixes, breaking changes)
- **Full history in Git**: The complete commit history is always available via `git log`

## Troubleshooting

### "no hash listed" error during dry-run

If you see this error during `cargo release --workspace`:

```
error: failed to verify package tarball
Caused by: no hash listed for agtrace-xxx v0.x.x
```

This is a known Cargo bug ([#14396](https://github.com/rust-lang/cargo/issues/14396)) that only affects dry-run verification of interdependent workspace crates. The actual release (`--execute`) succeeds because crates are published to crates.io sequentially, and each published crate gets a proper checksum in the registry.

**Solution**: Use `--no-verify` flag for dry-run:
```bash
cargo release --workspace patch --no-verify
```
