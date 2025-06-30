#!/bin/bash

# Script to test both sandbox security profiles
set -e

echo "======================================"
echo "Testing Para Sandbox Security Profiles"
echo "======================================"
echo ""

# Test 1: Permissive Profile
echo "1. Testing PERMISSIVE profile:"
echo "   - Should allow writing to: project dir, cargo, rustup, para config"
echo "   - Should block writing to: other directories"
echo ""

# Create session with permissive profile
para dispatch test-permissive --sandbox --sandbox-profile permissive <<'EOF'
Test the permissive sandbox profile by attempting various file operations:

1. Try to write a file in the current project directory (should succeed)
2. Try to create a directory in ~/.cargo (should succeed)
3. Try to write to ~/.rustup (should succeed)
4. Try to write to ~/.para (should succeed)
5. Try to write to /etc/hosts (should fail)
6. Try to write to ~/Desktop (should fail)
7. Try to write to another user's home directory (should fail)

Please report each operation and whether it succeeded or failed.
EOF

echo ""
echo "======================================"
echo ""

# Test 2: Restrictive Profile  
echo "2. Testing RESTRICTIVE profile:"
echo "   - Should ONLY allow writing to: project dir, temp files, Claude config"
echo "   - Should block writing to: cargo, rustup, home directory"
echo ""

# Create session with restrictive profile
para dispatch test-restrictive --sandbox --sandbox-profile restrictive <<'EOF'
Test the restrictive sandbox profile by attempting various file operations:

1. Try to write a file in the current project directory (should succeed)
2. Try to create a directory in ~/.cargo (should FAIL)
3. Try to write to ~/.rustup (should FAIL)
4. Try to write to ~/.para (should FAIL)
5. Try to write to /tmp (should succeed)
6. Try to write to ~/.claude (should succeed - required for Claude)
7. Try to write to ~/Desktop (should FAIL)
8. Try to read from ~/.cargo (should succeed - read is allowed)

Please report each operation and whether it succeeded or failed.
EOF

echo ""
echo "======================================"
echo "Both profiles have been dispatched."
echo "Wait for the agents to complete and check their reports."
echo "======================================"