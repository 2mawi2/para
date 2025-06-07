#!/usr/bin/env bats

# Tests for dispatch-multi functionality
# Tests repository setup and multi-instance creation

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "DM-1: dispatch-multi creates sessions with proper repository content" {
    # 1. Set up a real repository with some files to verify copying
    cd "$TEST_REPO"
    echo "module content" > module.py
    echo "config data" > config.json
    mkdir -p src
    echo "source code" > src/main.py
    git add module.py config.json src/main.py
    git commit -m "Add test files for dispatch-multi"

    # 2. Create dispatch-multi with 2 instances
    run "$PARA_SCRIPT" dispatch-multi 2 "test multi-instance prompt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 2 instances"* ]]
    [[ "$output" == *"Initialized group"* ]]

    # 3. Verify both sessions have the repository content
    # Find the session directories
    session_dirs=$(find subtrees/para -maxdepth 1 -type d -name "*multi-*" | head -2)
    session_count=$(echo "$session_dirs" | wc -l)
    [ "$session_count" -eq 2 ]

    # Check each session has the repository files
    for session_dir in $session_dirs; do
        [ -d "$session_dir" ]
        [ -f "$session_dir/test-file.py" ]  # Original file
        [ -f "$session_dir/module.py" ]     # Added file
        [ -f "$session_dir/config.json" ]   # Added file
        [ -f "$session_dir/src/main.py" ]   # Added file in subdirectory
        
        # Verify content is correct
        assert_file_contains "$session_dir/test-file.py" "Initial content"
        assert_file_contains "$session_dir/module.py" "module content"
        assert_file_contains "$session_dir/config.json" "config data"
        assert_file_contains "$session_dir/src/main.py" "source code"
        
        # Verify it's a proper git worktree
        cd "$session_dir"
        [ -d ".git" ] || [ -f ".git" ]  # Could be file pointing to worktree or directory
        run git status
        [ "$status" -eq 0 ]
        
        # Verify we're on the correct branch
        branch_name=$(git branch --show-current)
        [[ "$branch_name" == para/* ]]
        
        cd "$TEST_REPO"
    done
}

@test "DM-2: dispatch-multi with group name creates identifiable sessions" {
    cd "$TEST_REPO"
    
    # Create dispatch-multi with custom group name
    run "$PARA_SCRIPT" dispatch-multi 3 --group myfeature "implement feature with multiple approaches"
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 3 instances for group 'myfeature'"* ]]
    [[ "$output" == *"Initialized group 'myfeature' with 3 instances"* ]]

    # List sessions to verify group information
    run "$PARA_SCRIPT" list
    [ "$status" -eq 0 ]
    [[ "$output" == *"Group: myfeature"* ]]
    [[ "$output" == *"Instance: 1/3"* ]]
    [[ "$output" == *"Instance: 2/3"* ]]
    [[ "$output" == *"Instance: 3/3"* ]]
}

@test "DM-3: dispatch-multi argument parsing handles various formats" {
    cd "$TEST_REPO"
    
    # Test count and prompt
    run "$PARA_SCRIPT" dispatch-multi 2 "simple prompt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 2 instances"* ]]
    
    # Clean up first test
    group_name=$(echo "$output" | grep "Initialized group" | sed "s/.*group '\([^']*\)'.*/\1/")
    run "$PARA_SCRIPT" cancel --group "$group_name"
    [ "$status" -eq 0 ]
    
    # Test count, group, and prompt
    run "$PARA_SCRIPT" dispatch-multi 3 --group testgroup "prompt with group"
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 3 instances for group 'testgroup'"* ]]
    
    # Clean up second test
    run "$PARA_SCRIPT" cancel --group testgroup
    [ "$status" -eq 0 ]
}

@test "DM-4: dispatch-multi requires Claude Code IDE" {
    cd "$TEST_REPO"
    
    # Test with non-Claude IDE (mock for test)
    # Set IDE to something other than claude
    export IDE_NAME="code"
    
    run "$PARA_SCRIPT" dispatch-multi 2 "test prompt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"dispatch-multi command only works with Claude Code"* ]]
    
    # Reset IDE_NAME for other tests
    unset IDE_NAME
}

@test "DM-5: dispatch-multi validates instance count" {
    cd "$TEST_REPO"
    
    # Test with invalid count (0)
    run "$PARA_SCRIPT" dispatch-multi 0 "test prompt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"instance count must be a positive integer"* ]]
    
    # Test with invalid count (non-numeric)
    run "$PARA_SCRIPT" dispatch-multi abc "test prompt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"instance count must be a positive integer"* ]]
    
    # Test with too high count
    run "$PARA_SCRIPT" dispatch-multi 15 "test prompt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"instance count limited to 10"* ]]
}

@test "DM-6: dispatch-multi saves initial prompt correctly" {
    cd "$TEST_REPO"
    
    # Create dispatch-multi session
    run "$PARA_SCRIPT" dispatch-multi 2 "implement authentication system"
    [ "$status" -eq 0 ]
    
    # Extract session IDs from output - need to parse more carefully
    group_name=$(echo "$output" | grep "Initialized group" | sed "s/.*group '\([^']*\)'.*/\1/")
    [ -n "$group_name" ]
    
    # Find the actual session IDs by looking at state files
    session_ids=""
    for state_file in .para_state/*.state; do
        [ -f "$state_file" ] || continue
        session_id=$(basename "$state_file" .state)
        if [[ "$session_id" == *"$group_name"* ]]; then
            session_ids="$session_ids $session_id"
        fi
    done
    
    # Should have found 2 sessions
    session_count=$(echo $session_ids | wc -w)
    [ "$session_count" -eq 2 ]
    
    # Verify prompt is saved for each session
    for session_id in $session_ids; do
        [ -f ".para_state/$session_id.prompt" ]
        prompt_content=$(cat ".para_state/$session_id.prompt")
        [ "$prompt_content" = "implement authentication system" ]
    done
}

@test "DM-7: dispatch-multi with file input saves prompt correctly" {
    cd "$TEST_REPO"
    
    # Create prompt file with specific content
    echo "Build a scalable microservices architecture with Docker and Kubernetes" > microservices-prompt.txt
    
    # Create dispatch-multi session with file input
    run "$PARA_SCRIPT" dispatch-multi 2 --file microservices-prompt.txt
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 2 instances"* ]]
    [[ "$output" == *"Initialized group"* ]]
    
    # Extract group name from output
    group_name=$(echo "$output" | grep "Initialized group" | sed "s/.*group '\([^']*\)'.*/\1/")
    [ -n "$group_name" ]
    
    # Find the actual session IDs by looking at state files
    session_ids=""
    for state_file in .para_state/*.state; do
        [ -f "$state_file" ] || continue
        session_id=$(basename "$state_file" .state)
        if [[ "$session_id" == *"$group_name"* ]]; then
            session_ids="$session_ids $session_id"
        fi
    done
    
    # Should have found 2 sessions
    session_count=$(echo $session_ids | wc -w)
    [ "$session_count" -eq 2 ]
    
    # Verify prompt from file is saved for each session
    for session_id in $session_ids; do
        [ -f ".para_state/$session_id.prompt" ]
        prompt_content=$(cat ".para_state/$session_id.prompt")
        [ "$prompt_content" = "Build a scalable microservices architecture with Docker and Kubernetes" ]
    done
}

@test "DM-8: dispatch-multi with -f short option saves prompt correctly" {
    cd "$TEST_REPO"
    
    # Create prompt file with specific content
    echo "Implement OAuth2 authentication with PKCE flow and refresh tokens" > oauth-prompt.md
    
    # Create dispatch-multi session with -f option
    run "$PARA_SCRIPT" dispatch-multi 2 -f oauth-prompt.md
    [ "$status" -eq 0 ]
    [[ "$output" == *"creating 2 instances"* ]]
    [[ "$output" == *"Initialized group"* ]]
    
    # Extract group name from output
    group_name=$(echo "$output" | grep "Initialized group" | sed "s/.*group '\([^']*\)'.*/\1/")
    [ -n "$group_name" ]
    
    # Find the actual session IDs by looking at state files
    session_ids=""
    for state_file in .para_state/*.state; do
        [ -f "$state_file" ] || continue
        session_id=$(basename "$state_file" .state)
        if [[ "$session_id" == *"$group_name"* ]]; then
            session_ids="$session_ids $session_id"
        fi
    done
    
    # Should have found 2 sessions
    session_count=$(echo $session_ids | wc -w)
    [ "$session_count" -eq 2 ]
    
    # Verify prompt from file is saved for each session
    for session_id in $session_ids; do
        [ -f ".para_state/$session_id.prompt" ]
        prompt_content=$(cat ".para_state/$session_id.prompt")
        [ "$prompt_content" = "Implement OAuth2 authentication with PKCE flow and refresh tokens" ]
    done
}

@test "DM-9: dispatch-multi creates proper VS Code tasks with file input" {
    cd "$TEST_REPO"
    
    # Create prompt file with complex content that could break JSON
    echo 'Design a "scalable" microservices architecture with Docker & Kubernetes' > complex-prompt.txt
    
    # Create dispatch-multi session with file input
    run "$PARA_SCRIPT" dispatch-multi 2 --file complex-prompt.txt
    [ "$status" -eq 0 ]
    
    # Extract group name from output
    group_name=$(echo "$output" | grep "Initialized group" | sed "s/.*group '\([^']*\)'.*/\1/")
    [ -n "$group_name" ]
    
    # Find the actual session IDs and check their task files
    session_ids=""
    for state_file in .para_state/*.state; do
        [ -f "$state_file" ] || continue
        session_id=$(basename "$state_file" .state)
        if [[ "$session_id" == *"$group_name"* ]]; then
            session_ids="$session_ids $session_id"
        fi
    done
    
    # Should have found 2 sessions
    session_count=$(echo $session_ids | wc -w)
    [ "$session_count" -eq 2 ]
    
    # Verify tasks.json files are created with proper JSON format
    for session_id in $session_ids; do
        IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH MERGE_MODE < ".para_state/$session_id.state"
        
        # Check that task.json exists and is valid JSON
        [ -f "$WORKTREE_DIR/.vscode/tasks.json" ]
        
        # Verify the task contains the escaped prompt correctly
        task_content=$(cat "$WORKTREE_DIR/.vscode/tasks.json")
        [[ "$task_content" == *'"Design a \"scalable\" microservices architecture with Docker & Kubernetes"'* ]]
        
        # Verify it's valid JSON by parsing it
        if command -v jq >/dev/null 2>&1; then
            echo "$task_content" | jq . >/dev/null
        elif command -v python3 >/dev/null 2>&1; then
            echo "$task_content" | python3 -m json.tool >/dev/null
        fi
    done
}