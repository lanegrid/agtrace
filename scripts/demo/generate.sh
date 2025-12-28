#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

echo "==> Checking agtrace installation..."
if ! command -v agtrace &> /dev/null; then
    echo "agtrace not found. Installing from local source..."
    cargo install --path crates/agtrace-cli
else
    INSTALLED_VERSION=$(agtrace --version | awk '{print $2}')
    echo "Found agtrace version: $INSTALLED_VERSION"
fi

echo "==> Checking VHS installation..."
if ! command -v vhs &> /dev/null; then
    echo "Error: VHS is not installed."
    echo "Please install VHS: https://github.com/charmbracelet/vhs"
    exit 1
fi

echo "==> Removing old demo.gif if exists..."
rm -f demo.gif

echo "==> Generating demo.gif..."
cd "$SCRIPT_DIR"
vhs demo.tape

echo "==> Moving demo.gif to project root..."
mv demo.gif "$PROJECT_ROOT/"

cd "$PROJECT_ROOT"
echo "==> Done! demo.gif created successfully."
ls -lh demo.gif
