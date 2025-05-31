#!/usr/bin/env bats

# Integration test suite for pursor
# Tests complete workflows in isolated temporary directories

setup() {
    # Create fresh temporary directory for each test
    export TEST_REPO=$(mktemp -d)
    cd "$TEST_REPO"
    
    # Mock Cursor IDE - don't actually open it
    export CURSOR_CMD="true"
    
    # Set up minimal git config
    git init
    git config user.email "test@example.com"
    git config user.name "Test User"
    
    # Disable git hooks in test repositories to prevent interference
    git config core.hooksPath /dev/null
    
    # Create initial commit
    echo "Initial content" > test-file.py
    git add test-file.py
    git commit -m "Initial commit"
    
    # Get absolute path to pursor script
    export PURSOR_SCRIPT="$(dirname "${BATS_TEST_DIRNAME}")/pursor.sh"
}

teardown() {
    # Clean up temporary directory
    if [ -n "$TEST_REPO" ] && [ -d "$TEST_REPO" ]; then
        cd /
        rm -rf "$TEST_REPO"
    fi
}

# Helper function to run pursor
run_pursor() {
    "$PURSOR_SCRIPT" "$@"
}

@test "IT-1: Happy path - create session, edit file, merge successfully" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify session was created
    [ -d "subtrees" ]
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_dir" ]
    [ -d "$session_dir" ]
    
    # Verify state tracking
    [ -d ".pursor_state" ]
    [ "$(ls .pursor_state | wc -l)" -eq 1 ]
    
    # 2. Edit file in worktree
    cd "$session_dir"
    echo "Modified in session" >> test-file.py
    
    # 3. Merge with message
    run run_pursor merge "Integration test commit"
    [ "$status" -eq 0 ]
    
    # Go back to main repo
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    git log --oneline | grep "Integration test commit"
    
    # Verify changes are in main
    grep "Modified in session" test-file.py
    
    # Verify cleanup - either directory doesn't exist or has no .state files
    [ ! -d "$session_dir" ]
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
}

@test "IT-2: Cancel session" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # Get branch name
    cd "$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Cancel from within worktree
    run run_pursor cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    [ ! -d "$session_dir" ]
    
    # Verify state file removed - either directory doesn't exist or has no .state files
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
}

@test "IT-3: Conflict resolution with continue" {
    # 1. Create session A and edit file
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Store session A path immediately
    session_a=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_a" ]
    [ -d "$session_a" ]
    
    cd "$session_a"
    
    # Edit the existing file - replace the line with session A content
    echo "Change from session A" > test-file.py
    
    # Go back to main repo
    cd "$TEST_REPO"
    
    # Small delay to ensure different timestamps
    sleep 1
    
    # 2. Create session B and edit same line
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Store session B path immediately - find the one that's NOT session A
    session_b=""
    for dir in $(find subtrees/pc -maxdepth 1 -type d -name "20*"); do
        if [ "$dir" != "$session_a" ]; then
            session_b="$dir"
            break
        fi
    done
    [ -n "$session_b" ]
    [ -d "$session_b" ]
    
    cd "$session_b"
    
    # Edit the same file in a conflicting way - replace with different content
    echo "Change from session B" > test-file.py
    
    # Go back to main repo
    cd "$TEST_REPO"
    
    # 3. Merge A (should succeed)
    cd "$session_a"
    run run_pursor merge "Session A changes"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 4. Try to merge B (should have conflict)
    # Session B directory should still exist even after A is cleaned up
    [ -d "$session_b" ]
    cd "$session_b"
    run run_pursor merge "Session B changes"
    # The merge command succeeds in setting up the conflict state
    [ "$status" -eq 0 ]
    # But it should output conflict information
    [[ "$output" == *"conflict"* ]]
    
    # The session should still be active (not cleaned up due to conflict)
    cd "$TEST_REPO"
    [ -d "$session_b" ]
    if [ -d ".pursor_state" ]; then
        # Should still have the session B state file
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 1 ]
    fi
    
    # 5. Manually resolve conflict
    cd "$session_b"
    # Remove conflict markers and combine both changes
    cat > test-file.py << 'EOF'
Change from session A
Change from session B
EOF
    
    # Continue the merge
    run run_pursor continue
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify both changes are in history
    git log --oneline | grep "Session A changes"
    git log --oneline | grep "Session B changes"
    
    # Verify final content has both changes
    grep "Change from session A" test-file.py
    grep "Change from session B" test-file.py
    
    # Verify no active sessions after resolution - either directory doesn't exist or has no .state files
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
}

@test "IT-4: Clean all sessions" {
    # 1. Create two sessions
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify first session was created
    [ -d "subtrees" ]
    [ -d ".pursor_state" ]
    first_session_count=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | wc -l)
    [ "$first_session_count" -eq 1 ]
    
    # Small delay to ensure different timestamps
    sleep 1
    
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify two sessions exist
    session_count=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | wc -l)
    [ "$session_count" -eq 2 ]
    
    # Verify state files exist
    [ -d ".pursor_state" ]
    state_count=$(find .pursor_state -name '*.state' | wc -l)
    [ "$state_count" -eq 2 ]
    
    # 2. Clean all sessions
    run run_pursor clean
    [ "$status" -eq 0 ]
    
    # Verify all worktrees removed
    session_count=$(find subtrees/pc -maxdepth 1 -type d -name "20*" 2>/dev/null | wc -l || echo 0)
    [ "$session_count" -eq 0 ]
    
    # Verify state files removed - either directory doesn't exist or has no .state files
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
    
    # Verify original repo is untouched
    [ -f "test-file.py" ]
    git log --oneline | grep "Initial commit"
}

@test "IT-5: Auto-detect session from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # 2. Navigate to worktree and edit file
    cd "$session_dir"
    echo "Auto-detect test change" >> test-file.py
    
    # 3. Test merge from within worktree (should auto-detect session)
    run run_pursor merge "Auto-detect test commit"
    [ "$status" -eq 0 ]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    git log --oneline | grep "Auto-detect test commit"
    
    # Verify changes are in main
    grep "Auto-detect test change" test-file.py
    
    # Verify cleanup - session should be cleaned up after successful merge
    [ ! -d "$session_dir" ]
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
}

@test "IT-6: Auto-detect session for cancel from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # Get the branch name before navigating
    cd "$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Test cancel from within worktree (should auto-detect session)
    run run_pursor cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    [ ! -d "$session_dir" ]
    
    # Verify state file removed
    if [ -d ".pursor_state" ]; then
        [ "$(find .pursor_state -name '*.state' | wc -l)" -eq 0 ]
    fi
} 