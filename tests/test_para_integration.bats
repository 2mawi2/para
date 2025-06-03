#!/usr/bin/env bats

# Integration test suite for para
# Tests complete workflows in isolated temporary directories

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "IT-1: Happy path - create session, edit file, finish successfully" {
    # 1. Create a new session
    run run_para
    [ "$status" -eq 0 ]
    [[ "$output" == *"initialized session"* ]]

    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"

    # 2. Edit a file in the worktree
    cd "$TEST_REPO/$session_dir"
    echo "test content" > new-feature.py

    # Get the branch name before finishing
    branch_name=$(git branch --show-current)

    # 3. Finish with message
    run "$PARA_SCRIPT" finish "Integration test commit"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: $branch_name"* ]]

    # 4. Verify the worktree is cleaned up
    cd "$TEST_REPO"
    assert_session_not_exists "$session_dir"

    # 5. Verify branch exists with the commit (not on main)  
    run git checkout "$branch_name"
    [ "$status" -eq 0 ]

    # Check that the commit exists with the right message
    run git log --oneline --grep="Integration test commit"
    [ "$status" -eq 0 ]
    [ -n "$output" ]

    # Verify the changes are in the branch
    [ -f "new-feature.py" ]
    grep -q "test content" new-feature.py

    # 6. Verify main branch is unchanged
    run git checkout master 2>/dev/null || git checkout main 2>/dev/null
    [ "$status" -eq 0 ]
    [ ! -f "new-feature.py" ]  # File should not exist on main

    # Verify cleanup - session should be cleaned up after successful finish
    assert_session_not_exists "$session_dir"
}

@test "IT-2: Cancel session" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Get branch name
    cd "$TEST_REPO/$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Cancel from within worktree
    run "$PARA_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    assert_session_not_exists "$session_dir"
}

@test "IT-3: Conflict resolution with finish creates branches for manual merge" {
    # 1. Create session A and edit file
    run run_para
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
    session_a_branch=$(git branch --show-current)
    cd "$TEST_REPO"
    
    # Small delay to ensure different timestamps
    sleep 1
    
    # 2. Create session B and edit same line
    run run_para
    [ "$status" -eq 0 ]
    
    # Store session B path immediately - find the one that's NOT session A
    session_b=""
    for dir in $(find subtrees/pc -maxdepth 1 -type d \( -name "*_*_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "*-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" \)); do
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
    session_b_branch=$(git branch --show-current)
    
    # 3. Finish A (should succeed and create branch)
    cd "$TEST_REPO/$session_a"
    run "$PARA_SCRIPT" finish "Session A changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # 4. Finish B (should also succeed and create branch)
    cd "$TEST_REPO/$session_b"
    run "$PARA_SCRIPT" finish "Session B changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # 5. Verify both branches exist and have the expected changes
    # Check branch A
    run git branch --list "$session_a_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_a_branch"
    assert_file_contains "test-file.py" "Change from session A"
    
    # Check branch B  
    run git branch --list "$session_b_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_b_branch"
    assert_file_contains "test-file.py" "Change from session B"
    
    # 6. Verify sessions are cleaned up but branches remain
    assert_session_not_exists "$session_a"
    assert_session_not_exists "$session_b"
}

@test "IT-4: Clean all sessions" {
    # 1. Create two sessions
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify first session was created
    [ -d "subtrees" ]
    [ -d ".para_state" ]
    first_session_count=$(count_sessions)
    [ "$first_session_count" -eq 1 ]
    
    # Small delay to ensure different timestamps
    sleep 1
    
    run run_para
    [ "$status" -eq 0 ]
    
    # Verify two sessions exist
    session_count=$(count_sessions)
    [ "$session_count" -eq 2 ]
    
    # Verify state files exist
    [ -d ".para_state" ]
    state_count=$(count_state_files)
    [ "$state_count" -eq 2 ]
    
    # 2. Clean all sessions
    run run_para clean
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
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Navigate to worktree and edit file
    cd "$TEST_REPO/$session_dir"
    echo "Auto-detect test change" >> test-file.py
    
    # Get the branch name before finishing
    branch_name=$(git branch --show-current)
    
    # 3. Test finish from within worktree (should auto-detect session)
    run "$PARA_SCRIPT" finish "Auto-detect test commit"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify branch exists
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify changes are in the branch
    git checkout "$branch_name"
    assert_file_contains "test-file.py" "Auto-detect test change"
    
    # Verify cleanup - session should be cleaned up after successful finish
    assert_session_not_exists "$session_dir"
}

@test "IT-6: Auto-detect session for cancel from worktree directory" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Get the branch name before navigating
    cd "$TEST_REPO/$session_dir"
    branch_name=$(git branch --show-current)
    
    # 2. Test cancel from within worktree (should auto-detect session)
    run "$PARA_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Verify branch is deleted
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -z "$output" ]
    
    # Verify worktree directory is gone
    assert_session_not_exists "$session_dir"
}

@test "IT-7: Finish uncommitted changes should auto-stage and commit" {
    # 1. Create session
    run run_para
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
    
    # Get the branch name before finishing
    branch_name=$(git branch --show-current)
    
    # 3. Try to finish with uncommitted changes - this should work automatically
    run "$PARA_SCRIPT" finish "Test commit with uncommitted changes"
    [ "$status" -eq 0 ]
    # Should NOT see error about uncommitted changes
    [[ "$output" == *"staging all changes"* ]]
    [[ "$output" == *"committing changes"* ]]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    # Go back to main
    cd "$TEST_REPO"
    
    # Verify branch exists
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify changes are applied to the branch
    git checkout "$branch_name"
    assert_file_contains "test-file.py" "Uncommitted modification"
    assert_file_contains "new-file.txt" "New file content"
    
    # Verify cleanup - session should be cleaned up after successful finish  
    assert_session_not_exists "$session_dir"
}

@test "IT-8: Finish with conflicting changes creates separate branches" {
    # 1. Create session and edit the justfile (to test branch creation)
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    # Create a justfile so it exists in main branch for conflict testing
    echo "# Original justfile content" > justfile
    git add justfile
    git commit -m "Add justfile for conflict test"
    session_a=$(find_session_dir)
    assert_session_exists "$session_a"
    
    # Modify the justfile in the main branch first to set up potential conflicts
    echo "# Main branch change" >> justfile
    git add justfile
    git commit -m "Main branch: Modify justfile"
    
    # Now modify the same file differently in the session
    cd "$TEST_REPO/$session_a"
    cat > justfile << 'EOF'
# para development workflow automation
# Test file used for development

@bats test_a.bats
@bats test_b.bats
EOF
    
    # 4. Finish session A (should succeed and create branch)
    session_branch=$(git branch --show-current)
    run "$PARA_SCRIPT" finish "Modify justfile in session A"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    # 5. Verify the session was cleaned up
    cd "$TEST_REPO"
    assert_session_not_exists "$session_a"
    
    # 6. Verify the branch exists and has the expected content
    run git branch --list "$session_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_branch"
    assert_file_contains "justfile" "para development workflow automation"
    assert_file_contains "justfile" "@bats test_a.bats"
}

@test "IT-9: Squash finish mode - multiple commits become one (default behavior)" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Get the base branch name (could be main or master)
    base_branch=$(git symbolic-ref --short HEAD)
    
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
    commit_count=$(git rev-list --count HEAD ^"$base_branch")
    [ "$commit_count" -eq 3 ]
    
    # Get branch name for verification
    branch_name=$(git branch --show-current)
    
    # 3. Finish using default squash mode
    run "$PARA_SCRIPT" finish "Feature complete with squashed changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"mode: squash"* ]]
    [[ "$output" == *"squashed 3 commits"* ]]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # 4. Verify branch exists with squashed commit
    run git checkout "$branch_name"
    [ "$status" -eq 0 ]
    
    # Should have only ONE commit on the branch (squashed)
    branch_commit_count=$(git rev-list --count HEAD ^"$base_branch")
    [ "$branch_commit_count" -eq 1 ]
    
    # The final commit should have the specified message
    run git log --oneline -1 --grep="Feature complete with squashed changes"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Should NOT have the individual session commit messages on the branch
    run git log --oneline --grep="Session: First change"
    [ "$status" -eq 0 ]
    [ -z "$output" ]  # Should not find individual commit messages
    
    # But should have all the changes in the final file
    assert_file_contains "test-file.py" "change 1"
    assert_file_contains "test-file.py" "change 2"
    assert_file_contains "test-file.py" "change 3"
    
    # 5. Verify main branch is unchanged (still has only 1 commit)
    run git checkout "$base_branch"
    [ "$status" -eq 0 ]
    main_commit_count=$(git rev-list --count HEAD)
    [ "$main_commit_count" -eq 1 ]  # Should still be just the initial commit
    
    # Verify cleanup
    assert_session_not_exists "$session_dir"
}

@test "IT-10: Finish preserve mode - preserve individual commits" {
    # 1. Create session
    run run_para
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
    
    # Get branch name and base branch for commit counting
    branch_name=$(git branch --show-current)
    cd "$TEST_REPO"
    initial_commit_count=$(git rev-list --count HEAD)
    [ "$initial_commit_count" -eq 1 ]
    
    # 3. Finish using preserve mode
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish --preserve "Final uncommitted changes"
    [ "$status" -eq 0 ]
    [[ "$output" == *"mode: rebase"* ]]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    # Go back to main
    cd "$TEST_REPO"
    
    # Verify branch exists
    run git branch --list "$branch_name"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify content is correct on the branch
    git checkout "$branch_name"
    assert_file_contains "test-file.py" "First feature change"
    assert_file_contains "test-file.py" "Second feature change"
    assert_file_contains "test-file.py" "Uncommitted changes"
}

@test "IT-11: Preserve mode creates branches with individual commits" {
    # 1. Create session A
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_a=$(find_session_dir)
    
    # Small delay to ensure session A cleanup is complete
    sleep 1
    
    # 4. Create session B 
    run run_para
    [ "$status" -eq 0 ]
    
    session_b=""
    for dir in $(find subtrees/pc -maxdepth 1 -type d \( -name "*_*_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" -o -name "*-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" \)); do
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
    
    # 3. Finish session A with preserve mode (should succeed and create branch)
    cd "$TEST_REPO/$session_a"
    session_a_branch=$(git branch --show-current)
    run "$PARA_SCRIPT" finish --preserve "Session A complete"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # 4. Finish session B with preserve mode (should also succeed and create branch)
    cd "$TEST_REPO/$session_b"
    session_b_branch=$(git branch --show-current)
    run "$PARA_SCRIPT" finish --preserve "Session B final"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    # 5. Verify both sessions are cleaned up
    cd "$TEST_REPO"
    assert_session_not_exists "$session_a"
    assert_session_not_exists "$session_b"
    
    # 6. Verify both branches exist and have expected content
    # Check session A branch
    run git branch --list "$session_a_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_a_branch"
    assert_file_contains "conflict-file.txt" "Session A conflict line"
    
    # Check session B branch
    run git branch --list "$session_b_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_b_branch"
    assert_file_contains "conflict-file.txt" "Session B conflict line"
}

@test "IT-12: Multiple sessions create separate branches successfully" {
    # 1. Create session A
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_a=$(find_session_dir)
    assert_session_exists "$session_a"
    
    # 2. Create many files in session A
    cd "$TEST_REPO/$session_a"
    for i in $(seq 1 50); do
        echo "Session A content $i" > "file_$i.txt"
    done
    
    # 3. Get branch name and finish session A
    session_a_branch=$(git branch --show-current)
    run "$PARA_SCRIPT" finish "Session A with many files"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # Small delay to ensure session A cleanup is complete
    sleep 1
    
    # 4. Create session B 
    run run_para
    [ "$status" -eq 0 ]
    
    session_b=$(find_session_dir)
    assert_session_exists "$session_b"
    
    # 5. Create different files in session B
    cd "$TEST_REPO/$session_b"
    for i in $(seq 1 50); do
        echo "Session B content $i" > "other_file_$i.txt"
    done
    
    # 6. Finish session B
    session_b_branch=$(git branch --show-current)
    run "$PARA_SCRIPT" finish "Session B with different files"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    
    cd "$TEST_REPO"
    
    # 7. Verify both branches exist and have different content
    run git branch --list "$session_a_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_a_branch"
    [ -f "file_1.txt" ]
    [ ! -f "other_file_1.txt" ]
    
    run git branch --list "$session_b_branch"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    git checkout "$session_b_branch"
    [ -f "other_file_1.txt" ]
    [ ! -f "file_1.txt" ]
    
    # 8. Both sessions should be cleaned up
    assert_session_not_exists "$session_a"
    assert_session_not_exists "$session_b"
}

# Test configurable branch prefix in actual session creation
@test "IT-13: Configurable branch prefix creates correct branch names" {
    # Test custom prefix functionality
    TEST_DIR=$(mktemp -d)
    cd "$TEST_DIR"
    
    # Initialize git repo
    git init
    git config user.name "Test User"
    git config user.email "test@example.com"
    echo "test" > README.md
    git add README.md
    git commit -m "Initial commit"
    
    # Test with custom prefix
    export PARA_BRANCH_PREFIX="feature"
    
    # 1. Create session with custom named session
    run "$PARA_SCRIPT" start test-session
    [ "$status" -eq 0 ]
    [[ "$output" == *"initialized session"* ]]
    
    # Find the session directory and check branch name
    # With the new system, subtrees directory structure uses the branch prefix too
    WORKTREE_DIR=$(find subtrees/feature -name "test-session*" -type d | head -1)
    [ -d "$WORKTREE_DIR" ]
    cd "$WORKTREE_DIR"
    BRANCH_NAME=$(git branch --show-current)
    # Should use feature prefix instead of default para
    [[ "$BRANCH_NAME" == feature/test-session-* ]]
    
    # 2. Make a change and finish
    echo "test change" > test-file.py
    cd "$TEST_DIR"
    
    # Finish with default behavior (should use prefix)
    cd $WORKTREE_DIR
    run "$PARA_SCRIPT" finish "Test configurable prefix feature"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: $BRANCH_NAME"* ]]
    
    # 3. Verify branch exists and has correct name pattern
    cd "$TEST_DIR"
    git checkout "$BRANCH_NAME"
    assert_file_contains "test-file.py" "test change"
    [[ "$BRANCH_NAME" == feature/test-session-* ]]
    
    # Cleanup
    cd /
    rm -rf "$TEST_DIR"
    unset PARA_BRANCH_PREFIX
}

# New tests for --branch functionality
@test "IT-14: Custom branch name with --branch flag" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Make changes and finish with custom branch name
    cd "$TEST_REPO/$session_dir"
    echo "Custom branch test change" > test-file.py
    
    # Finish with custom branch name
    run "$PARA_SCRIPT" finish "Test custom branch" --branch feature-authentication
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: feature-authentication"* ]]
    
    # Go back to main repo to verify
    cd "$TEST_REPO"
    
    # Verify the custom branch exists
    run git branch --list "feature-authentication"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify changes are in the custom branch
    git checkout feature-authentication
    assert_file_contains "test-file.py" "Custom branch test change"
    
    # Verify session is cleaned up
    assert_session_not_exists "$session_dir"
}

@test "IT-15: --branch with --preserve flag combination" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Make multiple commits
    cd "$TEST_REPO/$session_dir"
    
    # First commit
    echo "First change" > test-file.py
    git add test-file.py
    git commit -m "First commit in session"
    
    # Second commit  
    echo "Second change" >> test-file.py
    git add test-file.py
    git commit -m "Second commit in session"
    
    # 3. Finish with preserve and custom branch name
    run "$PARA_SCRIPT" finish --preserve "Preserved commits on custom branch" --branch feature-preserved
    [ "$status" -eq 0 ]
    [[ "$output" == *"mode: rebase"* ]]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: feature-preserved"* ]]
    
    # Go back to verify
    cd "$TEST_REPO"
    
    # Verify the custom branch exists
    run git branch --list "feature-preserved"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify we have the individual commits (not squashed)
    # Get the actual default branch name (could be main or master) 
    cd "$TEST_REPO"
    default_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Verify changes are in the branch and both commits are preserved
    git checkout feature-preserved
    assert_file_contains "test-file.py" "First change"
    assert_file_contains "test-file.py" "Second change"
    
    commit_count=$(git rev-list --count HEAD ^"$default_branch")
    # Should have exactly 2 commits (preserved)
    [ "$commit_count" -eq 2 ]
}

@test "IT-16: Branch name conflict resolution" {
    # 1. Create a branch that will conflict  
    cd "$TEST_REPO"
    # Get the actual default branch name first
    default_branch=$(git rev-parse --abbrev-ref HEAD)
    
    git checkout -b existing-feature
    echo "existing content" > existing.txt
    git add existing.txt
    git commit -m "Existing feature content"
    git checkout "$default_branch"
    
    # 2. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 3. Make changes and try to finish with existing branch name
    cd "$TEST_REPO/$session_dir"
    echo "New session content" > new-session-file.py
    
    # Finish with branch name that already exists
    run "$PARA_SCRIPT" finish "Test conflict resolution" --branch existing-feature
    [ "$status" -eq 0 ]
    [[ "$output" == *"branch 'existing-feature' already exists, using 'existing-feature-1' instead"* ]]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: existing-feature-1"* ]]
    
    # Go back to verify
    cd "$TEST_REPO"
    
    # Verify both branches exist
    run git branch --list "existing-feature"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    run git branch --list "existing-feature-1"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
    
    # Verify the content is in the right branches
    git checkout existing-feature
    assert_file_contains "existing.txt" "existing content"
    [ ! -f "new-session-file.py" ]
    
    git checkout existing-feature-1
    assert_file_contains "new-session-file.py" "New session content"
    # existing.txt might or might not exist depending on branching point, just check our new file
}

@test "IT-17a: Invalid branch name validation - spaces" {
    # Test invalid characters (spaces)
    run run_para  
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    cd "$TEST_REPO/$session_dir"
    echo "test change" > test-file.py
    
    run "$PARA_SCRIPT" finish "Test invalid name" --branch "feature with spaces"
    [ "$status" -ne 0 ]
    [[ "$output" == *"invalid branch name"* ]]
    [[ "$output" == *"contains spaces"* ]]
    
    # Session should still exist after failed finish attempt
    cd "$TEST_REPO"
    assert_session_exists "$session_dir"
}

@test "IT-17b: Invalid branch name validation - leading dash" {
    # Test name starting with dash
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO" 
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    cd "$TEST_REPO/$session_dir"
    echo "test change" > test-file.py
    
    run "$PARA_SCRIPT" finish "Test invalid name" --branch "-feature"
    [ "$status" -ne 0 ]
    [[ "$output" == *"invalid branch name"* ]]
    [[ "$output" == *"cannot start with"* ]]
    
    # Session should still exist after failed finish attempt  
    cd "$TEST_REPO"
    assert_session_exists "$session_dir"
}

@test "IT-17c: Invalid branch name validation - empty name" {
    # Test empty name
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    cd "$TEST_REPO/$session_dir"
    echo "test change" > test-file.py
    
    run "$PARA_SCRIPT" finish "Test invalid name" --branch ""
    [ "$status" -ne 0 ]
    [[ "$output" == *"branch name cannot be empty"* ]]
    
    # Session should still exist after failed finish attempt
    cd "$TEST_REPO"
    assert_session_exists "$session_dir"
}

@test "IT-18: Different argument order combinations for --branch" {
    # 1. Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # 2. Test different argument orders
    cd "$TEST_REPO/$session_dir"
    echo "test change" > test-file.py
    
    # Test: --branch first, then message
    run "$PARA_SCRIPT" finish --branch feature-order-test "Test argument order"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Session finished successfully!"* ]]
    [[ "$output" == *"Your changes are ready on branch: feature-order-test"* ]]
    
    # Verify branch exists
    cd "$TEST_REPO"
    run git branch --list "feature-order-test"
    [ "$status" -eq 0 ]
    [ -n "$output" ]
} 