#!/bin/bash
# Script to set up GitHub labels for agtrace
# Usage: ./scripts/setup-labels.sh

set -e

LABELS_FILE=".github/labels.json"

if [ ! -f "$LABELS_FILE" ]; then
  echo "Error: $LABELS_FILE not found"
  exit 1
fi

echo "Setting up GitHub labels from $LABELS_FILE..."

# Read and create each label
jq -c '.[]' "$LABELS_FILE" | while read -r label; do
  name=$(echo "$label" | jq -r '.name')
  description=$(echo "$label" | jq -r '.description')
  color=$(echo "$label" | jq -r '.color')

  echo "Creating label: $name"

  # Try to create the label, ignore if it already exists
  gh label create "$name" \
    --description "$description" \
    --color "$color" \
    2>/dev/null || echo "  Label '$name' already exists (skipping)"
done

echo ""
echo "Label setup complete!"
echo ""
echo "Current labels:"
gh label list
