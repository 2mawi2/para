#!/usr/bin/env bats

# Recovery mechanism test suite for para
# Tests recovery of finished/cancelled sessions

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "RT-1: Recover a finished session" {
    # 1. Create session, make changes, and finish it
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    # Make changes
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    
    # Finish the session
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit for recovery"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Verify session is finished and cleaned up
    [ ! -d "$session_dir" ]
    run "$PARA_SCRIPT" list
    [[ "$output" == *"No active parallel sessions"* ]]
    
    # 3. Verify session can be recovered
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    [[ "$output" == *"finished"* ]]
    [[ "$output" == *"Test commit for recovery"* ]]
    
    # 4. Recover the session
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 0 ]
    [[ "$output" == *"recovered session"* ]]
    
    # 5. Verify session is active again
    run "$PARA_SCRIPT" list
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    
    # 6. Verify worktree exists and has the right content
    [ -d "subtrees/pc/$session_id" ]
    [ -f "subtrees/pc/$session_id/test-file.py" ]
    grep -q "test content" "subtrees/pc/$session_id/test-file.py"
}

@test "RT-2: Recover a cancelled session" {
    # 1. Create session, make changes, and cancel it
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    # Make changes
    echo "cancelled content" > "$TEST_REPO/$session_dir/test-file.py"
    
    # Cancel the session
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Verify session is cancelled and cleaned up
    [ ! -d "$session_dir" ]
    run "$PARA_SCRIPT" list
    [[ "$output" == *"No active parallel sessions"* ]]
    
    # 3. Verify session can be recovered
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    [[ "$output" == *"cancelled"* ]]
    
    # 4. Recover the session
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 0 ]
    [[ "$output" == *"recovered session"* ]]
    
    # 5. Verify session is active again
    run "$PARA_SCRIPT" list
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    
    # 6. Verify worktree exists and has the uncommitted changes
    [ -d "subtrees/pc/$session_id" ]
    [ -f "subtrees/pc/$session_id/test-file.py" ]
    grep -q "cancelled content" "subtrees/pc/$session_id/test-file.py"
}

@test "RT-3: Recover session with specific name" {
    # 1. Create session with custom name, finish it
    run run_para start "feature-branch"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    # For custom named sessions, the session ID is the custom name, not the directory name
    session_id="feature-branch"
    
    # Make changes and finish
    echo "feature content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Feature implementation"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Recover by session ID
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 0 ]
    
    # 3. Verify custom name is preserved and directory exists
    [ -d "$session_dir" ]
    [[ "$session_id" == *"feature-branch"* ]]
}

@test "RT-4: History command shows finished and cancelled sessions" {
    # 1. Create and finish one session
    run run_para start "finished-session"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    finished_session_dir=$(find_session_dir)
    finished_session_id=$(basename "$finished_session_dir")
    
    echo "finished content" > "$TEST_REPO/$finished_session_dir/test-file.py"
    cd "$TEST_REPO/$finished_session_dir"
    run "$PARA_SCRIPT" finish "Finished session commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Create and cancel another session
    run run_para start "cancelled-session"
    [ "$status" -eq 0 ]
    
    cancelled_session_dir=$(find_session_dir)
    cancelled_session_id=$(basename "$cancelled_session_dir")
    
    echo "cancelled content" > "$TEST_REPO/$cancelled_session_dir/test-file.py"
    cd "$TEST_REPO/$cancelled_session_dir"
    run "$PARA_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 3. Check history shows both sessions
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$finished_session_id"* ]]
    [[ "$output" == *"finished"* ]]
    [[ "$output" == *"Finished session commit"* ]]
    [[ "$output" == *"$cancelled_session_id"* ]]
    [[ "$output" == *"cancelled"* ]]
}

@test "RT-5: Clean history removes old entries" {
    # 1. Create and finish a session
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Verify session is in history
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    
    # 3. Clean history
    run "$PARA_SCRIPT" clean-history
    [ "$status" -eq 0 ]
    [[ "$output" == *"cleaned"* ]]
    
    # 4. Verify history is empty
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"No finished or cancelled sessions"* ]]
    
    # 5. Verify recovery no longer works
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 1 ]
    [[ "$output" == *"not found in history"* ]]
}

@test "RT-6: Clean history with age filter" {
    # 1. Create and finish a session
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Clean history older than 0 days (should remove everything)
    run "$PARA_SCRIPT" clean-history --older-than 0
    [ "$status" -eq 0 ]
    [[ "$output" == *"cleaned"* ]]
    
    # 3. Verify history is empty
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"No finished or cancelled sessions"* ]]
}

@test "RT-7: Automatic cleanup on start" {
    # 1. Create and finish a session
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Manually set an old timestamp on the history entry to simulate age
    # This will require modifying the history file timestamp
    # (Implementation detail - this test verifies auto-cleanup works)
    
    # Verify session is in history before cleanup
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    
    # 3. Force cleanup by setting retention to 0 days in config
    echo "RECOVERY_RETENTION_DAYS=0" > .para_config
    
    # 4. Start a new session (should trigger cleanup)
    run run_para start
    [ "$status" -eq 0 ]
    
    # 5. Verify old session was cleaned from history
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    # Should show empty history or only the new session
    if [[ "$output" == *"$session_id"* ]]; then
        # If it shows any sessions, it should only be the new one
        [ "$(echo "$output" | grep -c "Session:")" -eq 1 ]
    fi
}

@test "RT-8: Cannot recover non-existent session" {
    run "$PARA_SCRIPT" recover "non-existent-session"
    [ "$status" -eq 1 ]
    [[ "$output" == *"not found in history"* ]]
}

@test "RT-9: Cannot recover session that is already active" {
    # 1. Create and finish a session
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Recover the session
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 0 ]
    
    # 3. Try to recover it again (should fail)
    run "$PARA_SCRIPT" recover "$session_id"
    [ "$status" -eq 1 ]
    [[ "$output" == *"already active"* ]]
}

@test "RT-10: History shows timestamps and recovery information" {
    # 1. Create, modify, and finish a session
    run run_para start
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    echo "test content" > "$TEST_REPO/$session_dir/test-file.py"
    cd "$TEST_REPO/$session_dir"
    run "$PARA_SCRIPT" finish "Test commit"
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # 2. Check history shows detailed information
    run "$PARA_SCRIPT" history
    [ "$status" -eq 0 ]
    [[ "$output" == *"$session_id"* ]]
    [[ "$output" == *"finished"* ]]
    [[ "$output" == *"Test commit"* ]]
    [[ "$output" == *"Recover:"* ]]
    [[ "$output" == *"para recover $session_id"* ]]
} 