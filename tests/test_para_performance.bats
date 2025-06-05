#!/usr/bin/env bats

# Performance benchmark tests for para session operations
# Tests performance characteristics of session lifecycle operations

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
    
    # Create some baseline commits to make the repo more realistic
    cd "$TEST_REPO"
    for i in $(seq 1 5); do
        echo "Baseline file $i content" > "baseline-$i.txt"
        git add "baseline-$i.txt"
        git commit -m "Add baseline file $i"
    done
}

teardown() {
    teardown_temp_git_repo
}

# Utility function to measure time (in milliseconds)
measure_time() {
    # Use perl for high-precision timing if available, otherwise date
    if command -v perl >/dev/null 2>&1; then
        perl -MTime::HiRes=time -E 'say int(time*1000)'
    else
        # Fallback to date (less precise, but works on all systems)
        date +%s%3N 2>/dev/null || date +%s000
    fi
}

# Benchmark session creation
@test "PERF-1: Benchmark session creation time" {
    cd "$TEST_REPO"
    
    start_time=$(measure_time)
    run run_para
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    
    duration=$((end_time - start_time))
    echo "# Session creation took: ${duration}ms" >&3
    
    # Session creation should complete within reasonable time (adjust threshold as needed)
    [ "$duration" -lt 5000 ]  # 5 seconds
}

# Benchmark single session cancellation 
@test "PERF-2: Benchmark single session cancellation time" {
    cd "$TEST_REPO"
    
    # Create session first
    run run_para
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Add some files to make cancellation more realistic
    cd "$TEST_REPO/$session_dir"
    for i in $(seq 1 10); do
        echo "Test file $i content" > "test-file-$i.txt"
    done
    git add .
    git commit -m "Add test files"
    
    # Measure cancellation time
    start_time=$(measure_time)
    run "$PARA_SCRIPT" cancel
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    
    duration=$((end_time - start_time))
    echo "# Session cancellation took: ${duration}ms" >&3
    
    # Cancellation should be fast
    [ "$duration" -lt 3000 ]  # 3 seconds
    
    # Verify session is gone
    cd "$TEST_REPO"
    assert_session_not_exists "$session_dir"
}

# Benchmark multiple session cancellation (stress test)
@test "PERF-3: Benchmark multiple sessions creation and bulk cancellation" {
    cd "$TEST_REPO"
    
    # Create multiple sessions with unique names
    session_count=5
    
    echo "# Creating $session_count sessions..." >&3
    total_creation_time=0
    
    for i in $(seq 1 "$session_count"); do
        start_time=$(measure_time)
        run run_para "perf-test-session-$i"
        end_time=$(measure_time)
        
        [ "$status" -eq 0 ]
        duration=$((end_time - start_time))
        total_creation_time=$((total_creation_time + duration))
        
        # Store session for later verification
        session_dir=$(find_session_dir)
        assert_session_exists "$session_dir"
        
        # Add some content to each session
        cd "$TEST_REPO/$session_dir"
        echo "Session $i content" > "session-$i.txt"
        git add .
        git commit -m "Session $i commit"
        cd "$TEST_REPO"
        
        # Small delay to ensure different timestamps
        sleep 0.1
    done
    
    avg_creation_time=$((total_creation_time / session_count))
    echo "# Average session creation time: ${avg_creation_time}ms" >&3
    echo "# Total creation time for $session_count sessions: ${total_creation_time}ms" >&3
    
    # Verify all sessions exist
    session_count_actual=$(count_sessions)
    [ "$session_count_actual" -eq "$session_count" ]
    
    # Measure bulk cleanup time
    start_time=$(measure_time)
    run run_para clean
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    
    cleanup_duration=$((end_time - start_time))
    echo "# Bulk cleanup of $session_count sessions took: ${cleanup_duration}ms" >&3
    
    # Bulk cleanup should be efficient
    [ "$cleanup_duration" -lt 10000 ]  # 10 seconds
    
    # Verify all sessions are gone
    final_session_count=$(count_sessions)
    [ "$final_session_count" -eq 0 ]
}

# Benchmark session cancellation with large files
@test "PERF-4: Benchmark cancellation with large files" {
    cd "$TEST_REPO"
    
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    assert_session_exists "$session_dir"
    
    # Add large files to test worktree removal performance
    cd "$TEST_REPO/$session_dir"
    
    echo "# Creating large files..." >&3
    # Create several moderately large files (not huge to avoid test slowness)
    for i in $(seq 1 3); do
        # Create ~1MB file
        head -c 1048576 /dev/zero > "large-file-$i.bin" 2>/dev/null || \
        dd if=/dev/zero of="large-file-$i.bin" bs=1024 count=1024 2>/dev/null
    done
    
    # Create many small files
    for i in $(seq 1 100); do
        echo "Small file $i content" > "small-file-$i.txt"
    done
    
    git add .
    git commit -m "Add large and many files"
    
    # Measure cancellation time with large worktree
    start_time=$(measure_time)
    run "$PARA_SCRIPT" cancel
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    
    duration=$((end_time - start_time))
    echo "# Session cancellation with large files took: ${duration}ms" >&3
    
    # Should still be reasonably fast even with large files
    [ "$duration" -lt 5000 ]  # 5 seconds
    
    # Verify session is gone
    cd "$TEST_REPO"
    assert_session_not_exists "$session_dir"
}

# Benchmark individual git operations to identify bottlenecks
@test "PERF-5: Benchmark individual git operations" {
    cd "$TEST_REPO"
    
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    cd "$TEST_REPO/$session_dir"
    
    # Get the branch name and worktree path
    branch_name=$(git branch --show-current)
    worktree_path="$TEST_REPO/$session_dir"
    
    # Add some content
    echo "Test content" > test-file.txt
    git add test-file.txt
    git commit -m "Test commit"
    
    cd "$TEST_REPO"
    
    # Benchmark individual git operations that happen during cancellation
    
    # 1. git worktree remove
    start_time=$(measure_time)
    git worktree remove --force "$worktree_path" 2>/dev/null
    end_time=$(measure_time)
    worktree_remove_time=$((end_time - start_time))
    echo "# git worktree remove took: ${worktree_remove_time}ms" >&3
    
    # 2. git branch -D
    start_time=$(measure_time)
    git branch -D "$branch_name" 2>/dev/null
    end_time=$(measure_time)
    branch_delete_time=$((end_time - start_time))
    echo "# git branch -D took: ${branch_delete_time}ms" >&3
    
    # Clean up manually since we did it step by step
    session_id=$(basename "$session_dir")
    rm -f ".para_state/$session_id.state" 2>/dev/null || true
    
    total_git_time=$((worktree_remove_time + branch_delete_time))
    echo "# Total git operations time: ${total_git_time}ms" >&3
    
    # Individual operations should be fast
    [ "$worktree_remove_time" -lt 2000 ]  # 2 seconds
    [ "$branch_delete_time" -lt 1000 ]    # 1 second
}

# Benchmark filesystem operations during cancellation
@test "PERF-6: Benchmark filesystem operations" {
    cd "$TEST_REPO"
    
    # Create session
    run run_para
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    session_id=$(basename "$session_dir")
    
    # Create state files
    mkdir -p ".para_state"
    echo "test|state|data|squash" > ".para_state/$session_id.state"
    echo "test prompt" > ".para_state/$session_id.prompt"
    
    # Benchmark state file removal
    start_time=$(measure_time)
    rm -f ".para_state/$session_id.state"
    rm -f ".para_state/$session_id.prompt"
    rmdir ".para_state" 2>/dev/null || true
    end_time=$(measure_time)
    
    fs_time=$((end_time - start_time))
    echo "# Filesystem operations took: ${fs_time}ms" >&3
    
    # Filesystem operations should be very fast
    [ "$fs_time" -lt 100 ]  # 100ms
    
    # Clean up the session properly using git operations directly
    cd "$TEST_REPO/$session_dir"
    branch_name=$(git branch --show-current)
    cd "$TEST_REPO"
    
    # Remove worktree and branch manually since we removed state files
    git worktree remove --force "$session_dir" 2>/dev/null || true
    git branch -D "$branch_name" 2>/dev/null || true
}

# Performance regression test - baseline for future optimizations
@test "PERF-7: Performance regression baseline" {
    cd "$TEST_REPO"
    
    # This test establishes a baseline for overall performance
    # It creates, modifies, and cancels multiple sessions while measuring total time
    
    total_operations=3
    total_start_time=$(measure_time)
    
    for i in $(seq 1 "$total_operations"); do
        # Create session
        run run_para
        [ "$status" -eq 0 ]
        
        session_dir=$(find_session_dir)
        
        # Add content
        cd "$TEST_REPO/$session_dir"
        echo "Session $i content" > "file-$i.txt"
        git add .
        git commit -m "Session $i commit"
        
        # Cancel session
        run "$PARA_SCRIPT" cancel
        [ "$status" -eq 0 ]
        
        cd "$TEST_REPO"
        assert_session_not_exists "$session_dir"
    done
    
    total_end_time=$(measure_time)
    total_duration=$((total_end_time - total_start_time))
    avg_per_operation=$((total_duration / total_operations))
    
    echo "# Total time for $total_operations create-modify-cancel cycles: ${total_duration}ms" >&3
    echo "# Average time per operation: ${avg_per_operation}ms" >&3
    
    # Establish reasonable performance baseline
    [ "$avg_per_operation" -lt 3000 ]  # 3 seconds per full cycle
} 