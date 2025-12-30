#!/bin/bash
# Prepare a new release of agtrace
#
# Usage:
#   ./scripts/prepare-release.sh              # Interactive mode
#   ./scripts/prepare-release.sh 0.1.15       # Specify version
#   ./scripts/prepare-release.sh --dry-run    # Preview changes
#
# This script does everything needed for a release:
#   1. Validates working directory
#   2. Runs all tests
#   3. Validates README examples compile
#   4. Updates version numbers
#   5. Generates CHANGELOG
#   6. Runs fmt and clippy
#   7. Creates commit and git tag
#
# After success, just push:
#   git push origin main && git push origin v0.1.15

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
DRY_RUN=false
NEW_VERSION=""

for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            NEW_VERSION=$arg
            shift
            ;;
    esac
done

# Get current version
CURRENT_VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "agtrace") | .version')

echo -e "${BLUE}=== agtrace Release Preparation ===${NC}"
echo
echo "Current version: $CURRENT_VERSION"

# If no version specified, prompt
if [ -z "$NEW_VERSION" ]; then
    echo -e "${YELLOW}Enter new version number (e.g., 0.1.15):${NC}"
    read NEW_VERSION
fi

echo "New version:     $NEW_VERSION"
echo

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}🔍 DRY RUN MODE - No changes will be made${NC}"
    echo
fi

# Validate version format
if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format. Expected: MAJOR.MINOR.PATCH (e.g., 0.1.15)${NC}"
    exit 1
fi

# Check if version is newer
if [ "$NEW_VERSION" = "$CURRENT_VERSION" ]; then
    echo -e "${RED}Error: New version must be different from current version${NC}"
    exit 1
fi

echo -e "${BLUE}Step 1/7: Checking working directory${NC}"
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${YELLOW}Warning: Working directory has uncommitted changes${NC}"
    git status --short
    echo
    if [ "$DRY_RUN" = false ]; then
        echo -e "${YELLOW}Continue anyway? (y/n)${NC}"
        read -r response
        if [[ ! $response =~ ^[Yy]$ ]]; then
            echo "Aborted."
            exit 1
        fi
    fi
fi
echo -e "${GREEN}✓ Working directory checked${NC}"
echo

echo -e "${BLUE}Step 2/7: Running tests${NC}"
if [ "$DRY_RUN" = false ]; then
    cargo test --workspace --quiet
fi
echo -e "${GREEN}✓ Tests passed${NC}"
echo

echo -e "${BLUE}Step 3/7: Checking README sync${NC}"
if [ "$DRY_RUN" = false ]; then
    ./scripts/check-readme-sync.sh
fi
echo -e "${GREEN}✓ README validation passed${NC}"
echo

echo -e "${BLUE}Step 4/7: Updating version numbers${NC}"
if [ "$DRY_RUN" = false ]; then
    # Update workspace version
    sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
    rm Cargo.toml.bak

    # Update all workspace dependency versions
    sed -i.bak "s/version = \"$CURRENT_VERSION\", path = \"crates\//version = \"$NEW_VERSION\", path = \"crates\//" Cargo.toml
    rm Cargo.toml.bak
else
    echo "  Would update: Cargo.toml"
fi
echo -e "${GREEN}✓ Version numbers updated${NC}"
echo

echo -e "${BLUE}Step 5/7: Generating CHANGELOG${NC}"
if [ "$DRY_RUN" = false ]; then
    # Get the last tag
    LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

    if [ -n "$LAST_TAG" ]; then
        echo "Generating changelog from $LAST_TAG to HEAD..."
        git cliff $LAST_TAG..HEAD --unreleased --tag "v$NEW_VERSION" --prepend CHANGELOG.md
    else
        echo "Generating changelog for all commits..."
        git cliff --unreleased --tag "v$NEW_VERSION" --prepend CHANGELOG.md
    fi
else
    echo "  Would run: git cliff ... --tag v$NEW_VERSION --prepend CHANGELOG.md"
fi
echo -e "${GREEN}✓ CHANGELOG generated${NC}"
echo

echo -e "${BLUE}Step 6/7: Running checks${NC}"
if [ "$DRY_RUN" = false ]; then
    cargo fmt --all
    cargo clippy --all-targets -- -D warnings
fi
echo -e "${GREEN}✓ Formatting and linting passed${NC}"
echo

echo -e "${BLUE}Step 7/7: Summary${NC}"
echo "The following changes will be committed:"
echo "  - Version: $CURRENT_VERSION → $NEW_VERSION"
echo "  - CHANGELOG.md updated"
echo

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}This was a dry run. No changes were made.${NC}"
    exit 0
fi

echo -e "${YELLOW}Create release commit and tag? (y/n)${NC}"
read -r response
if [[ ! $response =~ ^[Yy]$ ]]; then
    echo "Aborted. Changes have been made but not committed."
    echo "To undo: git checkout Cargo.toml CHANGELOG.md"
    exit 1
fi

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to $NEW_VERSION and update CHANGELOG"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo
echo -e "${GREEN}✅ Release v$NEW_VERSION prepared!${NC}"
echo
echo "Next steps:"
echo "  1. Review the changes: git show HEAD"
echo "  2. Push to trigger release: git push origin main && git push origin v$NEW_VERSION"
echo
echo "To undo:"
echo "  git reset --hard HEAD~1"
echo "  git tag -d v$NEW_VERSION"
