#!/bin/bash
set -e

echo "Testing Para Sandbox Profiles..."

# Function to test a profile
test_profile() {
    local profile=$1
    echo -e "\nTesting profile: $profile"
    
    # Extract the profile to verify it works
    cargo run -- start test-$profile --sandbox --sandbox-profile $profile --dry-run 2>&1 | grep -E "(Sandboxing enabled|profile)" || echo "No sandbox output found"
}

# Build the binary
echo "Building para..."
cargo build --quiet

# Test each profile
test_profile "permissive-open"
test_profile "permissive-closed"
test_profile "restrictive-closed"

# Test invalid profile
echo -e "\nTesting invalid profile (should fail gracefully)..."
cargo run -- start test-invalid --sandbox --sandbox-profile invalid-profile --dry-run 2>&1 | grep -i "error\|unknown" || echo "No error message found"

echo -e "\nProfile tests complete!"