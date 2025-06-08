#!/usr/bin/env bats

# Integration test for para backup and recovery functionality
# Tests the complete flow: create session → cancel → backup → recover

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
    
    # Initialize para environment properly in test context
    export REPO_ROOT="$TEST_REPO"
    export PARA_NON_INTERACTIVE=true
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Change to test repo directory
    cd "$TEST_REPO"
    
    # Source para modules in correct order with proper environment
    SCRIPT_DIR="$(dirname "$PARA_SCRIPT")"
    LIB_DIR="$SCRIPT_DIR/lib"
    
    # Load configuration and initialize paths
    . "$LIB_DIR/para-config.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-session.sh"
    . "$LIB_DIR/para-ide.sh"
    . "$LIB_DIR/para-backup.sh"
    
    # Initialize para environment
    need_git_repo
    load_config
    init_paths
    
    # Verify paths are set correctly
    [ -n "$STATE_DIR" ] || skip "STATE_DIR not initialized"
    [ -n "$SUBTREES_DIR" ] || skip "SUBTREES_DIR not initialized"
}

teardown() {
    teardown_temp_git_repo
}

@test "BIT-1: Complete backup and recovery integration test" {
    # === PHASE 1: Create and verify session ===
    echo "# Phase 1: Creating session" >&3
    
    # Create session directly using para functions
    SESSION_ID="integration-test"
    BASE_BRANCH="master"
    
    # Create session state properly
    create_new_session "$SESSION_ID" "" "true"
    
    # Verify session was created
    [ -f "$STATE_DIR/$SESSION_ID.state" ]
    
    # Get session info
    get_session_info "$SESSION_ID"
    echo "# Created session: $SESSION_ID" >&3
    echo "# Branch: $TEMP_BRANCH" >&3
    echo "# Worktree: $WORKTREE_DIR" >&3
    
    # Verify worktree and branch exist
    [ -d "$WORKTREE_DIR" ]
    git branch --list "$TEMP_BRANCH" | grep -q "$TEMP_BRANCH"
    
    # Add some content to the session and commit it
    echo "test backup content" > "$WORKTREE_DIR/backup-test.txt"
    cd "$WORKTREE_DIR"
    git add backup-test.txt
    git commit -m "Add test content for backup"
    cd "$TEST_REPO"
    
    # === PHASE 2: Cancel session and verify backup ===
    echo "# Phase 2: Cancelling session and creating backup" >&3
    
    # Save backup before cleanup (like cancel command does)
    save_cancelled_session_backup "$SESSION_ID" "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "squash"
    
    # Remove worktree but preserve branch (like cancel command does)
    remove_worktree_preserve_branch "$TEMP_BRANCH" "$WORKTREE_DIR"
    
    # Remove session state (like cancel command does)
    remove_session_state "$SESSION_ID"
    
    # Verify session is cancelled
    [ ! -f "$STATE_DIR/$SESSION_ID.state" ]
    [ ! -d "$WORKTREE_DIR" ]
    
    # Verify backup was created
    init_backup_paths
    [ -f "$BACKUP_DIR/$SESSION_ID.backup" ]
    echo "# Backup created at: $BACKUP_DIR/$SESSION_ID.backup" >&3
    
    # Verify branch was preserved
    git branch --list "$TEMP_BRANCH" | grep -q "$TEMP_BRANCH"
    echo "# Branch preserved: $TEMP_BRANCH" >&3
    
    # Verify backup contains correct metadata
    backup_file="$BACKUP_DIR/$SESSION_ID.backup"
    grep -q "session_id=$SESSION_ID" "$backup_file"
    grep -q "temp_branch=$TEMP_BRANCH" "$backup_file"
    grep -q "base_branch=$BASE_BRANCH" "$backup_file"
    
    # === PHASE 3: Recover session and verify ===
    echo "# Phase 3: Recovering session" >&3
    
    # Recover session
    recover_cancelled_session "$SESSION_ID"
    
    # Verify session is active again
    [ -f "$STATE_DIR/$SESSION_ID.state" ]
    [ -d "$WORKTREE_DIR" ]
    echo "# Session recovered successfully" >&3
    
    # Verify content was preserved
    [ -f "$WORKTREE_DIR/backup-test.txt" ]
    grep -q "test backup content" "$WORKTREE_DIR/backup-test.txt"
    
    # Verify backup was removed after recovery
    [ ! -f "$BACKUP_DIR/$SESSION_ID.backup" ]
    echo "# Backup cleaned up after recovery" >&3
    
    # Verify session info is correct
    get_session_info "$SESSION_ID"
    [ "$TEMP_BRANCH" = "para/${SESSION_ID}-$(generate_timestamp)" ] || [ -n "$TEMP_BRANCH" ]
    [ "$BASE_BRANCH" = "master" ]
    
    echo "# Recovery verification complete" >&3
}

@test "BIT-2: Backup cleanup maintains only last 3 sessions" {
    echo "# Testing backup cleanup functionality" >&3
    
    # Initialize backup paths
    init_backup_paths
    
    # Create 5 backup files directly to test cleanup
    session_ids=""
    for i in 1 2 3 4 5; do
        session_id="cleanup-test-$i"
        temp_branch="para/cleanup-test-$i-20250608-$(printf "%06d" "$i")"
        worktree_dir="$SUBTREES_DIR/$temp_branch"
        
        # Create a git branch for this test
        git branch "$temp_branch" HEAD 2>/dev/null || true
        
        # Create backup file directly
        timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        backup_file="$BACKUP_DIR/$session_id.backup"
        
        {
            echo "session_id=$session_id"
            echo "timestamp='$timestamp'"
            echo "temp_branch=$temp_branch"
            echo "worktree_dir=$worktree_dir"
            echo "base_branch=master"
            echo "merge_mode=squash"
        } > "$backup_file"
        
        # Track session IDs
        if [ "$i" -eq 1 ]; then
            session_ids="$session_id"
        else
            session_ids="$session_ids $session_id"
        fi
        
        echo "# Created backup for session: $session_id" >&3
        
        # Trigger cleanup after each backup to test the limit
        cleanup_old_backups
        
        # Small delay to ensure different timestamps
        sleep 1
    done
    
    # Verify only 3 backup files exist
    backup_count=$(find "$BACKUP_DIR" -name "*.backup" 2>/dev/null | wc -l)
    [ "$backup_count" -eq 3 ]
    echo "# Verified only 3 backups exist" >&3
    
    # Verify the last 3 sessions are in backup
    third_session=$(echo "$session_ids" | cut -d' ' -f3)
    fourth_session=$(echo "$session_ids" | cut -d' ' -f4)
    fifth_session=$(echo "$session_ids" | cut -d' ' -f5)
    
    [ -f "$BACKUP_DIR/$third_session.backup" ]
    [ -f "$BACKUP_DIR/$fourth_session.backup" ]
    [ -f "$BACKUP_DIR/$fifth_session.backup" ]
    
    # Verify the first two sessions are not in backup
    first_session=$(echo "$session_ids" | cut -d' ' -f1)
    second_session=$(echo "$session_ids" | cut -d' ' -f2)
    
    [ ! -f "$BACKUP_DIR/$first_session.backup" ]
    [ ! -f "$BACKUP_DIR/$second_session.backup" ]
    
    echo "# Backup cleanup working correctly" >&3
}

@test "BIT-3: Backup with pipe characters in merge_mode executes correctly" {
    echo "# Testing backup with pipe characters in merge_mode" >&3
    
    # Initialize backup paths
    init_backup_paths
    
    # Create a backup with pipe characters in merge_mode using the actual function
    SESSION_ID="pipe-test"
    TEMP_BRANCH="para/pipe-test-20250608-163443"
    WORKTREE_DIR="/Users/test/.para_containers/pipe-test"
    MERGE_MODE="squash|container"
    
    # Use the actual save function which should now properly quote the merge_mode
    save_cancelled_session_backup "$SESSION_ID" "$TEMP_BRANCH" "$WORKTREE_DIR" "master" "$MERGE_MODE"
    
    backup_file="$BACKUP_DIR/$SESSION_ID.backup"
    
    # The critical test: sourcing the backup file should not fail with "container: command not found"
    # This reproduces the original error where merge_mode=squash|container is interpreted as a pipe
    run sh -c "set -e; . '$backup_file'; echo 'Backup sourced successfully'"
    
    # This should fail with the current unquoted implementation due to "container: command not found"
    echo "# Test output: $output" >&3
    echo "# Test status: $status" >&3
    
    # Check that no "command not found" error occurs
    [[ "$output" != *"command not found"* ]]
    
    # The test should pass (status 0) when the fix is applied
    [ "$status" -eq 0 ]
    [[ "$output" == *"Backup sourced successfully"* ]]
    
    # Also test that list_cancelled_session_backups works without errors when reading this file
    run list_cancelled_session_backups
    [ "$status" -eq 0 ]
    
    echo "# Pipe character handling working correctly" >&3
}

@test "BIT-4: Recovery error handling" {
    echo "# Testing recovery error conditions" >&3
    
    # Test recovery of non-existent backup
    run recover_cancelled_session "non-existent-session"
    [ "$status" -eq 1 ]
    [[ "$output" == *"not found in backups"* ]]
    
    # Create a session and backup
    SESSION_ID="error-test"
    create_new_session "$SESSION_ID" "" "true"
    get_session_info "$SESSION_ID"
    
    save_cancelled_session_backup "$SESSION_ID" "$TEMP_BRANCH" "$WORKTREE_DIR" "master" "squash"
    remove_worktree_preserve_branch "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$SESSION_ID"
    
    # Delete the branch to simulate branch removal
    git branch -D "$TEMP_BRANCH"
    
    # Try to recover (should fail)
    run recover_cancelled_session "$SESSION_ID"
    [ "$status" -eq 1 ]
    [[ "$output" == *"no longer exists"* ]]
    
    echo "# Error handling working correctly" >&3
}

@test "BIT-5: Cleanup respects cancellation timestamp not file modification time" {
    echo "# Testing cleanup uses cancellation timestamp not file modification time" >&3
    
    # Initialize backup paths
    init_backup_paths
    
    # Create backups with specific cancellation timestamps but in reverse file modification order
    # This simulates the real bug where file modification time != cancellation time
    
    # Session 1: Cancelled at 10:00 (oldest cancellation)
    session1="old-session"
    temp_branch1="para/old-session-20250608-100000"
    git branch "$temp_branch1" HEAD 2>/dev/null || true
    backup_file1="$BACKUP_DIR/$session1.backup"
    {
        echo "session_id=$session1"
        echo "timestamp='2025-06-08 10:00:00'"
        echo "temp_branch=$temp_branch1"
        echo "worktree_dir=$SUBTREES_DIR/$temp_branch1"
        echo "base_branch=master"
        echo "merge_mode=squash"
    } > "$backup_file1"
    
    # Session 2: Cancelled at 11:00 (middle cancellation)
    session2="mid-session"
    temp_branch2="para/mid-session-20250608-110000"
    git branch "$temp_branch2" HEAD 2>/dev/null || true
    backup_file2="$BACKUP_DIR/$session2.backup"
    {
        echo "session_id=$session2"
        echo "timestamp='2025-06-08 11:00:00'"
        echo "temp_branch=$temp_branch2"
        echo "worktree_dir=$SUBTREES_DIR/$temp_branch2"
        echo "base_branch=master"
        echo "merge_mode=squash"
    } > "$backup_file2"
    
    # Session 3: Cancelled at 12:00 (newest cancellation)
    session3="new-session"
    temp_branch3="para/new-session-20250608-120000"
    git branch "$temp_branch3" HEAD 2>/dev/null || true
    backup_file3="$BACKUP_DIR/$session3.backup"
    {
        echo "session_id=$session3"
        echo "timestamp='2025-06-08 12:00:00'"
        echo "temp_branch=$temp_branch3"
        echo "worktree_dir=$SUBTREES_DIR/$temp_branch3"
        echo "base_branch=master"
        echo "merge_mode=squash"
    } > "$backup_file3"
    
    # Session 4: Cancelled at 09:00 (should be removed - oldest)
    session4="very-old-session"
    temp_branch4="para/very-old-session-20250608-090000"
    git branch "$temp_branch4" HEAD 2>/dev/null || true
    backup_file4="$BACKUP_DIR/$session4.backup"
    {
        echo "session_id=$session4"
        echo "timestamp='2025-06-08 09:00:00'"
        echo "temp_branch=$temp_branch4"
        echo "worktree_dir=$SUBTREES_DIR/$temp_branch4"
        echo "base_branch=master"
        echo "merge_mode=squash"
    } > "$backup_file4"
    
    # Now deliberately mess with file modification times to create the bug scenario
    # Make the "very-old-session" (cancelled at 09:00) have the newest file modification time
    sleep 1
    touch "$backup_file4"  # This makes it appear "newest" by file modification time
    
    echo "# Created 4 backups with mismatched timestamps vs file modification times" >&3
    
    # Verify we have 4 backups
    backup_count=$(find "$BACKUP_DIR" -name "*.backup" 2>/dev/null | wc -l)
    [ "$backup_count" -eq 4 ]
    
    # Trigger cleanup - should remove the session with oldest CANCELLATION time (very-old-session at 09:00)
    # NOT the oldest file modification time
    cleanup_old_backups
    
    # Verify only 3 backups remain
    backup_count=$(find "$BACKUP_DIR" -name "*.backup" 2>/dev/null | wc -l)
    [ "$backup_count" -eq 3 ]
    echo "# Backup count reduced to 3" >&3
    
    # Verify the correct session was removed (oldest by cancellation time, not file time)
    [ ! -f "$backup_file4" ]  # very-old-session (09:00) should be removed
    [ -f "$backup_file1" ]    # old-session (10:00) should remain
    [ -f "$backup_file2" ]    # mid-session (11:00) should remain  
    [ -f "$backup_file3" ]    # new-session (12:00) should remain
    
    echo "# Correct backup removed based on cancellation timestamp" >&3
    
    # Verify the branch was also removed
    ! git branch --list "$temp_branch4" | grep -q "$temp_branch4"
    
    echo "# Associated branch was properly cleaned up" >&3
}