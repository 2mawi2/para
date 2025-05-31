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
    cd "$TEST_REPO"
    [ -d "subtrees" ]
    
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_dir" ]
    [ -d "$session_dir" ]
    
    # Verify state tracking
    [ -d ".pursor_state" ]
    session_count=$(ls .pursor_state | wc -l)
    [ "$session_count" -eq 1 ]
    
    # 2. Edit file in worktree
    cd "$session_dir"
    echo "Modified in session" >> test-file.py
    
    # 3. Merge with message
    run "$PURSOR_SCRIPT" merge "Integration test commit"
    [ "$status" -eq 0 ]
    
    # Go back to test repo
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    run git log --oneline
    [[ "$output" == *"Integration test commit"* ]]
    
    # Verify changes are in main
    grep -q "Modified in session" test-file.py
    
    # Verify cleanup - session directory should not exist after successful merge
    [ ! -d "$session_dir" ]
}

@test "IT-2: Cancel session" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # Get branch name
    cd "$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Cancel from within worktree
    run "$PURSOR_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    [ ! -d "$session_dir" ]
}

@test "IT-3: Conflict resolution with continue" {
    # 1. Create session A and edit file
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Store session A path immediately
    session_a=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -n "$session_a" ]
    [ -d "$session_a" ]
    
    # Edit the existing file - replace the line with session A content
    cd "$session_a"
    echo "Change from session A" > test-file.py
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
    
    # Edit the same file in a conflicting way - replace with different content
    cd "$session_b"
    echo "Change from session B" > test-file.py
    
    # 3. Merge A (should succeed)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" merge "Session A changes"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 4. Try to merge B (should have conflict)
    # Session B directory should still exist even after A is cleaned up
    [ -d "$session_b" ]
    
    cd "$session_b"
    run "$PURSOR_SCRIPT" merge "Session B changes"
    # The merge command should now fail due to rebase conflicts (this is the fix)
    [ "$status" -eq 1 ]
    # Should output conflict information
    [[ "$output" == *"rebase conflicts"* ]]
    
    cd "$TEST_REPO"
    
    # The session should still be active (not cleaned up due to conflict)
    [ -d "$session_b" ]
    
    # 5. Manually resolve conflict
    # Remove conflict markers and combine both changes
    cat > "$session_b/test-file.py" << 'EOF'
Change from session A
Change from session B
EOF
    
    # Continue the merge
    cd "$session_b"
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify both changes are in history
    run git log --oneline
    [[ "$output" == *"Session A changes"* ]]
    [[ "$output" == *"Session B changes"* ]]
    
    # Verify final content has both changes
    grep -q "Change from session A" test-file.py
    grep -q "Change from session B" test-file.py
}

@test "IT-4: Clean all sessions" {
    # 1. Create two sessions
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
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
    
    # Verify original repo is untouched
    [ -f "test-file.py" ]
    run git log --oneline
    [[ "$output" == *"Initial commit"* ]]
}

@test "IT-5: Auto-detect session from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # 2. Navigate to worktree and edit file
    cd "$session_dir"
    echo "Auto-detect test change" >> test-file.py
    
    # 3. Test merge from within worktree (should auto-detect session)
    run "$PURSOR_SCRIPT" merge "Auto-detect test commit"
    [ "$status" -eq 0 ]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    run git log --oneline
    [[ "$output" == *"Auto-detect test commit"* ]]
    
    # Verify changes are in main
    grep -q "Auto-detect test change" test-file.py
    
    # Verify cleanup - session should be cleaned up after successful merge
    [ ! -d "$session_dir" ]
}

@test "IT-6: Auto-detect session for cancel from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # Get the branch name before navigating
    cd "$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Test cancel from within worktree (should auto-detect session)
    run "$PURSOR_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    [ ! -d "$session_dir" ]
}

@test "IT-7: Merge uncommitted changes should auto-stage and commit" {
    # This reproduces the user's reported issue
    
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    [ -d "$session_dir" ]
    
    # 2. Navigate to worktree and make changes WITHOUT committing them
    cd "$session_dir"
    echo "Uncommitted change line 1" >> test-file.py
    echo "Uncommitted change line 2" >> test-file.py
    
    # Create a new file as well (untracked)
    echo "New file content" > new-file.txt
    
    # Verify we have uncommitted changes
    run git status --porcelain
    [ "$status" -eq 0 ]
    [ -n "$output" ]  # Should have output indicating changes
    
    # 3. Try to merge with uncommitted changes - this should work automatically
    run "$PURSOR_SCRIPT" merge "Test commit with uncommitted changes"
    [ "$status" -eq 0 ]
    # Should NOT see error about uncommitted changes
    [[ "$output" != *"You have unstaged changes"* ]]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    run git log --oneline
    [[ "$output" == *"Test commit with uncommitted changes"* ]]
    
    # Verify all changes are in main (both modified and new files)
    grep -q "Uncommitted change line 1" test-file.py
    grep -q "Uncommitted change line 2" test-file.py
    [ -f "new-file.txt" ]
    grep -q "New file content" new-file.txt
    
    # Verify cleanup - session should be cleaned up after successful merge  
    [ ! -d "$session_dir" ]
}

@test "IT-8: Merge conflict should not complete with unresolved markers" {
    # This tests the justfile conflict issue the user reported
    
    # 1. Create a file that will conflict in main repo
    cd "$TEST_REPO"
    cat > justfile << 'EOF'
# Original justfile content
test:
    @echo "Running original tests..."
    @bats test.bats
EOF
    git add justfile
    git commit -m "Add original justfile"
    
    # 2. Create session A and modify justfile
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_a=$(find subtrees/pc -maxdepth 1 -type d -name "20*" | head -1)
    cd "$session_a"
    cat > justfile << 'EOF'
# Modified justfile content A
test:
    @echo "Running modified tests A..."
    @bats test_a.bats
EOF
    
    # 3. Go back and modify main branch
    cd "$TEST_REPO"
    cat > justfile << 'EOF'
# Modified justfile content B
test:
    @echo "Running modified tests B..."
    @bats test_b.bats
EOF
    git add justfile
    git commit -m "Modify justfile in main"
    
    # 4. Try to merge session A (should create conflict)
    cd "$session_a"
    run "$PURSOR_SCRIPT" merge "Modify justfile in session A"
    # This should fail due to rebase conflict
    [ "$status" -eq 1 ]
    [[ "$output" == *"rebase conflicts"* ]]
    
    # 5. Try to continue without properly resolving conflicts
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 1 ]
    # Should detect unresolved conflicts
    [[ "$output" == *"still unresolved conflicts"* ]] || [[ "$output" == *"conflict markers"* ]]
    
    # 6. Properly resolve conflicts
    cat > justfile << 'EOF'
# Resolved justfile content
test:
    @echo "Running resolved tests..."
    @bats test_resolved.bats
EOF
    
    # 7. Now continue should work
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify the conflict was properly resolved
    grep -q "Running resolved tests" justfile
    # Should NOT contain conflict markers
    ! grep -q "<<<<<<< HEAD" justfile
    ! grep -q "=======" justfile
    ! grep -q ">>>>>>> " justfile
} 