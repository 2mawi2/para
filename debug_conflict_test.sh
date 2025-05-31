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

# Create initial files
echo "Initial content" > test-file.py
git add test-file.py
git commit -m "Initial commit"

# Create a justfile that will conflict
cat > justfile << 'EOF'
# Original justfile content
test:
    @echo "Running original tests..."
    @bats test.bats
EOF
git add justfile
git commit -m "Add original justfile"

# Get pursor script path
PURSOR_SCRIPT="/Users/mariusw/Documents/git/test-codebase/pursor.sh"

echo "=== Creating session A ==="
"$PURSOR_SCRIPT"

# Find session directory
session_a=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
echo "Session A dir: $session_a"

# Modify justfile in session A
cd "$session_a"
cat > justfile << 'EOF'
# Modified justfile content A
test:
    @echo "Running modified tests A..."
    @bats test_a.bats
EOF

cd "$TEST_REPO"

# Modify main branch to create conflict
cat > justfile << 'EOF'
# Modified justfile content B
test:
    @echo "Running modified tests B..."
    @bats test_b.bats
EOF
git add justfile
git commit -m "Modify justfile in main"

echo "=== Attempting merge (should fail) ==="
cd "$session_a"
set +e
"$PURSOR_SCRIPT" merge "Modify justfile in session A"
merge_status=$?
set -e

echo "Merge status: $merge_status"

if [ $merge_status -eq 1 ]; then
    echo "=== Merge failed as expected, checking conflict state ==="
    
    # Check if justfile has conflict markers
    if grep -q "<<<<<<< HEAD" justfile; then
        echo "✅ Conflict markers found in justfile"
        cat justfile
    else
        echo "❌ No conflict markers found in justfile"
        cat justfile
    fi
    
    echo "=== Attempting continue without resolving ==="
    set +e
    "$PURSOR_SCRIPT" continue
    continue_status=$?
    set -e
    
    echo "Continue status: $continue_status"
    
    if [ $continue_status -eq 1 ]; then
        echo "✅ Continue correctly failed with unresolved conflicts"
    else
        echo "❌ Continue should have failed but didn't"
    fi
fi

# Cleanup
rm -rf "$TEST_REPO" 