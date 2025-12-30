#!/bin/bash
# Extract a specific version section from CHANGELOG.md
# Usage: extract_changelog.sh VERSION CHANGELOG_FILE

VERSION=$1
CHANGELOG_FILE=$2

if [ -z "$VERSION" ] || [ -z "$CHANGELOG_FILE" ]; then
  echo "Usage: extract_changelog.sh VERSION CHANGELOG_FILE"
  exit 1
fi

# Remove 'v' prefix if present
VERSION_NUM=${VERSION#v}

# Extract the section for this version
# Start from ## [VERSION] and continue until the next ## [
awk -v version="$VERSION_NUM" '
  /^## \['"$VERSION_NUM"'\]/ {found=1; next}
  found && /^## \[/ {exit}
  found {print}
' "$CHANGELOG_FILE"
