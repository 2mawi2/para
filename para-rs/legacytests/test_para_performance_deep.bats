#!/usr/bin/env bats

# Deep performance analysis for para session operations
# Focuses on identifying real-world performance bottlenecks

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

# Test cancellation with many worktrees (potential bottleneck)
@test "DEEP-1: Performance with many existing worktrees" {
    cd "$TEST_REPO"
    
    # Create many worktrees first to simulate a busy repository
    echo "# Creating 10 background worktrees..." >&3
    for i in $(seq 1 10); do
        run run_para "background-session-$i"
        [ "$status" -eq 0 ]
        
        session_dir=$(find_session_dir)
        cd "$TEST_REPO/$session_dir"
        echo "Background session $i" > "bg-file-$i.txt"
        git add .
        git commit -m "Background session $i"
        cd "$TEST_REPO"
    done
    
    # Now test cancellation performance with many existing worktrees
    run run_para "target-session"
    [ "$status" -eq 0 ]
    
    target_session_dir=$(find_session_dir)
    cd "$TEST_REPO/$target_session_dir"
    echo "Target session content" > "target-file.txt"
    git add .
    git commit -m "Target session commit"
    
    # Measure cancellation time in busy environment
    start_time=$(measure_time)
    run "$PARA_SCRIPT" cancel
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    duration=$((end_time - start_time))
    echo "# Cancellation with 10 existing worktrees took: ${duration}ms" >&3
    
    # Should still be fast even with many worktrees
    [ "$duration" -lt 1000 ]  # 1 second
    
    # Cleanup background sessions
    cd "$TEST_REPO"
    run run_para clean
    [ "$status" -eq 0 ]
}

# Test cancellation with deep directory structure
@test "DEEP-2: Performance with deep directory structure" {
    cd "$TEST_REPO"
    
    run run_para "deep-test"
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    cd "$TEST_REPO/$session_dir"
    
    # Create deep directory structure
    echo "# Creating deep directory structure..." >&3
    mkdir -p "level1/level2/level3/level4/level5"
    for level in level1 level1/level2 level1/level2/level3 level1/level2/level3/level4 level1/level2/level3/level4/level5; do
        for i in $(seq 1 5); do
            echo "Deep file content $i" > "$level/file-$i.txt"
        done
    done
    
    git add .
    git commit -m "Deep directory structure"
    
    # Measure cancellation time
    start_time=$(measure_time)
    run "$PARA_SCRIPT" cancel
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    duration=$((end_time - start_time))
    echo "# Cancellation with deep directories took: ${duration}ms" >&3
    
    # Should handle deep structures efficiently
    [ "$duration" -lt 1000 ]  # 1 second
}

# Test cancellation with many git objects/commits
@test "DEEP-3: Performance with many commits" {
    cd "$TEST_REPO"
    
    run run_para "many-commits-test"
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    cd "$TEST_REPO/$session_dir"
    
    # Create many commits to simulate long-running session
    echo "# Creating 50 commits..." >&3
    for i in $(seq 1 50); do
        echo "Commit $i content" > "commit-$i.txt"
        git add "commit-$i.txt"
        git commit -m "Commit $i"
        
        # Show progress every 10 commits
        if [ $((i % 10)) -eq 0 ]; then
            echo "# Created $i commits..." >&3
        fi
    done
    
    # Measure cancellation time with many commits
    start_time=$(measure_time)
    run "$PARA_SCRIPT" cancel
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    duration=$((end_time - start_time))
    echo "# Cancellation with 50 commits took: ${duration}ms" >&3
    
    # Should handle many commits efficiently
    [ "$duration" -lt 2000 ]  # 2 seconds
}

# Test git performance under different conditions
@test "DEEP-4: Git operation stress test" {
    cd "$TEST_REPO"
    
    # Create a session with complex git state
    run run_para "git-stress-test"
    [ "$status" -eq 0 ]
    
    session_dir=$(find_session_dir)
    cd "$TEST_REPO/$session_dir"
    
    branch_name=$(git branch --show-current)
    
    # Create complex git state
    echo "# Creating complex git state..." >&3
    
    # Many files
    for i in $(seq 1 20); do
        echo "File $i content" > "file-$i.txt"
    done
    git add .
    git commit -m "Initial files"
    
    # Modify and stage some files
    for i in $(seq 1 5); do
        echo "Modified content $i" >> "file-$i.txt"
    done
    git add file-1.txt file-2.txt
    
    # Leave some files unstaged
    echo "Unstaged content" >> "file-10.txt"
    
    cd "$TEST_REPO"
    
    # Measure individual git operations under stress
    echo "# Testing git worktree remove performance..." >&3
    start_time=$(measure_time)
    git worktree remove --force "$session_dir" 2>/dev/null
    end_time=$(measure_time)
    worktree_time=$((end_time - start_time))
    echo "# git worktree remove under stress took: ${worktree_time}ms" >&3
    
    echo "# Testing git branch delete performance..." >&3
    start_time=$(measure_time)
    git branch -D "$branch_name" 2>/dev/null
    end_time=$(measure_time)
    branch_time=$((end_time - start_time))
    echo "# git branch delete under stress took: ${branch_time}ms" >&3
    
    total_time=$((worktree_time + branch_time))
    echo "# Total git operations under stress: ${total_time}ms" >&3
    
    # Git operations should remain fast even under stress
    [ "$worktree_time" -lt 1000 ]  # 1 second
    [ "$branch_time" -lt 500 ]     # 0.5 seconds
}

# Test filesystem performance bottlenecks
@test "DEEP-5: Filesystem performance analysis" {
    cd "$TEST_REPO"
    
    # Test state file operations under load
    echo "# Testing state file performance with multiple states..." >&3
    
    # Create many state files to simulate busy system
    mkdir -p ".para_state"
    for i in $(seq 1 100); do
        echo "branch$i|worktree$i|base$i|squash" > ".para_state/session-$i.state"
        echo "prompt for session $i" > ".para_state/session-$i.prompt"
    done
    
    # Measure removal of one state file among many
    start_time=$(measure_time)
    rm -f ".para_state/session-50.state"
    rm -f ".para_state/session-50.prompt"
    end_time=$(measure_time)
    
    fs_time=$((end_time - start_time))
    echo "# State file removal with 100 existing states took: ${fs_time}ms" >&3
    
    # Cleanup
    rm -rf ".para_state"
    
    # Filesystem ops should be very fast
    [ "$fs_time" -lt 50 ]  # 50ms
}

# Test system under load (simulate concurrent operations)
@test "DEEP-6: Concurrent operation simulation" {
    cd "$TEST_REPO"
    
    # This test simulates what happens when cancellation occurs
    # while the system is under load (multiple git operations)
    
    # Create background "load"
    echo "# Setting up background load..." >&3
    for i in $(seq 1 5); do
        run run_para "load-session-$i"
        [ "$status" -eq 0 ]
        
        session_dir=$(find_session_dir)
        cd "$TEST_REPO/$session_dir"
        echo "Load session $i" > "load-file-$i.txt"
        git add .
        git commit -m "Load session $i"
        cd "$TEST_REPO"
    done
    
    # Create target session
    run run_para "target-under-load"
    [ "$status" -eq 0 ]
    
    target_dir=$(find_session_dir)
    cd "$TEST_REPO/$target_dir"
    echo "Target content" > "target.txt"
    git add .
    git commit -m "Target commit"
    
    # Simulate concurrent git operations by checking git status 
    # in background while cancelling
    start_time=$(measure_time)
    
    # Start background "load" (multiple git status checks)
    (
        for i in $(seq 1 10); do
            git status >/dev/null 2>&1 &
        done
        wait
    ) &
    
    # Cancel while background operations run
    run "$PARA_SCRIPT" cancel
    
    # Wait for background to complete
    wait
    
    end_time=$(measure_time)
    
    [ "$status" -eq 0 ]
    duration=$((end_time - start_time))
    echo "# Cancellation under concurrent load took: ${duration}ms" >&3
    
    # Should handle concurrent operations gracefully
    [ "$duration" -lt 2000 ]  # 2 seconds
    
    # Cleanup
    cd "$TEST_REPO"
    run run_para clean
    [ "$status" -eq 0 ]
} 