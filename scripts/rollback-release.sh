#!/bin/bash
# Rollback a release preparation (before push only)
#
# Usage:
#   ./scripts/rollback-release.sh <VERSION>
#
# This script safely rolls back a release that was prepared locally
# but NOT YET PUSHED to remote.
#
# Safety checks:
#   - Ensures tag wasn't pushed to remote
#   - Verifies last commit is a version bump
#   - Removes local commit and tag atomically

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ -z "$1" ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: $0 <VERSION>"
    echo "Example: $0 0.1.15"
    exit 1
fi

VERSION=$1

echo -e "${YELLOW}=== Release Rollback ===${NC}"
echo "Version: $VERSION"
echo

# Safety check 1: Ensure tag wasn't pushed
echo "Checking if tag was pushed to remote..."
if git ls-remote --tags origin | grep -q "refs/tags/v$VERSION"; then
    echo -e "${RED}ERROR: Tag v$VERSION already pushed to remote!${NC}"
    echo
    echo "The tag exists on origin. Rolling back is DANGEROUS."
    echo
    echo "Options:"
    echo "  1. If CI hasn't published yet: Delete remote tag and force-push"
    echo "     git push origin :refs/tags/v$VERSION"
    echo "     git push --force-with-lease origin main~1:main"
    echo
    echo "  2. If already published: Release new patch version with fixes"
    echo "     cargo yank --vers $VERSION agtrace  # Mark as broken"
    echo "     ./scripts/prepare-release.sh <NEXT_VERSION>"
    echo
    exit 1
fi
echo -e "${GREEN}✓ Tag not pushed yet (safe to rollback)${NC}"
echo

# Safety check 2: Ensure last commit is version bump
echo "Verifying last commit is version bump..."
LAST_MSG=$(git log -1 --pretty=%B)
if [[ ! $LAST_MSG =~ "bump version to $VERSION" ]]; then
    echo -e "${RED}ERROR: Last commit doesn't look like version bump${NC}"
    echo "Last commit message: $LAST_MSG"
    echo
    echo "This might not be the commit you want to remove."
    echo "Please verify manually: git log -1"
    exit 1
fi
echo -e "${GREEN}✓ Last commit is version bump for v$VERSION${NC}"
echo

# Safety check 3: Ensure tag exists locally
if ! git tag -l | grep -q "^v$VERSION$"; then
    echo -e "${YELLOW}Warning: Tag v$VERSION doesn't exist locally${NC}"
    echo "Will only rollback commit."
fi

# Show what will be removed
echo "The following will be rolled back:"
echo
git --no-pager log -1 --stat HEAD
echo

echo -e "${YELLOW}Proceed with rollback? (y/n)${NC}"
read -r response
if [[ ! $response =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Perform rollback
echo
echo "Rolling back..."

# Remove commit
git reset --hard HEAD~1

# Remove tag (if exists)
if git tag -l | grep -q "^v$VERSION$"; then
    git tag -d "v$VERSION"
    echo -e "${GREEN}✓ Removed tag v$VERSION${NC}"
fi

echo -e "${GREEN}✓ Removed commit${NC}"
echo
echo -e "${GREEN}✅ Rollback complete${NC}"
echo
echo "You can now:"
echo "  - Fix issues and retry: ./scripts/prepare-release.sh $VERSION"
echo "  - Prepare different version: ./scripts/prepare-release.sh <OTHER_VERSION>"
