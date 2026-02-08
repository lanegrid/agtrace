# Release Procedure

This document describes the standard release process for agtrace.

## Prerequisites

- Ensure `cargo login` is configured with crates.io token
- On `main` branch with clean working directory
- Tools installed: `cargo install git-cliff cargo-release`

## Release Steps

### 0. Set Release Level

Choose the release level (`patch`, `minor`, or `major`):

```bash
# Set release level
RELEASE_LEVEL=patch  # or: minor, major
```

### 1. Dry-run (Verification)

Check what will happen without making changes (dry-run is the default):

```bash
cargo release --workspace ${RELEASE_LEVEL} --no-verify
```

**Flags explained**:
- `--workspace`: Required to publish all 6 crates in dependency order
- `--no-verify`: Skips local verification to avoid a known Cargo bug ([#14396](https://github.com/rust-lang/cargo/issues/14396)) that causes "no hash listed" errors during dry-run. The actual release with `--execute` verifies against crates.io and works correctly.

### 2. Update CHANGELOG

Generate CHANGELOG entries for changes since the last release:

```bash
# Get the latest tag and calculate next version
LAST_TAG=$(git describe --tags --abbrev=0)
CURRENT_VERSION=${LAST_TAG#v}  # Remove 'v' prefix

# Calculate next version based on release level
IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"
case "$RELEASE_LEVEL" in
  major) NEXT_VERSION="$((major + 1)).0.0" ;;
  minor) NEXT_VERSION="${major}.$((minor + 1)).0" ;;
  patch) NEXT_VERSION="${major}.${minor}.$((patch + 1))" ;;
esac

echo "Current version: $CURRENT_VERSION"
echo "Next version: $NEXT_VERSION"

# Generate changelog for commits since last tag with version and date
git cliff ${LAST_TAG}..HEAD --unreleased --tag v${NEXT_VERSION} --prepend CHANGELOG.md
```

This prepends new entries to CHANGELOG.md with the calculated version and current date.

Review the generated changes to verify correctness.

### 3. Commit CHANGELOG

Commit the updated CHANGELOG with auto-generated commit message:

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for v${NEXT_VERSION}"
```

### 4. Execute Release

This command will:
- Update version in all `Cargo.toml` files
- Create git commit
- Publish to crates.io (in dependency order: types → providers → index → engine → runtime → cli)
- Create and push git tag

```bash
cargo release --workspace ${RELEASE_LEVEL} --execute
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

- **Deterministic generation**: Version and date are automatically calculated from Git state
  - Version: Computed from last tag + release level
  - Date: Current date in YYYY-MM-DD format (git-cliff default)
- **Incremental releases**: Only commits since last tag are included
- **Initial release**: Manually write a concise "Initial public release" summary instead of dumping all commits
- **Keep it readable**: Use conventional commits (feat:, fix:, docs:) for automatic grouping
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
