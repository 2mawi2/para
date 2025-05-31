#!/usr/bin/env bats

# Integration test suite for pursor
# Tests complete workflows in isolated temporary directories

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "IT-1: Happy path - create session, edit file, merge successfully" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify session was created (all checks in test directory)
    cd "$TEST_REPO"
    [ -d "subtrees" ]
    
    session_dir=$(find_session_dir)
    [ -n "$session_dir" ]
    assert_session_exists "$session_dir"
    
    # Verify state tracking
    [ -d ".pursor_state" ]
    session_count=$(count_sessions)
    [ "$session_count" -eq 1 ]
    
    # 2. Edit file in worktree
    cd "$TEST_REPO/$session_dir"
    echo "Modified in session" >> test-file.py
    
    # 3. Merge with message
    run "$PURSOR_SCRIPT" merge "Integration test commit"
    [ "$status" -eq 0 ]
    
    # Go back to test repo
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    assert_commit_exists "Integration test commit"
    
    # Verify changes are in main
    assert_file_contains "test-file.py" "Modified in session"
    
    # Verify cleanup - session directory should not exist after successful merge
    assert_session_not_exists "$session_dir"
}

@test "IT-2: Cancel session" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Get branch name
    cd "$TEST_REPO/$session_dir"
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
    assert_session_not_exists "$session_dir"
}

@test "IT-3: Conflict resolution with continue" {
    # 1. Create session A and edit file
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Store session A path immediately
    session_a=$(find_session_dir)
    [ -n "$session_a" ]
    assert_session_exists "$session_a"
    
    # Edit the existing file - replace the line with session A content
    cd "$TEST_REPO/$session_a"
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
    assert_session_exists "$session_b"
    
    # Edit the same file in a conflicting way - replace with different content
    cd "$TEST_REPO/$session_b"
    echo "Change from session B" > test-file.py
    
    # 3. Merge A (should succeed)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" merge "Session A changes"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 4. Try to merge B (should have conflict)
    # Session B directory should still exist even after A is cleaned up
    assert_session_exists "$session_b"
    
    cd "$TEST_REPO/$session_b"
    run "$PURSOR_SCRIPT" merge "Session B changes"
    # The merge command should now fail due to rebase conflicts (this is the fix)
    [ "$status" -eq 1 ]
    # Should output conflict information
    [[ "$output" == *"rebase conflicts"* ]]
    
    cd "$TEST_REPO"
    
    # The session should still be active (not cleaned up due to conflict)
    assert_session_exists "$session_b"
    
    # 5. Manually resolve conflict
    # Remove conflict markers and combine both changes
    cat > "$TEST_REPO/$session_b/test-file.py" << 'EOF'
Change from session A
Change from session B
EOF
    
    # Continue the merge
    cd "$TEST_REPO/$session_b"
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify both changes are in history
    assert_commit_exists "Session A changes"
    assert_commit_exists "Session B changes"
    
    # Verify final content has both changes
    assert_file_contains "test-file.py" "Change from session A"
    assert_file_contains "test-file.py" "Change from session B"
}

@test "IT-4: Clean all sessions" {
    # 1. Create two sessions
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify first session was created
    [ -d "subtrees" ]
    [ -d ".pursor_state" ]
    first_session_count=$(count_sessions)
    [ "$first_session_count" -eq 1 ]
    
    # Small delay to ensure different timestamps
    sleep 1
    
    run run_pursor
    [ "$status" -eq 0 ]
    
    # Verify two sessions exist
    session_count=$(count_sessions)
    [ "$session_count" -eq 2 ]
    
    # Verify state files exist
    [ -d ".pursor_state" ]
    state_count=$(count_state_files)
    [ "$state_count" -eq 2 ]
    
    # 2. Clean all sessions
    run run_pursor clean
    [ "$status" -eq 0 ]
    
    # Verify all worktrees removed
    session_count=$(count_sessions)
    [ "$session_count" -eq 0 ]
    
    # Verify original repo is untouched
    [ -f "test-file.py" ]
    assert_commit_exists "Initial commit"
}

@test "IT-5: Auto-detect session from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Navigate to worktree and edit file
    cd "$TEST_REPO/$session_dir"
    echo "Auto-detect test change" >> test-file.py
    
    # 3. Test merge from within worktree (should auto-detect session)
    run "$PURSOR_SCRIPT" merge "Auto-detect test commit"
    [ "$status" -eq 0 ]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    assert_commit_exists "Auto-detect test commit"
    
    # Verify changes are in main
    assert_file_contains "test-file.py" "Auto-detect test change"
    
    # Verify cleanup - session should be cleaned up after successful merge
    assert_session_not_exists "$session_dir"
}

@test "IT-6: Auto-detect session for cancel from worktree directory" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Get the branch name before navigating
    cd "$TEST_REPO/$session_dir"
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
    assert_session_not_exists "$session_dir"
}

@test "IT-7: Merge uncommitted changes should auto-stage and commit" {
    # This reproduces the user's reported issue
    
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Navigate to worktree and make changes WITHOUT committing them
    cd "$TEST_REPO/$session_dir"
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
    assert_commit_exists "Test commit with uncommitted changes"
    
    # Verify all changes are in main (both modified and new files)
    assert_file_contains "test-file.py" "Uncommitted change line 1"
    assert_file_contains "test-file.py" "Uncommitted change line 2"
    [ -f "new-file.txt" ]
    assert_file_contains "new-file.txt" "New file content"
    
    # Verify cleanup - session should be cleaned up after successful merge  
    assert_session_not_exists "$session_dir"
}

@test "IT-8: Merge conflict should not complete with unresolved markers" {
    # This tests the justfile conflict issue the user reported
    
    # Use the helper to set up a repo with files for conflict testing
    teardown_temp_git_repo
    setup_temp_git_repo_with_files
    
    # 2. Create session A and modify justfile
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_a=$(find_session_dir)
    cd "$TEST_REPO/$session_a"
    cat > justfile << 'EOF'
# Modified justfile content A
test:
    @echo "Running modified tests A..."
    @bats test_a.bats
EOF
    
    # 3. Go back and modify main branch
    create_main_conflict "justfile" "# Modified justfile content B
test:
    @echo \"Running modified tests B...\"
    @bats test_b.bats"
    
    # 4. Try to merge session A (should fail due to conflict)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" merge "Modify justfile in session A"
    [ "$status" -eq 1 ]
    [[ "$output" == *"rebase conflicts"* ]]
    
    # Session should still exist due to unresolved conflict
    cd "$TEST_REPO"
    assert_session_exists "$session_a"
    
    # 5. Check that conflict markers exist
    cd "$TEST_REPO/$session_a"
    run grep "<<<<<<< HEAD" justfile
    [ "$status" -eq 0 ]
    
    # 6. Try continue without resolving (should fail)
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 1 ]
    [[ "$output" == *"unresolved conflicts"* ]]
    
    # 7. Resolve conflict and continue
    cat > justfile << 'EOF'
# Resolved justfile content
test:
    @echo "Running resolved tests..."
    @bats test_resolved.bats
EOF
    
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify commit exists and content is resolved
    assert_commit_exists "Modify justfile in session A"
    assert_file_contains "justfile" "Resolved justfile content"
    
    # Session should be cleaned up after successful resolution
    assert_session_not_exists "$session_a"
} 