#!/bin/bash
set -e

# Create a temporary directory
TEST_REPO=$(mktemp -d)
echo "Test repo: $TEST_REPO"
cd "$TEST_REPO"

# Mock Cursor
export CURSOR_CMD="true"

# Initialize git repo
git init
git config user.email "test@example.com"  
git config user.name "Test User"
git config core.hooksPath /dev/null
echo "Initial content" > test-file.py
git add test-file.py
git commit -m "Initial commit"

# Get pursor script path
PURSOR_SCRIPT="/Users/mariusw/Documents/git/test-codebase/pursor.sh"

# Create session
echo "=== Creating session ==="
"$PURSOR_SCRIPT"

# Find session directory
session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
echo "Session dir: $session_dir"
test -d "$session_dir" && echo "Session dir exists" || echo "Session dir does not exist"

# Edit file
cd "$session_dir"
echo "Modified in session" >> test-file.py
echo "=== Merging ==="

# Merge
"$PURSOR_SCRIPT" merge "Test commit"

# Go back and check
cd "$TEST_REPO"
echo "=== After merge ==="
test -d "$session_dir" && echo "Session dir still exists" || echo "Session dir cleaned up"

# Cleanup
rm -rf "$TEST_REPO" 