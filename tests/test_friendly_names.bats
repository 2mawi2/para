#!/usr/bin/env bats

# Integration tests for friendly name functionality
# Tests the complete workflow with friendly names

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
    export LIB_DIR="$(dirname "${BATS_TEST_FILENAME}")/../lib"
}

teardown() {
    teardown_temp_git_repo
}

@test "FN-1: Create session generates friendly name automatically" {
    # Create session without providing name
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    [ -n "$session_dir" ]
    
    # Extract session name from path
    session_name=$(basename "$session_dir")
    
    # Should match friendly name pattern: adjective_noun_YYYYMMDD-HHMMSS
    [[ "$session_name" =~ ^[a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]$ ]]
    
    # Verify session exists in state
    [ -d ".para_state" ]
    state_files=$(find .para_state -name "*.state" | wc -l)
    [ "$state_files" -eq 1 ]
    
    # State file should use friendly name as session ID
    state_file=$(find .para_state -name "*.state" | head -1)
    state_session_id=$(basename "$state_file" .state)
    [[ "$state_session_id" =~ ^[a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]$ ]]
}

@test "FN-2: List sessions shows friendly names clearly" {
    # Create a session with friendly name
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # List sessions and check output format
    run run_para list
    [ "$status" -eq 0 ]
    
    # Should contain "Session:" followed by friendly name
    [[ "$output" =~ Session:\ [a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9] ]]
    
    # Should not show "(legacy:" or "(custom)" for friendly names
    [[ ! "$output" =~ \(legacy: ]]
    [[ ! "$output" =~ \(custom\) ]]
    
    # Should show resume command with friendly name
    [[ "$output" =~ Resume:\ para\ resume\ [a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9] ]]
}

@test "FN-3: Resume session by friendly name works" {
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_name=$(basename "$session_dir")
    
    # Extract session ID from directory name
    session_id=$(echo "$session_name" | sed 's|pc/||')
    
    # Set IDE to test mode to avoid launching actual IDE
    export IDE_CMD="echo"
    
    # Resume session by friendly name
    run run_para resume "$session_id"
    [ "$status" -eq 0 ]
    [[ "$output" =~ resuming\ session\ $session_id ]]
}

@test "FN-4: Auto-detect session with friendly name from worktree" {
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    
    # Navigate to worktree
    cd "$TEST_REPO/$session_dir"
    
    # Make some changes
    echo "auto-detect test with friendly name" >> test-file.py
    
    # Rebase from within worktree (should auto-detect)
    run "$PARA_SCRIPT" rebase "Friendly name auto-detect test"
    [ "$status" -eq 0 ]
    
    # Go back to main repo
    cd "$TEST_REPO"
    
    # Verify commit exists
    assert_commit_exists "Friendly name auto-detect test"
    assert_file_contains "test-file.py" "auto-detect test with friendly name"
}

@test "FN-5: Cancel session by friendly name works" {
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session_dir=$(find_session_dir)
    session_name=$(basename "$session_dir")
    session_id=$(echo "$session_name" | sed 's|pc/||')
    
    # Cancel by friendly name
    run run_para cancel "$session_id"
    [ "$status" -eq 0 ]
    [[ "$output" =~ aborting\ session\ $session_id ]]
    
    # Verify session is gone
    assert_session_not_exists "$session_dir"
    
    # Verify state file is gone
    [ ! -f ".para_state/$session_id.state" ]
}

@test "FN-6: Multiple sessions get different friendly names" {
    # Create first session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    session1_dir=$(find_session_dir)
    session1_name=$(basename "$session1_dir")
    
    # Small delay to ensure different timestamps
    sleep 1
    
    # Create second session
    run run_para
    [ "$status" -eq 0 ]
    
    # Find both sessions
    session_count=$(count_sessions)
    [ "$session_count" -eq 2 ]
    
    # Get second session directory
    session2_dir=""
    for dir in $(find subtrees/pc -maxdepth 1 -type d \( -name "*_*_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]" \)); do
        if [ "$dir" != "$session1_dir" ]; then
            session2_dir="$dir"
            break
        fi
    done
    [ -n "$session2_dir" ]
    
    session2_name=$(basename "$session2_dir")
    
    # Sessions should have different names
    [ "$session1_name" != "$session2_name" ]
    
    # Both should be friendly format
    [[ "$session1_name" =~ ^[a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]$ ]]
    [[ "$session2_name" =~ ^[a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]$ ]]
}

@test "FN-7: Custom named session still works alongside friendly names" {
    # Create auto-named session (friendly name)
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    auto_session_dir=$(find_session_dir)
    
    # Create custom named session
    run run_para "my-feature"
    [ "$status" -eq 0 ]
    
    # Should have 2 sessions
    session_count=$(count_sessions)
    [ "$session_count" -eq 2 ]
    
    # List sessions and verify both types are shown correctly
    run run_para list
    [ "$status" -eq 0 ]
    
    # Should show friendly name without special annotation
    [[ "$output" =~ Session:\ [a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9] ]]
    
    # Should show custom name with (custom) annotation
    [[ "$output" =~ Session:\ my-feature\ \(custom\) ]]
}

@test "FN-8: Backward compatibility with legacy timestamp sessions" {
    # Create a session directory structure that mimics legacy format
    cd "$TEST_REPO"
    mkdir -p subtrees/pc
    
    # Simulate legacy session by creating state file directly
    mkdir -p .para_state
    legacy_timestamp="20240531-143022"
    legacy_session_id="pc-$legacy_timestamp"
    legacy_branch="pc/$legacy_timestamp"
    legacy_worktree="subtrees/$legacy_branch"
    
    # Create legacy worktree directory structure
    mkdir -p "$legacy_worktree"
    echo "$legacy_branch|$legacy_worktree|master|squash" > ".para_state/$legacy_session_id.state"
    
    # List sessions should handle legacy format
    run run_para list
    [ "$status" -eq 0 ]
    [[ "$output" =~ Session:\ $legacy_session_id\ \(legacy: ]]
}

@test "FN-9: Mixed environment - friendly and legacy sessions coexist" {
    # Create friendly name session
    run run_para
    [ "$status" -eq 0 ]
    
    cd "$TEST_REPO"
    
    # Simulate legacy session
    legacy_timestamp="20240531-143022"
    legacy_session_id="pc-$legacy_timestamp"
    legacy_branch="pc/$legacy_timestamp"
    legacy_worktree="subtrees/$legacy_branch"
    
    mkdir -p "$legacy_worktree"
    echo "$legacy_branch|$legacy_worktree|master|squash" > ".para_state/$legacy_session_id.state"
    
    # Should have 2 sessions total
    session_count=$(count_sessions)
    [ "$session_count" -eq 2 ]
    
    # List should show both with appropriate annotations
    run run_para list
    [ "$status" -eq 0 ]
    
    # Should show friendly name normally
    [[ "$output" =~ Session:\ [a-z]+_[a-z]+_[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9] ]]
    
    # Should show legacy with annotation
    [[ "$output" =~ Session:\ $legacy_session_id\ \(legacy: ]]
}

@test "FN-10: Friendly name generation is consistent across calls" {
    # Source para utilities to access friendly name functions
    . "$LIB_DIR/para-utils.sh"
    
    # Mock date to ensure consistent timestamps
    date() {
        case "$1" in
            "+%s")
                echo "1640995200"  # Fixed epoch time
                ;;
            "+%Y%m%d-%H%M%S")
                echo "20240531-184623"  # Fixed timestamp
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    # Multiple calls should generate same friendly name when timestamp is same
    friendly1=$(generate_friendly_name)
    friendly2=$(generate_friendly_name)
    [ "$friendly1" = "$friendly2" ]
    
    # Session ID should also be consistent
    session_id1=$(generate_session_id)
    session_id2=$(generate_session_id)
    [ "$session_id1" = "$session_id2" ]
    
    # Should match expected pattern
    [[ "$session_id1" =~ ^[a-z]+_[a-z]+_20240531-184623$ ]]
} 