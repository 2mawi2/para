#!/bin/bash
set -euo pipefail

# Script to run existing bats tests against the Rust binary
# This ensures behavioral compatibility during the rewrite

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

echo "▶ Building Rust binary..."
cd "$PROJECT_ROOT"
cargo build

RUST_BINARY_PATH="$PROJECT_ROOT/target/debug/para"

if [ ! -f "$RUST_BINARY_PATH" ]; then
    echo "❌ Rust binary not found at: $RUST_BINARY_PATH"
    exit 1
fi

echo "▶ Setting up test environment..."

# Check if original para.sh exists in current directory
if [ ! -f "para.sh" ]; then
    echo "❌ Original para.sh not found in: $PROJECT_ROOT"
    exit 1
fi

# Backup original para.sh
echo "▶ Backing up original para.sh..."
cp para.sh para.sh.bak

# Create shim that calls the Rust binary
echo "▶ Creating Rust binary shim..."
cat > para.sh << EOF
#!/bin/sh
exec "$RUST_BINARY_PATH" "\$@"
EOF
chmod +x para.sh

# Function to cleanup and restore original para.sh
cleanup() {
    echo "▶ Restoring original para.sh..."
    if [ -f "para.sh.bak" ]; then
        mv para.sh.bak para.sh
    fi
}

# Trap cleanup function to ensure restoration
trap cleanup EXIT

echo "▶ Running bats tests against Rust binary..."
echo "  Binary: $RUST_BINARY_PATH"
echo "  Working directory: $PROJECT_ROOT"
echo ""

# Run the bats tests
if command -v bats >/dev/null 2>&1; then
    # Run all bats tests from current directory
    bats test_*.bats 2>/dev/null || test_result=$?
else
    echo "❌ bats not found. Please install bats-core:"
    echo "  macOS: brew install bats-core"
    echo "  Ubuntu: apt-get install bats"
    exit 1
fi

# Note: cleanup() will run automatically due to trap

echo ""
echo "✅ Test run completed"

# Exit with the test result if there was one
exit ${test_result:-0}