#!/usr/bin/env bats

# Tests for prompt functionality using the new dispatch command
# This file tests the dispatch command, session prompt persistence, and command building

setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export LIB_DIR="$TEST_DIR/lib"
    
    # Source the library files
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    . "$LIB_DIR/para-session.sh"
    . "$LIB_DIR/para-ide.sh"
    
    # Set up safe test configuration
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    export IDE_WRAPPER_ENABLED="true"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="echo"
    
    # Create temporary test directory
    export TEST_TEMP_DIR=$(mktemp -d)
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"

    # Stub the launch function that outputs the command
    launch_claude_terminal_app() {
        echo "launch_claude_terminal_app called with args: $*"
    }
    export -f launch_claude_terminal_app
}

teardown() {
    # Clean up temporary directory
    [ -n "$TEST_TEMP_DIR" ] && rm -rf "$TEST_TEMP_DIR"
}

# Tests for session prompt persistence
@test "save_session_prompt stores prompt correctly" {
    # Set up a clean test directory
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    prompt="This is a test prompt"
    session_id="test-session"
    
    save_session_prompt "$session_id" "$prompt"
    
    [ -f "$STATE_DIR/$session_id.prompt" ]
    [ "$(cat "$STATE_DIR/$session_id.prompt")" = "$prompt" ]
}

@test "load_session_prompt retrieves stored prompt" {
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    prompt="Another test prompt"
    session_id="test-session-2"
    
    # Save the prompt first
    save_session_prompt "$session_id" "$prompt"
    
    # Load and verify
    result=$(load_session_prompt "$session_id")
    [ "$result" = "$prompt" ]
}

@test "load_session_prompt returns empty for non-existent session" {
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    result=$(load_session_prompt "non-existent-session")
    [ -z "$result" ]
}

@test "remove_session_prompt deletes prompt file" {
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    prompt="Prompt to be deleted"
    session_id="test-session-3"
    
    # Save and verify it exists
    save_session_prompt "$session_id" "$prompt"
    [ -f "$STATE_DIR/$session_id.prompt" ]
    
    # Remove and verify it's gone
    remove_session_prompt "$session_id"
    [ ! -f "$STATE_DIR/$session_id.prompt" ]
}

@test "save_session_prompt handles special characters" {
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    prompt='Special chars: !@#$%^&*(){}[]"'\''`~'
    session_id="test-session-special"
    
    save_session_prompt "$session_id" "$prompt"
    result=$(load_session_prompt "$session_id")
    [ "$result" = "$prompt" ]
}

@test "save_session_prompt handles multiline prompts" {
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
    
    prompt="Line 1
Line 2
Line 3"
    session_id="test-session-multiline"
    
    save_session_prompt "$session_id" "$prompt"
    result=$(load_session_prompt "$session_id")
    [ "$result" = "$prompt" ]
}

# Tests for command building functions
@test "build_claude_command returns IDE_CMD for JSON usage" {
    result=$(build_claude_command "Test prompt")
    [ "$result" = "$IDE_CMD" ]
    
    result=$(build_claude_command "")
    [ "$result" = "$IDE_CMD" ]
}

# Tests for enhanced JSON task generation with prompts
@test "write_vscode_autorun_task with prompt generates correct JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Test JSON prompt"
    
    write_vscode_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should contain the prompt as an argument
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Start Claude Code with Prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task with session resumption" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Resume prompt"
    session_id="session-abc"
    
    write_vscode_autorun_task "$temp_dir" "$prompt" "$session_id"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should contain resume arguments
    grep -q '"command":.*claude.*--resume.*session-abc.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Resume Claude Code Session with Prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task escapes quotes in JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt='Test "quoted" prompt'
    
    write_vscode_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should use temp file approach for complex content
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    # Check that temp file contains the original unescaped content
    [ -f "$temp_dir/.claude_prompt_temp" ]
    grep -q 'Test "quoted" prompt' "$temp_dir/.claude_prompt_temp"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task with prompt generates correct JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Test Cursor prompt"
    
    write_cursor_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should contain the prompt as an argument
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Start Claude Code with Prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

# Tests for IDE launch functions with prompts (using stubs)
@test "launch_claude uses wrapper mode" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="true"  # Stub command - NEVER launch real IDE
    export IDE_WRAPPER_ENABLED="true"
    export IDE_WRAPPER_NAME="code"
    prompt="Test launch prompt"
    
    # Mock the wrapper functions
    write_vscode_autorun_task() {
        echo "vscode_task_written:$1:$2"
    }
    launch_vscode_wrapper() {
        echo "vscode_wrapper_launched:$1:$2"
    }
    export -f write_vscode_autorun_task launch_vscode_wrapper
    
    result=$(launch_claude "$temp_dir" "$prompt")
    [[ "$result" =~ vscode_task_written.*Test.launch.prompt ]]
    [[ "$result" =~ vscode_wrapper_launched.*Test.launch.prompt ]]
    
    rm -rf "$temp_dir"
}

@test "launch_ide_with_wrapper passes prompt through to wrapper functions" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="true"  # Stub command - NEVER launch real IDE
    export IDE_WRAPPER_NAME="code"
    prompt="Test wrapper prompt"
    
    # Mock the wrapper functions
    write_vscode_autorun_task() {
        echo "vscode_task_written:$1:$2"
    }
    launch_vscode_wrapper() {
        echo "vscode_wrapper_launched:$1:$2"
    }
    export -f write_vscode_autorun_task launch_vscode_wrapper
    
    result=$(launch_ide_with_wrapper "claude" "$temp_dir" "$prompt")
    [[ "$result" =~ vscode_task_written.*Test.wrapper.prompt ]]
    [[ "$result" =~ vscode_wrapper_launched.*Test.wrapper.prompt ]]
    
    rm -rf "$temp_dir"
}

# Integration tests for remove_session_state cleanup
@test "remove_session_state cleans up both state and prompt files" {
    session_id="cleanup-test-session"
    
    # Create both state and prompt files
    mkdir -p "$STATE_DIR"
    echo "test state" > "$STATE_DIR/$session_id.state"
    save_session_prompt "$session_id" "test prompt"
    
    # Verify both exist
    [ -f "$STATE_DIR/$session_id.state" ]
    [ -f "$STATE_DIR/$session_id.prompt" ]
    
    # Remove session state (should clean up both)
    remove_session_state "$session_id"
    
    # Verify both are gone
    [ ! -f "$STATE_DIR/$session_id.state" ]
    [ ! -f "$STATE_DIR/$session_id.prompt" ]
} 