#!/bin/bash
set -e

echo "Testing Para Sandboxing API..."

# Build the binary
cargo build

# Test 1: CLI flag --no-sandbox overrides everything
echo -e "\n1. Testing --no-sandbox flag (should override config and env)..."
export PARA_SANDBOX=true
export PARA_SANDBOX_PROFILE=restrictive-closed
./target/debug/para start test-no-sandbox --no-sandbox --dry-run 2>&1 | grep -i sandbox || echo "No sandbox message found"

# Test 2: CLI flag --sandbox overrides config
echo -e "\n2. Testing --sandbox flag with profile..."
unset PARA_SANDBOX
unset PARA_SANDBOX_PROFILE
./target/debug/para start test-sandbox --sandbox --sandbox-profile permissive-closed --dry-run 2>&1 | grep -i sandbox || echo "No sandbox message found"

# Test 3: Environment variables
echo -e "\n3. Testing environment variables..."
export PARA_SANDBOX=true
export PARA_SANDBOX_PROFILE=permissive-closed
./target/debug/para start test-env --dry-run 2>&1 | grep -i sandbox || echo "No sandbox message found"

# Test 4: Config file (would need config setup)
echo -e "\n4. Config file test would require setting up config..."

# Clean up
unset PARA_SANDBOX
unset PARA_SANDBOX_PROFILE

echo -e "\nAPI test complete!"