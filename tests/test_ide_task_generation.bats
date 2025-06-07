#!/usr/bin/env bats

# Tests for IDE task generation with proper shell escaping
# Tests the fix for file-based dispatch with special characters

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "TG-1: write_vscode_autorun_task creates temp file for simple prompts" {
    cd "$TEST_REPO"
    
    # Create a temporary worktree directory
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    # Source the IDE functions
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test simple prompt
    prompt="simple test prompt"
    
    # Call the function
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check that tasks.json was created
    [ -f "$worktree_dir/.vscode/tasks.json" ]
    
    # Check that temp file is referenced in the command
    grep -q "claude.*cat.*claude_prompt_temp" "$worktree_dir/.vscode/tasks.json"
    
    # Check that temp file was created with correct content
    [ -f "$worktree_dir/.claude_prompt_temp" ]
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
}

@test "TG-2: write_vscode_autorun_task handles single quotes correctly" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test prompt with single quotes (the original problem)
    prompt="simple's test here"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check temp file has correct content
    [ -f "$worktree_dir/.claude_prompt_temp" ]
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
    
    # Check tasks.json contains temp file command approach
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *'$(cat'* ]] && [[ "$tasks_content" == *'claude_prompt_temp'* ]]
}

@test "TG-3: write_vscode_autorun_task handles double quotes correctly" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test prompt with double quotes
    prompt='test "quoted string" here'
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check temp file has correct content
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
    
    # Verify the JSON is valid (no unescaped quotes breaking JSON)
    python3 -c "import json; json.load(open('$worktree_dir/.vscode/tasks.json'))" 2>/dev/null || {
        echo "Invalid JSON generated"
        cat "$worktree_dir/.vscode/tasks.json"
        return 1
    }
}

@test "TG-4: write_vscode_autorun_task handles complex multiline content" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test the exact problematic content from the issue
    prompt="please fix the following issue. sometimes when i use 'para finish \"message\"' but message contains 
signs like ' or \" then

for example para dispatch 'testtest'

gives me in claude this error:
Executing task: claude test'test 

fish: Unexpected end of string, quotes are not balanced
claude test'test
       ^"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check temp file preserves exact content including newlines
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
    
    # Check that multiline content doesn't break JSON structure
    [ -f "$worktree_dir/.vscode/tasks.json" ]
    
    # Verify JSON is still valid
    python3 -c "import json; json.load(open('$worktree_dir/.vscode/tasks.json'))" 2>/dev/null || {
        echo "Invalid JSON generated with multiline content"
        return 1
    }
}

@test "TG-5: write_vscode_autorun_task handles special shell characters" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test prompt with various shell metacharacters
    prompt='test $HOME `echo hello` $(pwd) & ; | < > ( ) { } [ ] * ? \\'
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check temp file has exact content
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
    
    # Check the command uses temp file approach (avoids direct shell escaping)
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *'$(cat'*'claude_prompt_temp'* ]]
}

@test "TG-6: write_vscode_autorun_task with skip permissions flag" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    prompt="test with permissions"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "true"
    
    # Check that --dangerously-skip-permissions is in the command
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *"--dangerously-skip-permissions"* ]]
    
    # Check temp file still works
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
}

@test "TG-7: write_vscode_autorun_task with session resumption" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    prompt="resume session test"
    session_id="test-session-123"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "$session_id" "false"
    
    # Check that --resume and session ID are in the command
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *"--resume"* ]]
    [[ "$tasks_content" == *"$session_id"* ]]
    
    # Check temp file approach is still used
    [[ "$tasks_content" == *'$(cat'*'claude_prompt_temp'* ]]
    
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
}

@test "TG-8: write_cursor_autorun_task also uses temp file approach" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    prompt="cursor test with 'quotes'"
    
    write_cursor_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check that Cursor task generation also uses temp file
    [ -f "$worktree_dir/.vscode/tasks.json" ]
    [ -f "$worktree_dir/.claude_prompt_temp" ]
    
    content=$(cat "$worktree_dir/.claude_prompt_temp")
    [ "$content" = "$prompt" ]
    
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *'$(cat'*'claude_prompt_temp'* ]]
}

@test "TG-9: temp file cleanup is included in command" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    prompt="cleanup test"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check that the command includes both cat and rm operations
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *'$(cat'*'claude_prompt_temp'*'; rm'*'claude_prompt_temp'* ]]
}

@test "TG-10: temp file path is correctly scoped to worktree" {
    cd "$TEST_REPO"
    
    worktree_dir="$TEST_REPO/test-worktree"
    mkdir -p "$worktree_dir"
    
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    prompt="path scoping test"
    
    write_vscode_autorun_task "$worktree_dir" "$prompt" "" "false"
    
    # Check temp file is created in the worktree directory
    [ -f "$worktree_dir/.claude_prompt_temp" ]
    
    # Check the command references the full path to the temp file
    tasks_content=$(cat "$worktree_dir/.vscode/tasks.json")
    [[ "$tasks_content" == *"$worktree_dir/.claude_prompt_temp"* ]]
}