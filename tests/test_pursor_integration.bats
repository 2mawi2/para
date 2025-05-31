#!/usr/bin/env bats

# Integration test suite for pursor
# Tests complete workflows in isolated temporary directories

setup() {
    # Save current directory and environment for restoration
    export ORIGINAL_DIR="$PWD"
    export ORIGINAL_REPO_ROOT="$REPO_ROOT"
    
    # Create fresh temporary directory for each test
    export TEST_REPO=$(mktemp -d)
    
    # Mock Cursor IDE - don't actually open it
    export CURSOR_CMD="true"
    
    # Clear any pursor-related environment variables to ensure isolation
    unset REPO_ROOT
    unset STATE_DIR
    unset SUBTREES_DIR
    unset BASE_BRANCH
    unset SUBTREES_DIR_NAME
    unset STATE_DIR_NAME
    
    # Set up minimal git config in the isolated test directory
    (
        cd "$TEST_REPO"
        git init
        git config user.email "test@example.com"
        git config user.name "Test User"
        
        # Disable git hooks in test repositories to prevent interference
        git config core.hooksPath /dev/null
        
        # Create initial commit
        echo "Initial content" > test-file.py
        git add test-file.py
        git commit -m "Initial commit"
    )
    
    # Get absolute path to pursor script
    export PURSOR_SCRIPT="$(dirname "${BATS_TEST_DIRNAME}")/pursor.sh"
}

teardown() {
    # Clean up temporary directory
    if [ -n "$TEST_REPO" ] && [ -d "$TEST_REPO" ]; then
        rm -rf "$TEST_REPO"
    fi
    
    # Restore original directory and environment
    cd "$ORIGINAL_DIR" 2>/dev/null || true
    export REPO_ROOT="$ORIGINAL_REPO_ROOT"
}

# Helper function to run pursor in the test directory
run_pursor() {
    cd "$TEST_REPO" && "$PURSOR_SCRIPT" "$@"
}

# Helper function to run commands in test directory
run_in_test_repo() {
    cd "$TEST_REPO" && "$@"
}

@test "IT-1: Happy path - create session, edit file, merge successfully" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify session was created (all checks in test directory)
    run_in_test_repo test -d "subtrees"
    [ "$?" -eq 0 ]
    
    session_dir=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_dir" ]
    run_in_test_repo test -d "$session_dir"
    [ "$?" -eq 0 ]
    
    # Verify state tracking
    run_in_test_repo test -d ".pursor_state"
    [ "$?" -eq 0 ]
    session_count=$(run_in_test_repo ls .pursor_state | wc -l)
    [ "$session_count" -eq 1 ]
    
    # 2. Edit file in worktree
    (cd "$TEST_REPO/$session_dir" && echo "Modified in session" >> test-file.py)
    
    # 3. Merge with message
    run bash -c "cd '$TEST_REPO/$session_dir' && '$PURSOR_SCRIPT' merge 'Integration test commit'"
    [ "$status" -eq 0 ]
    
    # Verify commit exists on main
    run run_in_test_repo git log --oneline
    [[ "$output" == *"Integration test commit"* ]]
    
    # Verify changes are in main
    run_in_test_repo grep "Modified in session" test-file.py
    [ "$?" -eq 0 ]
    
    # Verify cleanup - either directory doesn't exist or has no .state files
    run_in_test_repo test -d "$session_dir"
    [ "$?" -ne 0 ]  # Should not exist after successful merge
}

@test "IT-2: Cancel session" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    run_in_test_repo test -d "$session_dir"
    [ "$?" -eq 0 ]
    
    # Get branch name
    branch_name=$(cd "$TEST_REPO/$session_dir" && git branch --show-current)
    
    # 2. Cancel from within worktree
    run bash -c "cd '$TEST_REPO/$session_dir' && '$PURSOR_SCRIPT' cancel"
    [ "$status" -eq 0 ]
    
    # Verify branch is deleted
    run run_in_test_repo git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    run_in_test_repo test -d "$session_dir"
    [ "$?" -ne 0 ]  # Should not exist
}

@test "IT-3: Conflict resolution with continue" {
    # 1. Create session A and edit file
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Store session A path immediately
    session_a=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_a" ]
    run_in_test_repo test -d "$session_a"
    [ "$?" -eq 0 ]
    
    # Edit the existing file - replace the line with session A content
    (cd "$TEST_REPO/$session_a" && echo "Change from session A" > test-file.py)
    
    # Small delay to ensure different timestamps
    sleep 1
    
    # 2. Create session B and edit same line
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Store session B path immediately - find the one that's NOT session A
    session_b=""
    for dir in $(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*"); do
        if [ "$dir" != "$session_a" ]; then
            session_b="$dir"
            break
        fi
    done
    [ -n "$session_b" ]
    run_in_test_repo test -d "$session_b"
    [ "$?" -eq 0 ]
    
    # Edit the same file in a conflicting way - replace with different content
    (cd "$TEST_REPO/$session_b" && echo "Change from session B" > test-file.py)
    
    # 3. Merge A (should succeed)
    run bash -c "cd '$TEST_REPO/$session_a' && '$PURSOR_SCRIPT' merge 'Session A changes'"
    [ "$status" -eq 0 ]
    
    # 4. Try to merge B (should have conflict)
    # Session B directory should still exist even after A is cleaned up
    run_in_test_repo test -d "$session_b"
    [ "$?" -eq 0 ]
    
    run bash -c "cd '$TEST_REPO/$session_b' && '$PURSOR_SCRIPT' merge 'Session B changes'"
    # The merge command succeeds in setting up the conflict state
    [ "$status" -eq 0 ]
    # But it should output conflict information
    [[ "$output" == *"conflict"* ]]
    
    # The session should still be active (not cleaned up due to conflict)
    run_in_test_repo test -d "$session_b"
    [ "$?" -eq 0 ]
    
    # 5. Manually resolve conflict
    # Remove conflict markers and combine both changes
    cat > "$TEST_REPO/$session_b/test-file.py" << 'EOF'
Change from session A
Change from session B
EOF
    
    # Continue the merge
    run bash -c "cd '$TEST_REPO/$session_b' && '$PURSOR_SCRIPT' continue"
    [ "$status" -eq 0 ]
    
    # Verify both changes are in history
    run run_in_test_repo git log --oneline
    [[ "$output" == *"Session A changes"* ]]
    [[ "$output" == *"Session B changes"* ]]
    
    # Verify final content has both changes
    run_in_test_repo grep "Change from session A" test-file.py
    [ "$?" -eq 0 ]
    run_in_test_repo grep "Change from session B" test-file.py
    [ "$?" -eq 0 ]
}

@test "IT-4: Clean all sessions" {
    # 1. Create two sessions
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify first session was created
    run_in_test_repo test -d "subtrees"
    [ "$?" -eq 0 ]
    run_in_test_repo test -d ".pursor_state"
    [ "$?" -eq 0 ]
    first_session_count=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | wc -l)
    [ "$first_session_count" -eq 1 ]
    
    # Small delay to ensure different timestamps
    sleep 1
    
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify two sessions exist
    session_count=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | wc -l)
    [ "$session_count" -eq 2 ]
    
    # Verify state files exist
    run_in_test_repo test -d ".pursor_state"
    [ "$?" -eq 0 ]
    state_count=$(run_in_test_repo find .pursor_state -name '*.state' | wc -l)
    [ "$state_count" -eq 2 ]
    
    # 2. Clean all sessions
    run run_pursor clean
    [ "$status" -eq 0 ]
    
    # Verify all worktrees removed
    session_count=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" 2>/dev/null | wc -l || echo 0)
    [ "$session_count" -eq 0 ]
    
    # Verify original repo is untouched
    run_in_test_repo test -f "test-file.py"
    [ "$?" -eq 0 ]
    run run_in_test_repo git log --oneline
    [[ "$output" == *"Initial commit"* ]]
}

@test "IT-5: Auto-detect session from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    run_in_test_repo test -d "$session_dir"
    [ "$?" -eq 0 ]
    
    # 2. Navigate to worktree and edit file
    (cd "$TEST_REPO/$session_dir" && echo "Auto-detect test change" >> test-file.py)
    
    # 3. Test merge from within worktree (should auto-detect session)
    run bash -c "cd '$TEST_REPO/$session_dir' && '$PURSOR_SCRIPT' merge 'Auto-detect test commit'"
    [ "$status" -eq 0 ]
    
    # Verify commit exists on main
    run run_in_test_repo git log --oneline
    [[ "$output" == *"Auto-detect test commit"* ]]
    
    # Verify changes are in main
    run_in_test_repo grep "Auto-detect test change" test-file.py
    [ "$?" -eq 0 ]
    
    # Verify cleanup - session should be cleaned up after successful merge
    run_in_test_repo test -d "$session_dir"
    [ "$?" -ne 0 ]  # Should not exist
}

@test "IT-6: Auto-detect session for cancel from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_dir=$(run_in_test_repo find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    run_in_test_repo test -d "$session_dir"
    [ "$?" -eq 0 ]
    
    # Get the branch name before navigating
    branch_name=$(cd "$TEST_REPO/$session_dir" && git branch --show-current)
    
    # 2. Test cancel from within worktree (should auto-detect session)
    run bash -c "cd '$TEST_REPO/$session_dir' && '$PURSOR_SCRIPT' cancel"
    [ "$status" -eq 0 ]
    
    # Verify branch is deleted
    run run_in_test_repo git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    run_in_test_repo test -d "$session_dir"
    [ "$?" -ne 0 ]  # Should not exist
} 