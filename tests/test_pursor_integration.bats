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

@test "IT-1: Happy path - create session, edit file, rebase successfully" {
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
    
    # 3. Rebase with message
    run "$PURSOR_SCRIPT" rebase "Integration test commit"
    [ "$status" -eq 0 ]
    
    # Go back to test repo
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    assert_commit_exists "Integration test commit"
    
    # Verify changes are in main
    assert_file_contains "test-file.py" "Modified in session"
    
    # Verify cleanup - session directory should not exist after successful rebase
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
    
    # Create a justfile so it exists in main branch for conflict testing
    echo "# Original justfile content" > justfile
    git add justfile
    git commit -m "Add justfile for conflict test"
    
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
    
    # 3. Rebase A (should succeed)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" rebase "Session A changes"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 4. Try to rebase B (should have conflict)
    # Session B directory should still exist even after A is cleaned up
    assert_session_exists "$session_b"
    
    cd "$TEST_REPO/$session_b"
    run "$PURSOR_SCRIPT" rebase "Session B changes"
    # The rebase command should now fail due to rebase conflicts (this is the fix)
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
    
    # Continue the rebase
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
    
    # 3. Test rebase from within worktree (should auto-detect session)
    run "$PURSOR_SCRIPT" rebase "Auto-detect test commit"
    [ "$status" -eq 0 ]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify commit exists on main
    assert_commit_exists "Auto-detect test commit"
    
    # Verify changes are in main
    assert_file_contains "test-file.py" "Auto-detect test change"
    
    # Verify cleanup - session should be cleaned up after successful rebase
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

@test "IT-7: Rebase uncommitted changes should auto-stage and commit" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Make changes WITHOUT committing them
    cd "$TEST_REPO/$session_dir"
    
    # Modify existing file
    echo "Uncommitted modification" >> test-file.py
    
    # Add new file  
    echo "New file content" > new-file.txt
    
    # Verify these are uncommitted
    run git diff --quiet --exit-code
    [ "$status" -ne 0 ]  # Should have uncommitted changes
    
    run git status --porcelain
    [ "$status" -eq 0 ]
    [ -n "$output" ]  # Should have output indicating changes
    
    # 3. Try to rebase with uncommitted changes - this should work automatically
    run "$PURSOR_SCRIPT" rebase "Test commit with uncommitted changes"
    [ "$status" -eq 0 ]
    # Should NOT see error about uncommitted changes
    [[ "$output" == *"staging all changes"* ]]
    [[ "$output" == *"committing changes"* ]]
    
    # Go back to main
    cd "$TEST_REPO"
    
    # Verify commit exists
    assert_commit_exists "Test commit with uncommitted changes"
    
    # Verify changes are applied
    assert_file_contains "test-file.py" "Uncommitted modification"
    assert_file_contains "new-file.txt" "New file content"
    
    # Verify cleanup - session should be cleaned up after successful rebase  
    assert_session_not_exists "$session_dir"
}

@test "IT-8: Rebase conflict should not complete with unresolved markers" {
    # 1. Create session and edit the justfile (to cause conflicts)
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    # Create a justfile so it exists in main branch for conflict testing
    echo "# Original justfile content" > justfile
    git add justfile
    git commit -m "Add justfile for conflict test"
    session_a=$(find_session_dir)
    assert_session_exists "$session_a"
    
    # Store the original justfile content
    original_content=$(cat justfile)
    
    # Modify the justfile in the main branch first to set up a conflict
    echo "# Main branch change" >> justfile
    git add justfile
    git commit -m "Main branch: Modify justfile"
    
    # Now modify the same file differently in the session
    cd "$TEST_REPO/$session_a"
    cat > justfile << 'EOF'
# pursor development workflow automation
# Test file used for development

@bats test_a.bats
@bats test_b.bats
EOF
    
    # 4. Try to rebase session A (should fail due to conflict)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" rebase "Modify justfile in session A"
    [ "$status" -eq 1 ]
    [[ "$output" == *"rebase conflicts"* ]]
    
    # Verify the session still exists (not cleaned up due to conflict)
    cd "$TEST_REPO"
    assert_session_exists "$session_a"
    
    # 5. Verify conflict markers exist
    cd "$TEST_REPO/$session_a"
    run grep -q "<<<<<<< HEAD" justfile
    [ "$status" -eq 0 ]  # Should find conflict markers
    
    # 6. Try to continue WITHOUT resolving conflicts (should fail)
    run "$PURSOR_SCRIPT" continue
    [ "$status" -eq 1 ]
    [[ "$output" == *"unresolved conflicts"* ]]
    
    # Session should still exist
    cd "$TEST_REPO"
    assert_session_exists "$session_a"
}

@test "IT-9: Squash rebase mode - multiple commits become one (default behavior)" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Make multiple commits in the session
    cd "$TEST_REPO/$session_dir"
    
    # First commit
    echo "change 1" >> test-file.py
    git add test-file.py
    git commit -m "Session: First change"
    
    # Second commit
    echo "change 2" >> test-file.py
    git add test-file.py
    git commit -m "Session: Second change"
    
    # Third commit
    echo "change 3" >> test-file.py
    git add test-file.py
    git commit -m "Session: Third change"
    
    # Verify we have multiple commits beyond the base
    commit_count=$(git rev-list --count HEAD ^master)
    [ "$commit_count" -eq 3 ]
    
    # 3. Rebase using default squash mode
    run "$PURSOR_SCRIPT" rebase "Feature complete with squashed changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"mode: squash"* ]]
    [[ "$output" == *"squashed 3 commits"* ]]
    
    cd "$TEST_REPO"
    
    # 4. Verify only ONE new commit was added to main
    # Should have: original commit + 1 squashed commit = 2 total
    total_commits=$(git rev-list --count HEAD)
    [ "$total_commits" -eq 2 ]
    
    # The final commit should have the specified message
    assert_commit_exists "Feature complete with squashed changes"
    
    # Should NOT have the individual session commit messages
    run git log --oneline --grep="Session: First change"
    [ "$status" -eq 0 ]
    [ -z "$output" ]  # Should not find individual commit messages
    
    # But should have all the changes in the final file
    assert_file_contains "test-file.py" "change 1"
    assert_file_contains "test-file.py" "change 2"
    assert_file_contains "test-file.py" "change 3"
    
    # Verify cleanup
    assert_session_not_exists "$session_dir"
}

@test "IT-10: Rebase preserve mode - preserve individual commits" {
    # 1. Create session
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Create multiple commits
    cd "$TEST_REPO/$session_dir"
    
    # First commit
    echo "First feature change" >> test-file.py
    git add test-file.py
    git commit -m "Session: First feature"
    
    # Second commit
    echo "Second feature change" >> test-file.py  
    git add test-file.py
    git commit -m "Session: Second feature"
    
    # Make uncommitted changes that should be auto-committed
    echo "Uncommitted changes" >> test-file.py
    
    # Count commits before (should be 3: initial + 2 session commits, uncommitted changes will become 4th)
    cd "$TEST_REPO"
    initial_commit_count=$(git rev-list --count HEAD)
    [ "$initial_commit_count" -eq 1 ]
    
    # 3. Rebase using preserve mode
    cd "$TEST_REPO/$session_dir"
    run "$PURSOR_SCRIPT" rebase --preserve "Final uncommitted changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"mode: rebase"* ]]
    
    # Go back to main
    cd "$TEST_REPO"
    
    # Should have 4 commits total (initial + 3 from session)
    final_commit_count=$(git rev-list --count HEAD)
    [ "$final_commit_count" -eq 4 ]
    
    # Verify individual commit messages are preserved
    assert_commit_exists "Session: First feature"
    assert_commit_exists "Session: Second feature"
    assert_commit_exists "Final uncommitted changes"
    
    # Verify content is correct
    assert_file_contains "test-file.py" "First feature change"
    assert_file_contains "test-file.py" "Second feature change"
    assert_file_contains "test-file.py" "Uncommitted changes"
}

@test "IT-11: Rebase conflict resolution with preserve mode" {
    # 1. Create session A
    run run_pursor
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_a=$(find_session_dir)
    
    # 2. Create session B (after small delay)
    sleep 1
    run run_pursor
    [ "$status" -eq 0 ]
    
    session_b=""
    for dir in $(find subtrees/pc -maxdepth 1 -type d -name "20*"); do
        if [ "$dir" != "$session_a" ]; then
            session_b="$dir"
            break
        fi
    done
    
    # Session A: Make commits and edit conflict file
    cd "$TEST_REPO/$session_a"
    echo "Session A first" >> test-file.py
    git add test-file.py
    git commit -m "Session A: First commit"
    
    echo "Session A conflict line" > conflict-file.txt
    git add conflict-file.txt
    git commit -m "Session A: Conflicting commit"
    
    # Session B: Make commits with conflicting changes
    cd "$TEST_REPO/$session_b"
    echo "Session B first" >> test-file.py
    git add test-file.py
    git commit -m "Session B: First commit"
    
    echo "Session B conflict line" > conflict-file.txt
    git add conflict-file.txt 
    git commit -m "Session B: Conflicting commit"
    
    # 3. Rebase session A with preserve mode (should succeed)
    cd "$TEST_REPO/$session_a"
    run "$PURSOR_SCRIPT" rebase --preserve "Session A complete"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 4. Try to rebase session B with preserve mode (should conflict)
    cd "$TEST_REPO/$session_b"
    run "$PURSOR_SCRIPT" rebase --preserve "Session B final"
    [ "$status" -eq 1 ]
    [[ "$output" == *"rebase conflicts"* ]]
    
    # Session should still exist
    cd "$TEST_REPO"
    assert_session_exists "$session_b"
} 