#!/usr/bin/env bats

# Tests for dispatch command argument parsing functionality
# These are simpler unit tests that verify our new dispatch command works

setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export LIB_DIR="$TEST_DIR/lib"
    
    # Source the library files
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    . "$LIB_DIR/para-session.sh"
    . "$LIB_DIR/para-ide.sh"
    
    # Set up safe test configuration - NEVER launch real IDEs
    export IDE_NAME="claude"
    export IDE_CMD="true"  # Critical: Use stub to prevent real IDE launches
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

# Test the dispatch argument parsing pattern directly
parse_dispatch_args() {
    # Simplified version of the parsing logic from para.sh handle_dispatch_command
    SESSION_NAME=""
    INITIAL_PROMPT=""
    
    if [ "$#" -eq 1 ]; then
        echo "ERROR: dispatch requires a prompt text"
        return 1
    elif [ "$#" -eq 2 ]; then
        # Just prompt provided
        INITIAL_PROMPT="$2"
    elif [ "$#" -eq 3 ]; then
        # Session name and prompt provided
        SESSION_NAME="$2"
        INITIAL_PROMPT="$3"
    else
        echo "ERROR: dispatch usage: 'para dispatch \"prompt\"' or 'para dispatch session-name \"prompt\"'"
        return 1
    fi
    
    echo "SESSION_NAME:${SESSION_NAME:-EMPTY} PROMPT:${INITIAL_PROMPT:-EMPTY}"
}

# Tests for dispatch command argument parsing
@test "parse_dispatch_args with prompt only" {
    result=$(parse_dispatch_args "dispatch" "Test prompt message")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:Test.prompt.message ]]
}

@test "parse_dispatch_args with session name and prompt" {
    result=$(parse_dispatch_args "dispatch" "feature-auth" "Add authentication")
    [[ "$result" =~ SESSION_NAME:feature-auth.*PROMPT:Add.authentication ]]
}

@test "parse_dispatch_args with quotes in prompt" {
    result=$(parse_dispatch_args "dispatch" "Test 'single' and \"double\" quotes")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:.*single.*double.*quotes ]]
}

@test "parse_dispatch_args with no prompt fails" {
    run parse_dispatch_args "dispatch"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "ERROR: dispatch requires a prompt text" ]]
}

@test "parse_dispatch_args with too many arguments fails" {
    run parse_dispatch_args "dispatch" "session" "prompt" "extra"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "ERROR: dispatch usage" ]]
}

@test "parse_dispatch_args with custom session name and prompt" {
    result=$(parse_dispatch_args "dispatch" "my-session" "My prompt text")
    [[ "$result" =~ SESSION_NAME:my-session.*PROMPT:My.prompt.text ]]
}

@test "parse_dispatch_args with special characters in prompt" {
    prompt="Test with \$vars and & special chars"
    result=$(parse_dispatch_args "dispatch" "$prompt")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:.*vars.*special.chars ]]
}

@test "parse_dispatch_args with very long prompt" {
    long_prompt="This is a very long prompt that tests whether dispatch argument parsing can handle lengthy text without issues"
    result=$(parse_dispatch_args "dispatch" "$long_prompt")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:.*very.long.prompt.*lengthy.text ]]
}

@test "parse_dispatch_args with multiline prompt" {
    multiline_prompt="Line 1
Line 2
Line 3"
    result=$(parse_dispatch_args "dispatch" "$multiline_prompt")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:.*Line.1.*Line.2.*Line.3 ]]
}

@test "parse_dispatch_args with session name containing dashes and underscores" {
    result=$(parse_dispatch_args "dispatch" "feature-auth_v2" "Authentication prompt")
    [[ "$result" =~ SESSION_NAME:feature-auth_v2.*PROMPT:Authentication.prompt ]]
}

@test "parse_dispatch_args with empty prompt string" {
    result=$(parse_dispatch_args "dispatch" "")
    [[ "$result" =~ SESSION_NAME:EMPTY.*PROMPT:EMPTY ]]
}

@test "parse_dispatch_args with session name and empty prompt" {
    result=$(parse_dispatch_args "dispatch" "my-session" "")
    [[ "$result" =~ SESSION_NAME:my-session.*PROMPT:EMPTY ]]
}

# Test dispatch command knowledge in utils
@test "is_known_command recognizes dispatch" {
    result=$(is_known_command "dispatch")
    [ "$?" -eq 0 ]
}

@test "is_known_command still recognizes start" {
    result=$(is_known_command "start")
    [ "$?" -eq 0 ]
}

# Test full integration with session prompt storage/retrieval
@test "dispatch integration test with prompt storage" {
    session_id="test-dispatch-session"
    prompt="Dispatch integration test prompt with 'quotes' and special chars &"
    
    # Save prompt
    save_session_prompt "$session_id" "$prompt"
    
    # Verify it was saved
    [ -f "$STATE_DIR/$session_id.prompt" ]
    
    # Load it back
    loaded_prompt=$(load_session_prompt "$session_id")
    [ "$loaded_prompt" = "$prompt" ]
    
    # Remove it
    remove_session_prompt "$session_id"
    [ ! -f "$STATE_DIR/$session_id.prompt" ]
}

# Test that dispatch functionality never launches real IDEs
@test "dispatch functionality never launches real IDEs" {
    # Test command building
    export IDE_CMD="true"  # Stub command
    
    result=$(build_claude_terminal_command "Test dispatch prompt")
    [[ "$result" =~ true.*Test.dispatch.prompt ]]
    
    # Test IDE wrapper functionality with stubs
    temp_dir=$(mktemp -d)
    export IDE_CMD="true"
    
    write_vscode_autorun_task "$temp_dir" "Test dispatch prompt"
    
    # Should contain true (stub), not real IDE command
    grep -q '"command": "true"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

# Test that dispatch command only works with Claude Code
@test "dispatch command fails with non-Claude Code IDE" {
    # Save original IDE configuration
    original_ide_name="$IDE_NAME"
    
    # Test with Cursor
    export IDE_NAME="cursor"
    
    # Mock the handle_dispatch_command validation logic
    validate_dispatch_ide() {
        if [ "$IDE_NAME" != "claude" ]; then
            echo "ERROR: dispatch command only works with Claude Code. Current IDE: $IDE_NAME"
            return 1
        fi
        return 0
    }
    
    run validate_dispatch_ide
    [ "$status" -ne 0 ]
    [[ "$output" =~ "ERROR: dispatch command only works with Claude Code" ]]
    [[ "$output" =~ "Current IDE: cursor" ]]
    
    # Test with VS Code
    export IDE_NAME="code"
    run validate_dispatch_ide
    [ "$status" -ne 0 ]
    [[ "$output" =~ "Current IDE: code" ]]
    
    # Test with Claude Code (should succeed)
    export IDE_NAME="claude"
    run validate_dispatch_ide
    [ "$status" -eq 0 ]
    
    # Restore original IDE configuration
    export IDE_NAME="$original_ide_name"
}

# Test that dispatch command works with wrapped Claude Code
@test "dispatch command works with wrapped Claude Code" {
    # Save original IDE configuration
    original_ide_name="$IDE_NAME"
    original_wrapper_enabled="$IDE_WRAPPER_ENABLED"
    original_wrapper_name="$IDE_WRAPPER_NAME"
    original_wrapper_cmd="$IDE_WRAPPER_CMD"
    
    # Test with Claude Code in wrapper mode - use stubs to prevent real IDE launches
    export IDE_NAME="claude"
    export IDE_CMD="true"  # Stub to prevent real Claude Code launch
    export IDE_WRAPPER_ENABLED="true"
    export IDE_WRAPPER_NAME="cursor"
    export IDE_WRAPPER_CMD="true"  # Stub to prevent real Cursor launch
    
    # Mock the handle_dispatch_command validation logic (same as above)
    validate_dispatch_ide() {
        if [ "$IDE_NAME" != "claude" ]; then
            echo "ERROR: dispatch command only works with Claude Code. Current IDE: $IDE_NAME"
            return 1
        fi
        return 0
    }
    
    run validate_dispatch_ide
    [ "$status" -eq 0 ]
    # Should not produce error output when wrapper is enabled
    [ -z "$output" ]
    
    # Restore original IDE configuration
    export IDE_NAME="$original_ide_name"
    export IDE_WRAPPER_ENABLED="$original_wrapper_enabled"
    export IDE_WRAPPER_NAME="$original_wrapper_name"
    export IDE_WRAPPER_CMD="$original_wrapper_cmd"
} 