#!/bin/bash
# Update past GitHub releases with CHANGELOG content
# This script updates all existing releases to include their CHANGELOG sections

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CHANGELOG_FILE="$REPO_ROOT/CHANGELOG.md"

# Get all release tags (tag name is in the 3rd column, tab-separated)
TAGS=$(gh release list --limit 100 | awk -F'\t' '{print $3}')

for TAG in $TAGS; do
  echo "Processing release $TAG..."

  # Get current release body
  CURRENT_BODY=$(gh release view "$TAG" --json body --jq .body)

  # Extract CHANGELOG content for this version
  CHANGELOG_CONTENT=$("$SCRIPT_DIR/extract_changelog.sh" "$TAG" "$CHANGELOG_FILE")

  if [ -z "$CHANGELOG_CONTENT" ]; then
    echo "  No CHANGELOG content found for $TAG, skipping..."
    continue
  fi

  # Check if CHANGELOG content is already in the release notes
  if echo "$CURRENT_BODY" | grep -q "### Bug Fixes\|### Features\|### Documentation\|### Refactor"; then
    echo "  CHANGELOG content already present in $TAG, skipping..."
    continue
  fi

  # Use only CHANGELOG content for release notes
  echo "$CHANGELOG_CONTENT" | gh release edit "$TAG" --notes-file -
  echo "  Updated release $TAG"
done

echo "Done updating all releases!"
