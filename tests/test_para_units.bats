#!/usr/bin/env bats

# Unit tests for pure functions in para
# Tests functions that don't require Git or filesystem operations

setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export LIB_DIR="$TEST_DIR/lib"
    
    # Source the library files
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    . "$LIB_DIR/para-ide.sh"
}

# Tests for validate_session_name function
@test "validate_session_name accepts valid names" {
    # Should accept feature_x
    run validate_session_name "feature_x"
    [ "$status" -eq 0 ]
    
    # Should accept other valid formats
    run validate_session_name "feature-auth"
    [ "$status" -eq 0 ]
    
    run validate_session_name "test123"
    [ "$status" -eq 0 ]
    
    run validate_session_name "feature_test_123"
    [ "$status" -eq 0 ]
}

@test "validate_session_name rejects invalid names" {
    # Should reject foo!
    run validate_session_name "foo!"
    [ "$status" -ne 0 ]
    
    # Should reject names with spaces
    run validate_session_name "bad name"
    [ "$status" -ne 0 ]
    
    # Should reject names with special characters
    run validate_session_name "test@feature"
    [ "$status" -ne 0 ]
    
    run validate_session_name "feature#1"
    [ "$status" -ne 0 ]
    
    run validate_session_name "test.feature"
    [ "$status" -ne 0 ]
}

# Tests for is_known_command function
@test "is_known_command returns true for known commands" {
    # Known commands from the case statement
    run is_known_command "start"
    [ "$status" -eq 0 ]
    
    run is_known_command "finish"
    [ "$status" -eq 0 ]
    
    run is_known_command "clean"
    [ "$status" -eq 0 ]
    
    run is_known_command "list"
    [ "$status" -eq 0 ]
    
    run is_known_command "ls"
    [ "$status" -eq 0 ]
    
    run is_known_command "continue"
    [ "$status" -eq 0 ]
    
    run is_known_command "cancel"
    [ "$status" -eq 0 ]
    
    run is_known_command "abort"
    [ "$status" -eq 0 ]
    
    run is_known_command "resume"
    [ "$status" -eq 0 ]
    
    run is_known_command "--help"
    [ "$status" -eq 0 ]
    
    run is_known_command "-h"
    [ "$status" -eq 0 ]
    
    run is_known_command "--preserve"
    [ "$status" -eq 0 ]
}

@test "is_known_command returns false for unknown commands" {
    # Should return false for foobar
    run is_known_command "foobar"
    [ "$status" -ne 0 ]
    
    # Should return false for other unknown commands
    run is_known_command "unknown"
    [ "$status" -ne 0 ]
    
    run is_known_command "invalid"
    [ "$status" -ne 0 ]
    
    run is_known_command ""
    [ "$status" -ne 0 ]
}

# Tests for generate_timestamp function with mocked date
@test "generate_timestamp returns YYYYMMDD-HHMMSS pattern" {
    # Mock the date command to return a fixed date
    date() {
        case "$1" in
            "+%Y%m%d-%H%M%S")
                echo "20240531-184623"
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    result=$(generate_timestamp)
    [[ "$result" =~ ^[0-9]{8}-[0-9]{6}$ ]]
    [ "$result" = "20240531-184623" ]
}

@test "generate_timestamp without mock returns valid format" {
    # Test that the function returns a valid timestamp format
    result=$(generate_timestamp)
    [[ "$result" =~ ^[0-9]{8}-[0-9]{6}$ ]]
}

# Tests for init_paths with environment overrides
@test "init_paths uses default SUBTREES_DIR_NAME" {
    # Set up mock REPO_ROOT
    export REPO_ROOT="/test/repo"
    export SUBTREES_DIR_NAME=""  # Clear any existing value
    
    # Load config first to set defaults
    load_config
    
    # Call init_paths
    init_paths
    
    # Should use default "subtrees"
    [ "$SUBTREES_DIR" = "/test/repo/subtrees" ]
    [ "$STATE_DIR" = "/test/repo/.para_state" ]
}

@test "init_paths respects SUBTREES_DIR_NAME override" {
    # Set up mock REPO_ROOT and override
    export REPO_ROOT="/test/repo"
    export SUBTREES_DIR_NAME="wt"
    
    # Load config first
    load_config
    
    # Call init_paths
    init_paths
    
    # Should use overridden value "wt"
    [ "$SUBTREES_DIR" = "/test/repo/wt" ]
    [ "$STATE_DIR" = "/test/repo/.para_state" ]
}

@test "init_paths respects STATE_DIR_NAME override" {
    # Set up mock REPO_ROOT and override
    export REPO_ROOT="/test/repo"
    export STATE_DIR_NAME=".custom_state"
    
    # Load config first
    load_config
    
    # Call init_paths
    init_paths
    
    # Should use overridden state dir name
    [ "$STATE_DIR" = "/test/repo/.custom_state" ]
    [ "$SUBTREES_DIR" = "/test/repo/subtrees" ]
}

# Tests for friendly name generation
@test "generate_friendly_name returns valid format" {
    # Test that the function returns adjective_noun format
    result=$(generate_friendly_name)
    [[ "$result" =~ ^[a-z]+_[a-z]+$ ]]
}

@test "generate_friendly_name is deterministic for same timestamp" {
    # Mock date to return same timestamp
    date() {
        case "$1" in
            "+%s")
                echo "1640995200"  # Fixed epoch time
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    result1=$(generate_friendly_name)
    result2=$(generate_friendly_name)
    [ "$result1" = "$result2" ]
    [[ "$result1" =~ ^[a-z]+_[a-z]+$ ]]
}

@test "generate_session_id returns friendly name with timestamp" {
    # Mock both date commands
    date() {
        case "$1" in
            "+%s")
                echo "1640995200"  # Fixed epoch time for friendly name
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
    
    result=$(generate_session_id)
    [[ "$result" =~ ^[a-z]+_[a-z]+_[0-9]{8}-[0-9]{6}$ ]]
    [[ "$result" =~ _20240531-184623$ ]]
}

# Tests for config validation edge cases
@test "validate_ide_name accepts known IDEs" {
    run validate_ide_name "cursor"
    [ "$status" -eq 0 ]
    
    run validate_ide_name "claude"
    [ "$status" -eq 0 ]
    
    run validate_ide_name "code"
    [ "$status" -eq 0 ]
}

@test "validate_ide_name accepts custom IDE names" {
    run validate_ide_name "custom-ide"
    [ "$status" -eq 0 ]
    
    run validate_ide_name "MyIDE"
    [ "$status" -eq 0 ]
}

@test "validate_ide_name rejects empty IDE name" {
    run validate_ide_name ""
    [ "$status" -ne 0 ]
}

@test "validate_config fails with empty IDE_NAME" {
    export IDE_NAME=""
    export IDE_CMD="cursor"
    export SUBTREES_DIR_NAME="subtrees"
    export STATE_DIR_NAME=".para_state"
    
    run validate_config
    [ "$status" -ne 0 ]
    [[ "$output" =~ "IDE configuration is incomplete" ]]
}

@test "validate_config fails with empty IDE_CMD" {
    export IDE_NAME="cursor"
    export IDE_CMD=""
    export SUBTREES_DIR_NAME="subtrees"
    export STATE_DIR_NAME=".para_state"
    
    run validate_config
    [ "$status" -ne 0 ]
    [[ "$output" =~ "IDE configuration is incomplete" ]]
}

@test "validate_config fails with path separators in directory names" {
    export IDE_NAME="cursor"
    export IDE_CMD="cursor"
    export SUBTREES_DIR_NAME="sub/trees"
    export STATE_DIR_NAME=".para_state"
    
    run validate_config
    [ "$status" -ne 0 ]
    [[ "$output" =~ "Directory names cannot contain path separators" ]]
}

@test "validate_config passes with valid configuration" {
    export IDE_NAME="cursor"
    export IDE_CMD="cursor"
    export SUBTREES_DIR_NAME="subtrees"
    export STATE_DIR_NAME=".para_state"
    
    run validate_config
    [ "$status" -eq 0 ]
}

@test "get_default_user_data_dir returns correct paths for known IDEs" {
    result=$(get_default_user_data_dir "cursor")
    [ "$result" = ".cursor-userdata" ]
    
    result=$(get_default_user_data_dir "code")
    [ "$result" = ".vscode-userdata" ]
    
    result=$(get_default_user_data_dir "claude")
    [ "$result" = "" ]
}

@test "get_default_user_data_dir returns generic path for unknown IDEs" {
    result=$(get_default_user_data_dir "myide")
    [ "$result" = ".myide-userdata" ]
}

@test "get_ide_display_name returns proper display names" {
    # Mock IDE_NAME for testing
    export IDE_NAME="cursor"
    result=$(get_ide_display_name)
    [ "$result" = "Cursor" ]
    
    export IDE_NAME="claude"
    result=$(get_ide_display_name)
    [ "$result" = "Claude Code" ]
    
    export IDE_NAME="code"
    result=$(get_ide_display_name)
    [ "$result" = "VS Code" ]
    
    export IDE_NAME="unknown"
    result=$(get_ide_display_name)
    [ "$result" = "unknown" ]
}

# Tests for friendly name edge cases and consistency
@test "generate_friendly_name uses only safe characters" {
    result=$(generate_friendly_name)
    # Should only contain lowercase letters and underscore
    [[ "$result" =~ ^[a-z_]+$ ]]
    # Should not contain consecutive underscores
    [[ ! "$result" =~ __ ]]
    # Should start and end with letters, not underscore
    [[ "$result" =~ ^[a-z] ]]
    [[ "$result" =~ [a-z]$ ]]
}

@test "generate_friendly_name produces different names for different timestamps" {
    # Mock date for first call
    date() {
        case "$1" in
            "+%s")
                echo "1640995200"
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    result1=$(generate_friendly_name)
    
    # Mock date for second call with different timestamp
    date() {
        case "$1" in
            "+%s")
                echo "1640995300"  # Different timestamp
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    result2=$(generate_friendly_name)
    
    # Should produce different names
    [ "$result1" != "$result2" ]
    
    # Both should be valid format
    [[ "$result1" =~ ^[a-z]+_[a-z]+$ ]]
    [[ "$result2" =~ ^[a-z]+_[a-z]+$ ]]
}

@test "generate_friendly_name has reasonable length" {
    result=$(generate_friendly_name)
    length=${#result}
    
    # Should be between 6 and 20 characters (reasonable for typing)
    [ "$length" -ge 6 ]
    [ "$length" -le 20 ]
}

@test "friendly names contain exactly one underscore separator" {
    result=$(generate_friendly_name)
    underscore_count=$(echo "$result" | tr -cd '_' | wc -c)
    [ "$underscore_count" -eq 1 ]
}

# Tests for validate_session_name edge cases
@test "validate_session_name accepts single character names" {
    run validate_session_name "a"
    [ "$status" -eq 0 ]
    
    run validate_session_name "1"
    [ "$status" -eq 0 ]
}

@test "validate_session_name accepts maximum length reasonable names" {
    long_name="feature_authentication_system_with_oauth_2024"
    run validate_session_name "$long_name"
    [ "$status" -eq 0 ]
}

@test "validate_session_name rejects names with dots" {
    run validate_session_name "feature.auth"
    [ "$status" -ne 0 ]
}

@test "validate_session_name rejects names with slashes" {
    run validate_session_name "feature/auth"
    [ "$status" -ne 0 ]
    
    run validate_session_name "feature\\auth"
    [ "$status" -ne 0 ]
}

# Tests for error handling
@test "die function exits with error" {
    # Test die function in a subshell to avoid exiting the test
    run bash -c '. "$LIB_DIR/para-utils.sh"; die "test error message"'
    [ "$status" -eq 1 ]
    [[ "$output" =~ "para: test error message" ]]
}

# Tests for is_known_command comprehensive coverage
@test "is_known_command recognizes all documented commands" {
    # Test all commands that should be recognized
    known_commands="start finish clean list ls continue cancel abort resume config"
    for cmd in $known_commands; do
        run is_known_command "$cmd"
        [ "$status" -eq 0 ]
    done
}

@test "is_known_command recognizes help flags" {
    run is_known_command "--help"
    [ "$status" -eq 0 ]
    
    run is_known_command "-h"
    [ "$status" -eq 0 ]
    
    run is_known_command "--preserve"
    [ "$status" -eq 0 ]
}

@test "is_known_command case sensitivity" {
    # Should be case sensitive
    run is_known_command "START"
    [ "$status" -ne 0 ]
    
    run is_known_command "List"
    [ "$status" -ne 0 ]
}

# Tests for path initialization with various scenarios
@test "init_paths creates correct relative paths" {
    export REPO_ROOT="/test/repo"
    export SUBTREES_DIR_NAME="worktrees"
    export STATE_DIR_NAME=".para_data"
    export IDE_NAME="cursor"
    
    init_paths
    
    [ "$SUBTREES_DIR" = "/test/repo/worktrees" ]
    [ "$STATE_DIR" = "/test/repo/.para_data" ]
    [[ "$TEMPLATE_DIR" =~ cursor-template$ ]]
}

@test "init_paths handles IDE name case conversion" {
    export REPO_ROOT="/test/repo"
    export IDE_NAME="CURSOR"
    
    init_paths
    
    [[ "$TEMPLATE_DIR" =~ cursor-template$ ]]
}

# Tests for edge cases in timestamp generation
@test "generate_timestamp format consistency" {
    # Call multiple times and verify format is consistent
    ts1=$(generate_timestamp)
    ts2=$(generate_timestamp)
    
    # Both should match the expected pattern
    [[ "$ts1" =~ ^[0-9]{8}-[0-9]{6}$ ]]
    [[ "$ts2" =~ ^[0-9]{8}-[0-9]{6}$ ]]
}

@test "generate_session_id combines components correctly" {
    # Mock both date functions for predictable output
    date() {
        case "$1" in
            "+%s")
                echo "1609459200"  # 2021-01-01 00:00:00 UTC
                ;;
            "+%Y%m%d-%H%M%S")
                echo "20210101-000000"
                ;;
            *)
                command date "$@"
                ;;
        esac
    }
    export -f date
    
    result=$(generate_session_id)
    
    # Should end with the timestamp
    [[ "$result" =~ _20210101-000000$ ]]
    
    # Should start with adjective_noun
    [[ "$result" =~ ^[a-z]+_[a-z]+_ ]]
    
    # Should have exactly 3 parts separated by underscores
    part_count=$(echo "$result" | tr -cd '_' | wc -c)
    [ "$part_count" -eq 2 ]
}

# Tests for IDE wrapper functionality
@test "write_vscode_autorun_task creates correct tasks.json" {
    temp_dir=$(mktemp -d)
    
    write_vscode_autorun_task "$temp_dir"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task file contains expected content
    grep -q '"label": "claude"' "$temp_dir/.vscode/tasks.json"
    grep -q '"command": "claude"' "$temp_dir/.vscode/tasks.json"
    grep -q '"runOn": "folderOpen"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task creates correct tasks.json" {
    temp_dir=$(mktemp -d)
    
    write_cursor_autorun_task "$temp_dir"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task file contains expected content
    grep -q '"label": "claude"' "$temp_dir/.vscode/tasks.json"
    grep -q '"command": "claude"' "$temp_dir/.vscode/tasks.json"
    grep -q '"runOn": "folderOpen"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "launch_ide_with_wrapper calls correct wrapper function" {
    temp_dir=$(mktemp -d)
    
    # Mock the wrapper functions to verify they're called
    launch_vscode_wrapper() {
        echo "vscode_wrapper_called:$1"
    }
    launch_cursor_wrapper() {
        echo "cursor_wrapper_called:$1"
    }
    export -f launch_vscode_wrapper launch_cursor_wrapper
    
    # Test VS Code wrapper
    export IDE_WRAPPER_NAME="code"
    result=$(launch_ide_with_wrapper "claude" "$temp_dir")
    [[ "$result" =~ vscode_wrapper_called ]]
    
    # Test Cursor wrapper
    export IDE_WRAPPER_NAME="cursor"
    result=$(launch_ide_with_wrapper "claude" "$temp_dir")
    [[ "$result" =~ cursor_wrapper_called ]]
    
    rm -rf "$temp_dir"
}

@test "launch_ide_with_wrapper falls back to regular claude for unsupported wrapper" {
    temp_dir=$(mktemp -d)
    
    # Mock launch_claude to verify fallback
    launch_claude() {
        echo "fallback_called:$1"
    }
    export -f launch_claude
    
    export IDE_WRAPPER_NAME="unsupported"
    result=$(launch_ide_with_wrapper "claude" "$temp_dir")
    [[ "$result" =~ fallback_called ]]
    
    rm -rf "$temp_dir"
} 