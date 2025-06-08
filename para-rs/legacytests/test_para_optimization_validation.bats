#!/usr/bin/env bats

# Validation test for para performance optimizations
# Simple test to validate cancellation performance improvements

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

# Utility function to measure time (in milliseconds)
measure_time() {
    if command -v perl >/dev/null 2>&1; then
        perl -MTime::HiRes=time -E 'say int(time*1000)'
    else
        date +%s%3N 2>/dev/null || date +%s000
    fi
}

# Validate optimized cancellation performance
@test "OPT-1: Optimized cancellation performance validation" {
    cd "$TEST_REPO"
    
    # Test multiple cancellation cycles to get average performance
    total_time=0
    cycles=5
    
    echo "# Testing $cycles cancellation cycles..." >&3
    
    for i in $(seq 1 "$cycles"); do
        # Create session
        run run_para "opt-test-$i"
        [ "$status" -eq 0 ]
        
        session_dir=$(find_session_dir)
        assert_session_exists "$session_dir"
        
        # Add some content to make it realistic
        cd "$TEST_REPO/$session_dir"
        echo "Test content $i" > "test-file-$i.txt"
        git add .
        git commit -m "Test commit $i"
        
        # Measure cancellation time
        start_time=$(measure_time)
        run "$PARA_SCRIPT" cancel
        end_time=$(measure_time)
        
        [ "$status" -eq 0 ]
        
        duration=$((end_time - start_time))
        total_time=$((total_time + duration))
        
        echo "# Cycle $i cancellation: ${duration}ms" >&3
        
        # Verify session is gone
        cd "$TEST_REPO"
        assert_session_not_exists "$session_dir"
    done
    
    avg_time=$((total_time / cycles))
    echo "# Average cancellation time over $cycles cycles: ${avg_time}ms" >&3
    echo "# Total time for all cycles: ${total_time}ms" >&3
    
    # Performance should be good - allow for some variance
    [ "$avg_time" -lt 200 ]  # 200ms average should be achievable
}

# Validate git operations are efficient
@test "OPT-2: Git operations efficiency validation" {
    cd "$TEST_REPO"
    
    # Create session with substantial content
    run run_para "git-efficiency-test"
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    cd "$TEST_REPO/$session_dir"
    
    # Create moderate amount of content
    for i in $(seq 1 15); do
        echo "Content for file $i" > "file-$i.txt"
    done
    git add .
    git commit -m "Initial commit"
    
    # Add more commits
    for i in $(seq 1 5); do
        echo "Update $i" >> "file-1.txt"
        git add file-1.txt
        git commit -m "Update $i"
    done
    
    branch_name=$(git branch --show-current)
    worktree_path="$TEST_REPO/$session_dir"
    
    cd "$TEST_REPO"
    
    # Test individual git operations
    start_time=$(measure_time)
    git worktree remove --force "$worktree_path" 2>/dev/null || true
    mid_time=$(measure_time)
    git branch -D "$branch_name" 2>/dev/null || true
    end_time=$(measure_time)
    
    worktree_time=$((mid_time - start_time))
    branch_time=$((end_time - mid_time))
    total_git_time=$((end_time - start_time))
    
    echo "# Optimized git worktree remove: ${worktree_time}ms" >&3
    echo "# Optimized git branch delete: ${branch_time}ms" >&3
    echo "# Total optimized git operations: ${total_git_time}ms" >&3
    
    # Git operations should be fast
    [ "$total_git_time" -lt 100 ]  # 100ms total for git operations
    
    # Clean up state manually since we did git ops manually
    session_id=$(basename "$session_dir")
    rm -f ".para_state/$session_id.state" ".para_state/$session_id.prompt" 2>/dev/null || true
    rmdir ".para_state" 2>/dev/null || true
}

# Test filesystem operations efficiency 
@test "OPT-3: Filesystem operations efficiency validation" {
    cd "$TEST_REPO"
    
    # Create state files
    mkdir -p ".para_state"
    session_id="fs-efficiency-test"
    echo "test|state|data|squash" > ".para_state/$session_id.state"
    echo "test prompt" > ".para_state/$session_id.prompt"
    
    # Test optimized state removal
    start_time=$(measure_time)
    rm -f ".para_state/$session_id.state" ".para_state/$session_id.prompt" 2>/dev/null || true
    rmdir ".para_state" 2>/dev/null || true
    end_time=$(measure_time)
    
    fs_time=$((end_time - start_time))
    echo "# Optimized filesystem operations: ${fs_time}ms" >&3
    
    # Filesystem operations should be very fast
    [ "$fs_time" -lt 50 ]  # 50ms for filesystem operations
}

# End-to-end performance regression test
@test "OPT-4: End-to-end optimized performance" {
    cd "$TEST_REPO"
    
    # Test full create-modify-cancel cycle
    start_time=$(measure_time)
    
    # Create
    run run_para "e2e-test"
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    
    # Modify
    cd "$TEST_REPO/$session_dir"
    echo "End-to-end test content" > "e2e-file.txt"
    git add .
    git commit -m "E2E test commit"
    
    # Cancel
    run "$PARA_SCRIPT" cancel
    [ "$status" -eq 0 ]
    
    end_time=$(measure_time)
    
    total_e2e_time=$((end_time - start_time))
    echo "# Optimized end-to-end cycle: ${total_e2e_time}ms" >&3
    
    # End-to-end should be efficient
    [ "$total_e2e_time" -lt 300 ]  # 300ms for full cycle
    
    # Verify cleanup
    cd "$TEST_REPO"
    assert_session_not_exists "$session_dir"
} 