#!/usr/bin/env bats

# Basic test suite for pursor
# Uses bats-core testing framework

# Setup function run before each test
setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export PURSOR_SCRIPT="$TEST_DIR/pursor.sh"
    export LIB_DIR="$TEST_DIR/lib"
    
    # Ensure we're in a git repository for testing
    if [ ! -d ".git" ]; then
        git init . >/dev/null 2>&1 || true
        git config user.email "test@example.com" >/dev/null 2>&1 || true
        git config user.name "Test User" >/dev/null 2>&1 || true
        
        # Create initial commit if no commits exist
        if ! git rev-parse HEAD >/dev/null 2>&1; then
            touch README.md
            git add README.md >/dev/null 2>&1
            git commit -m "Initial commit" >/dev/null 2>&1 || true
        fi
    fi
}

# Test that pursor.sh exists and is executable
@test "pursor.sh exists and is executable" {
    [ -f "$PURSOR_SCRIPT" ]
    [ -x "$PURSOR_SCRIPT" ]
}
