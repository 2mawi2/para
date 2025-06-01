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
    export IDE_CMD="true"  # Use stub command - NEVER launch real IDE
    export STATE_DIR_NAME=".para_state"
    
    # Create temporary test directory
    export TEST_TEMP_DIR=$(mktemp -d)
    export STATE_DIR="$TEST_TEMP_DIR/.para_state"
    mkdir -p "$STATE_DIR"
}

teardown() {
    # Clean up temporary directory
    [ -n "$TEST_TEMP_DIR" ] && rm -rf "$TEST_TEMP_DIR"
}

# Tests for session prompt persistence
@test "save_session_prompt stores prompt correctly" {
    session_id="test-session"
    prompt="Test prompt for session"
    
    save_session_prompt "$session_id" "$prompt"
    
    [ -f "$STATE_DIR/$session_id.prompt" ]
    stored_prompt=$(cat "$STATE_DIR/$session_id.prompt")
    [ "$stored_prompt" = "$prompt" ]
}

@test "load_session_prompt retrieves stored prompt" {
    session_id="test-session"
    prompt="Test prompt for loading"
    
    # Store prompt first
    save_session_prompt "$session_id" "$prompt"
    
    # Load it back
    loaded_prompt=$(load_session_prompt "$session_id")
    [ "$loaded_prompt" = "$prompt" ]
}

@test "load_session_prompt returns empty for non-existent session" {
    session_id="non-existent-session"
    
    loaded_prompt=$(load_session_prompt "$session_id")
    [ -z "$loaded_prompt" ]
}

@test "remove_session_prompt deletes prompt file" {
    session_id="test-session"
    prompt="Test prompt for deletion"
    
    # Store prompt first
    save_session_prompt "$session_id" "$prompt"
    [ -f "$STATE_DIR/$session_id.prompt" ]
    
    # Remove it
    remove_session_prompt "$session_id"
    [ ! -f "$STATE_DIR/$session_id.prompt" ]
}

@test "save_session_prompt handles special characters" {
    session_id="test-session"
    prompt="Test 'prompt' with \"quotes\" and $special & characters"
    
    save_session_prompt "$session_id" "$prompt"
    loaded_prompt=$(load_session_prompt "$session_id")
    [ "$loaded_prompt" = "$prompt" ]
}

@test "save_session_prompt handles multiline prompts" {
    session_id="test-session"
    prompt="Line 1
Line 2
Line 3"
    
    save_session_prompt "$session_id" "$prompt"
    loaded_prompt=$(load_session_prompt "$session_id")
    [ "$loaded_prompt" = "$prompt" ]
}

# Tests for command building functions
@test "build_claude_terminal_command with simple prompt" {
    export IDE_CMD="claude"
    prompt="Simple test prompt"
    
    result=$(build_claude_terminal_command "$prompt")
    expected="claude 'Simple test prompt'"
    [ "$result" = "$expected" ]
}

@test "build_claude_terminal_command with quotes in prompt" {
    export IDE_CMD="claude"
    prompt="Test 'single' and \"double\" quotes"
    
    result=$(build_claude_terminal_command "$prompt")
    # The function escapes single quotes but keeps double quotes as-is
    expected="claude 'Test '\''single'\'' and \"double\" quotes'"
    [ "$result" = "$expected" ]
}

@test "build_claude_terminal_command without prompt" {
    export IDE_CMD="claude"
    
    result=$(build_claude_terminal_command "")
    [ "$result" = "claude" ]
}

@test "build_claude_terminal_command with session resumption" {
    export IDE_CMD="claude"
    prompt="Resume with prompt"
    session_id="session-123"
    
    result=$(build_claude_terminal_command "$prompt" "$session_id")
    expected="claude --resume 'session-123' 'Resume with prompt'"
    [ "$result" = "$expected" ]
}

@test "build_claude_terminal_command resume without prompt" {
    export IDE_CMD="claude"
    session_id="session-123"
    
    result=$(build_claude_terminal_command "" "$session_id")
    expected="claude --resume 'session-123'"
    [ "$result" = "$expected" ]
}

@test "build_claude_command returns IDE_CMD for JSON usage" {
    export IDE_CMD="claude"
    prompt="Test prompt"
    
    result=$(build_claude_command "$prompt")
    [ "$result" = "claude" ]
    
    result_no_prompt=$(build_claude_command "")
    [ "$result_no_prompt" = "claude" ]
}

# Tests for enhanced JSON task generation with prompts
@test "write_vscode_autorun_task with prompt generates correct JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Test JSON prompt"
    
    write_vscode_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should contain the prompt as an argument
    grep -q '"args": \["Test JSON prompt"\]' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Start Claude Code with Prompt"' "$temp_dir/.vscode/tasks.json"
    grep -q '"command": "claude"' "$temp_dir/.vscode/tasks.json"
    
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
    grep -q '"args": \["--resume", "session-abc", "Resume prompt"\]' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Resume Claude Code Session with Prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task escapes quotes in JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt='Test "quoted" prompt'
    
    write_vscode_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should properly escape quotes in JSON
    grep -q '"Test \\"quoted\\" prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task with prompt generates correct JSON" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Test Cursor prompt"
    
    write_cursor_autorun_task "$temp_dir" "$prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Should contain the prompt as an argument
    grep -q '"args": \["Test Cursor prompt"\]' "$temp_dir/.vscode/tasks.json"
    grep -q '"label": "Start Claude Code with Prompt"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

# Tests for IDE launch functions with prompts (using stubs)
@test "launch_claude passes prompt to terminal command builder" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="true"  # Stub command - NEVER launch real IDE
    export CLAUDE_TERMINAL_CMD="terminal"
    prompt="Test launch prompt"
    
    # Mock the terminal app launcher to capture what command would be built
    launch_claude_terminal_app() {
        echo "terminal_app_called:$1:$2"
    }
    export -f launch_claude_terminal_app
    
    result=$(launch_claude "$temp_dir" "$prompt")
    [[ "$result" =~ terminal_app_called.*Test.launch.prompt ]]
    
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

@test "launch_claude_custom_terminal uses prompt in command substitution" {
    temp_dir=$(mktemp -d)
    export IDE_CMD="claude"
    prompt="Custom terminal prompt"
    custom_cmd="echo 'Terminal: %d Command: %c'"
    
    result=$(launch_claude_custom_terminal "$temp_dir" "$custom_cmd" "$prompt")
    
    # Should substitute %d with directory and %c with full claude command including prompt
    [[ "$result" =~ Terminal:.*$temp_dir ]]
    [[ "$result" =~ Command:.*claude.*Custom.terminal.prompt ]]
    
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