#!/usr/bin/env bash

# Common test environment setup functions for para tests
# This file should be sourced by test files to reuse environment setup

# Setup a temporary git repository for testing
# Sets up the following environment variables:
# - TEST_REPO: Path to temporary repository
# - PARA_SCRIPT: Path to para script
# - ORIGINAL_DIR: Original working directory (for teardown)
# - ORIGINAL_REPO_ROOT: Original REPO_ROOT (for teardown)
setup_temp_git_repo() {
    export ORIGINAL_DIR="$PWD"
    export ORIGINAL_REPO_ROOT="$REPO_ROOT"
    
    export TEST_REPO=$(mktemp -d)
    
    # Set up comprehensive IDE mocking to prevent opening actual IDEs
    export IDE_CMD="echo 'mock-ide-launched'"
    export CURSOR_CMD="echo 'mock-cursor-launched'"  # Backwards compatibility
    
    # Enable non-interactive mode for tests
    export PARA_NON_INTERACTIVE="true"
    
    # Mock all possible IDE commands
    export MOCK_BIN_DIR="$TEST_REPO/.mock_bin"
    mkdir -p "$MOCK_BIN_DIR"
    
    # Create mock IDE executables
    cat > "$MOCK_BIN_DIR/cursor" << 'EOF'
#!/bin/sh
echo "mock-cursor-launched: $*" >&2
exit 0
EOF
    
    cat > "$MOCK_BIN_DIR/claude" << 'EOF'
#!/bin/sh
echo "mock-claude-launched: $*" >&2
exit 0
EOF
    
    cat > "$MOCK_BIN_DIR/code" << 'EOF'
#!/bin/sh
echo "mock-code-launched: $*" >&2
exit 0
EOF
    
    # Make them executable
    chmod +x "$MOCK_BIN_DIR/cursor"
    chmod +x "$MOCK_BIN_DIR/claude"
    chmod +x "$MOCK_BIN_DIR/code"
    
    # Add mock bin to PATH at the beginning so it takes precedence
    export PATH="$MOCK_BIN_DIR:$PATH"
    
    # Clear any existing para configuration to ensure clean test state
    unset REPO_ROOT
    unset STATE_DIR
    unset SUBTREES_DIR
    unset BASE_BRANCH
    unset SUBTREES_DIR_NAME
    unset STATE_DIR_NAME
    unset IDE_NAME
    unset IDE_USER_DATA_DIR
    
    # Set branch prefix to para for tests
    export PARA_BRANCH_PREFIX="para"
    
    (
        cd "$TEST_REPO"
        git init
        git config user.email "test@example.com"
        git config user.name "Test User"
        
        git config core.hooksPath /dev/null
        
        echo "Initial content" > test-file.py
        git add test-file.py
        git commit -m "Initial commit"
    )
    
    if [ -n "${BATS_TEST_DIRNAME:-}" ]; then
        export PARA_SCRIPT="$(dirname "${BATS_TEST_DIRNAME}")/para.sh"
    else
        local script_dir="$(cd "$(dirname "$0")" && pwd)"
        
        if [ -f "$script_dir/para.sh" ]; then
            export PARA_SCRIPT="$script_dir/para.sh"
        else
            local current_dir="$script_dir"
            while [ "$current_dir" != "/" ]; do
                if [ -f "$current_dir/para.sh" ]; then
                    export PARA_SCRIPT="$current_dir/para.sh"
                    break
                fi
                current_dir="$(dirname "$current_dir")"
            done
        fi
        
        if [ -z "${PARA_SCRIPT:-}" ]; then
            echo "Error: Could not find para.sh from $script_dir" >&2
            return 1
        fi
    fi
}

# Teardown temporary environment
# Cleans up TEST_REPO and restores original environment
teardown_temp_git_repo() {
    # Restore original PATH by removing the mock bin directory
    if [ -n "$MOCK_BIN_DIR" ]; then
        export PATH="${PATH#$MOCK_BIN_DIR:}"
    fi
    
    if [ -n "$TEST_REPO" ] && [ -d "$TEST_REPO" ]; then
        rm -rf "$TEST_REPO"
    fi
    
    cd "$ORIGINAL_DIR" 2>/dev/null || true
    export REPO_ROOT="$ORIGINAL_REPO_ROOT"
    
    # Clean up test-specific environment variables
    unset MOCK_BIN_DIR
    unset IDE_CMD
    unset CURSOR_CMD
}

# Helper function to run para in the test directory
run_para() {
    if [ "$#" -eq 0 ]; then
        # No arguments - use start command
        cd "$TEST_REPO" && "$PARA_SCRIPT" start
    else
        cd "$TEST_REPO" && "$PARA_SCRIPT" "$@"
    fi
}

# Helper function to run commands in test directory
run_in_test_repo() {
    cd "$TEST_REPO" && "$@"
}

# Setup a git repository with multiple initial files
# Useful for conflict testing
setup_temp_git_repo_with_files() {
    setup_temp_git_repo
    
    (
        cd "$TEST_REPO"
        
        cat > justfile << 'EOF'
# Original justfile content
test:
    @echo "Running original tests..."
    @bats test.bats
EOF
        git add justfile
        git commit -m "Add original justfile"
        
        echo "# README" > README.md
        echo "node_modules/" > .gitignore
        git add README.md .gitignore
        git commit -m "Add README and gitignore"
    )
}

# Find the first session directory in a test repo
# Returns the path relative to TEST_REPO
find_session_dir() {
    cd "$TEST_REPO"
    # Look for sessions in both new wip namespace and legacy locations
    find subtrees/para -type d \( -name "*_*_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "*-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" \) 2>/dev/null | head -1
}

# Count active sessions in test repo
count_sessions() {
    cd "$TEST_REPO"
    # Count sessions in both new wip namespace and legacy locations
    find subtrees/para -type d \( -name "*_*_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "*-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" \) 2>/dev/null | wc -l
}

# Count state files in test repo
count_state_files() {
    cd "$TEST_REPO"
    find .para_state -name '*.state' 2>/dev/null | wc -l || echo 0
}

# Create a conflicting change in main branch
# Usage: create_main_conflict <filename> <content>
create_main_conflict() {
    local filename="$1"
    local content="$2"
    
    cd "$TEST_REPO"
    echo "$content" > "$filename"
    git add "$filename"
    git commit -m "Create conflict in main: $filename"
}

# Assert that a session directory exists
assert_session_exists() {
    local session_dir="$1"
    if [ ! -d "$TEST_REPO/$session_dir" ]; then
        echo "Expected session directory $session_dir to exist but it doesn't"
        return 1
    fi
}

# Assert that a session directory does not exist
assert_session_not_exists() {
    local session_dir="$1"
    if [ -d "$TEST_REPO/$session_dir" ]; then
        echo "Expected session directory $session_dir to not exist but it does"
        return 1
    fi
}

# Assert that a commit with message exists in git log
assert_commit_exists() {
    local commit_message="$1"
    cd "$TEST_REPO"
    if ! git log --oneline | grep -q "$commit_message"; then
        echo "Expected commit with message '$commit_message' to exist in git log"
        return 1
    fi
}

# Assert that a file contains specific content
assert_file_contains() {
    local filename="$1"
    local content="$2"
    if ! grep -q "$content" "$filename"; then
        echo "Expected file $filename to contain '$content'"
        return 1
    fi
} 