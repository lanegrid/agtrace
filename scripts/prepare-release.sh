#!/bin/bash
# Prepare a new release of agtrace
#
# Usage:
#   ./scripts/prepare-release.sh              # Interactive mode
#   ./scripts/prepare-release.sh 0.1.15       # Specify version
#   ./scripts/prepare-release.sh 0.1.15 -y    # Auto-confirm all prompts
#   ./scripts/prepare-release.sh --dry-run    # Preview changes
#
# This script does everything needed for a release:
#   1. Validates working directory
#   2. Runs all tests
#   3. Validates and auto-fixes README version references
#   4. Updates version numbers
#   5. Generates CHANGELOG
#   6. Runs fmt and clippy
#   7. Creates commit and git tag
#
# Idempotency: This script can be safely re-run if it fails partway through.
# It detects partial progress and resumes from where it left off.
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
AUTO_YES=false
NEW_VERSION=""

for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --yes|-y)
            AUTO_YES=true
            shift
            ;;
        *)
            NEW_VERSION=$arg
            shift
            ;;
    esac
done

# Get current version from Cargo.toml
CARGO_VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "agtrace") | .version')

# Get latest git tag (represents last released version)
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [ -n "$LAST_TAG" ]; then
    RELEASED_VERSION=${LAST_TAG#v}  # Remove 'v' prefix
else
    RELEASED_VERSION="0.0.0"
fi

echo -e "${BLUE}=== agtrace Release Preparation ===${NC}"
echo
echo "Last released:   $RELEASED_VERSION (git tag: ${LAST_TAG:-none})"
echo "Cargo.toml:      $CARGO_VERSION"

# If no version specified, prompt
if [ -z "$NEW_VERSION" ]; then
    echo -e "${YELLOW}Enter new version number (e.g., 0.1.15):${NC}"
    read NEW_VERSION
fi

echo "Target version:  $NEW_VERSION"
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

# Detect resume mode: Cargo.toml already updated but not yet committed
RESUME_MODE=false
if [ "$CARGO_VERSION" = "$NEW_VERSION" ] && [ "$RELEASED_VERSION" != "$NEW_VERSION" ]; then
    RESUME_MODE=true
    echo -e "${YELLOW}📍 Resume mode detected: Cargo.toml already updated to $NEW_VERSION${NC}"
    echo -e "${YELLOW}   Continuing from where we left off...${NC}"
    echo
fi

# Check if version is newer than last release
if [ "$NEW_VERSION" = "$RELEASED_VERSION" ]; then
    # Check if tag already exists
    if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
        echo -e "${RED}Error: Version v$NEW_VERSION is already released (tag exists)${NC}"
        exit 1
    fi
fi

# Validate we're moving forward (unless in resume mode)
if [ "$RESUME_MODE" = false ]; then
    if [ "$NEW_VERSION" = "$CARGO_VERSION" ]; then
        echo -e "${RED}Error: Cargo.toml already at version $NEW_VERSION but no resume state detected${NC}"
        echo -e "${RED}This shouldn't happen. Check git status and working directory state.${NC}"
        exit 1
    fi
fi

echo -e "${BLUE}Step 1/7: Checking working directory${NC}"
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${YELLOW}Warning: Working directory has uncommitted changes${NC}"
    git status --short
    echo
    if [ "$DRY_RUN" = false ] && [ "$AUTO_YES" = false ]; then
        echo -e "${YELLOW}Continue anyway? (y/n)${NC}"
        read -r response
        if [[ ! $response =~ ^[Yy]$ ]]; then
            echo "Aborted."
            exit 1
        fi
    elif [ "$AUTO_YES" = true ]; then
        echo -e "${YELLOW}Continuing automatically (--yes flag)...${NC}"
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
    # Use --fix to automatically update version references in READMEs
    ./scripts/check-readme-sync.sh --fix
fi
echo -e "${GREEN}✓ README validation passed${NC}"
echo

echo -e "${BLUE}Step 4/7: Updating version numbers${NC}"
if [ "$RESUME_MODE" = true ]; then
    echo -e "${YELLOW}  Already updated (resume mode), skipping...${NC}"
elif [ "$DRY_RUN" = false ]; then
    # Update workspace version
    sed -i.bak "s/^version = \"$CARGO_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
    rm Cargo.toml.bak

    # Update all workspace dependency versions
    sed -i.bak "s/version = \"$CARGO_VERSION\", path = \"crates\//version = \"$NEW_VERSION\", path = \"crates\//" Cargo.toml
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
if [ "$RESUME_MODE" = true ]; then
    echo "  - Version: $RELEASED_VERSION → $NEW_VERSION (already in Cargo.toml)"
else
    echo "  - Version: $RELEASED_VERSION → $NEW_VERSION"
fi
echo "  - CHANGELOG.md updated"
echo

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}This was a dry run. No changes were made.${NC}"
    exit 0
fi

if [ "$AUTO_YES" = false ]; then
    echo -e "${YELLOW}Create release commit and tag? (y/n)${NC}"
    read -r response
    if [[ ! $response =~ ^[Yy]$ ]]; then
        echo "Aborted. Changes have been made but not committed."
        echo "To undo: git checkout Cargo.toml CHANGELOG.md"
        exit 1
    fi
else
    echo -e "${YELLOW}Creating release commit and tag automatically (--yes flag)...${NC}"
fi

# Commit and tag
git add Cargo.toml CHANGELOG.md README.md crates/agtrace-sdk/README.md
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
