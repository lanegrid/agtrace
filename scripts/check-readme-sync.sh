#!/bin/bash
# Check that README examples are up-to-date and compile correctly
set -e

echo "=== README Sync Checker ==="
echo

# 1. Check SDK README is up-to-date with rustdoc (via cargo-rdme)
echo "📚 Checking SDK README is synchronized with rustdoc..."
if ! command -v cargo-rdme &> /dev/null; then
    echo "⚠️  cargo-rdme not found. Install with: cargo install cargo-rdme"
    echo "   Skipping README sync check..."
else
    cargo rdme --workspace-project agtrace-sdk --check
    echo "✓ SDK README is up-to-date with rustdoc"
fi
echo

# 2. Check SDK README examples compile (via include_str! in lib.rs)
echo "📚 Checking SDK README examples compile..."
cargo test --doc -p agtrace-sdk --quiet
echo "✓ SDK README examples compile successfully"
echo

# 3. Check for outdated version references
echo "🔍 Checking version references..."
CURRENT_VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "agtrace-sdk") | .version')
echo "   Current agtrace-sdk version: $CURRENT_VERSION"

# SDK README should use "0.1" (major.minor only) or exact version
if grep -q 'agtrace-sdk = "0\.1"' crates/agtrace-sdk/README.md; then
    echo "✓ SDK README uses major.minor version (recommended)"
elif grep -q "agtrace-sdk = \"$CURRENT_VERSION\"" crates/agtrace-sdk/README.md; then
    echo "✓ SDK README uses exact current version"
else
    echo "✗ Version mismatch in SDK README"
    echo "   Expected: agtrace-sdk = \"0.1\" or \"$CURRENT_VERSION\""
    echo "   Found:"
    grep 'agtrace-sdk = ' crates/agtrace-sdk/README.md || echo "   (no version found)"
    exit 1
fi

# Root README should also use consistent versioning
if grep -q 'agtrace-sdk = "0\.1"' README.md; then
    echo "✓ Root README uses major.minor version (recommended)"
elif grep -q "agtrace-sdk = \"$CURRENT_VERSION\"" README.md; then
    echo "✓ Root README uses exact current version"
else
    echo "✗ Version mismatch in root README"
    echo "   Expected: agtrace-sdk = \"0.1\" or \"$CURRENT_VERSION\""
    echo "   Found:"
    grep 'agtrace-sdk = ' README.md || echo "   (no version found)"
    exit 1
fi

echo
echo "✅ All README checks passed!"
