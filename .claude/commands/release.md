# Release Procedure

This document describes the standard release process for agtrace.

## How releasing works

The release is split between local steps and CI:

- **Local** (`cargo release`): bump versions, update CHANGELOG, commit, tag, push.
  `release.toml` sets `publish = false`, so nothing is published from your machine.
- **CI** (`.github/workflows/release.yml`, triggered by the `v*` tag push):
  - `publish-crates`: publishes all crates to crates.io in dependency order
    using the `CARGO_REGISTRY_TOKEN` repository secret
  - `build-local/global-artifacts` + `host`: cargo-dist builds binaries for all
    platforms and creates the GitHub Release
  - `publish-npm`: publishes the npm package

**Do NOT publish to crates.io locally.** No `cargo login` is needed. If a crate
version is already on crates.io when CI runs, `cargo publish` fails with
"already uploaded" and the whole release workflow (including the GitHub
Release) is blocked.

## Prerequisites

- On `main` branch with clean working directory
- Tools installed: `git-cliff` and `cargo-release`
  (`brew install git-cliff cargo-release` or `cargo install git-cliff cargo-release`)

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
cargo release --workspace ${RELEASE_LEVEL}
```

Confirm the version bump and the "Pushing main, vX.Y.Z to origin" plan.

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

### 4. Execute Release (local part)

This command will:
- Update version in all `Cargo.toml` files
- Create the release commit
- Create the `vX.Y.Z` git tag
- Push `main` and the tag to origin (this triggers the CI release workflow)

```bash
cargo release --workspace ${RELEASE_LEVEL} --execute
```

### 5. Watch GitHub Actions (publishing part)

The tag push triggers `.github/workflows/release.yml`, which does all
publishing:

- **crates.io**: `publish-crates` job publishes every crate in dependency order
- **GitHub Release**: cargo-dist builds binaries for all platforms and uploads artifacts
- **npm**: `publish-npm` job

Watch it to completion:

```bash
gh run watch $(gh run list --workflow=release.yml --limit 1 --json databaseId -q '.[0].databaseId') --exit-status
```

## Notes

- First release reserves package names on crates.io
- If a name is already taken, update `name` in `Cargo.toml`
- Binary distribution uses `cargo-dist` (handled by CI)
- crates.io publishing uses plain `cargo publish` (handled by CI, NOT locally)

### CHANGELOG Best Practices

- **Deterministic generation**: Version and date are automatically calculated from Git state
  - Version: Computed from last tag + release level
  - Date: Current date in YYYY-MM-DD format (git-cliff default)
- **Incremental releases**: Only commits since last tag are included
- **Initial release**: Manually write a concise "Initial public release" summary instead of dumping all commits
- **Keep it readable**: Use conventional commits (feat:, fix:, docs:) for automatic grouping
- **Full history in Git**: The complete commit history is always available via `git log`

## Troubleshooting

### Release stopped halfway (e.g. before tag/push)

`cargo release` runs discrete steps; if it aborts partway you can resume with
the step subcommands instead of re-running the whole release (which would bump
versions again):

```bash
cargo release tag --workspace --execute   # create vX.Y.Z from the release commit
cargo release push --workspace --execute  # push main + tag (triggers CI)
```

### publish-crates fails with "already uploaded"

A crate version was published outside CI (or the job re-ran after a partial
publish). `cargo publish` treats an existing version as a hard error. Re-run
the job after yanking is NOT possible for the same version — bump a new patch
release instead, and never publish locally.
