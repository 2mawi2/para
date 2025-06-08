#!/usr/bin/env bats

# Unit tests for pure functions in para
# Tests functions that don't require Git or filesystem operations

# Source common test functions  
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

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
    run is_known_command "start"
    [ "$status" -eq 0 ]
    
    run is_known_command "finish"
    [ "$status" -eq 0 ]
    
    run is_known_command "list"
    [ "$status" -eq 0 ]
    
    run is_known_command "cancel"
    [ "$status" -eq 0 ]
    
    run is_known_command "clean"
    [ "$status" -eq 0 ]
    
    run is_known_command "--help"
    [ "$status" -eq 0 ]
    
    run is_known_command "-h"
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
    known_commands="start finish clean list ls cancel abort resume config"
    for cmd in $known_commands; do
        run is_known_command "$cmd"
        [ "$status" -eq 0 ] || {
            echo "Command '$cmd' should be recognized as known"
            return 1
        }
    done
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

# Tests for is_file_path function
@test "is_file_path returns false for empty string" {
    run is_file_path ""
    [ "$status" -ne 0 ]
}

@test "is_file_path returns true for existing files" {
    # Create a temporary file
    temp_file=$(mktemp)
    
    run is_file_path "$temp_file"
    [ "$status" -eq 0 ]
    
    rm -f "$temp_file"
}

@test "is_file_path returns true for paths with separators" {
    run is_file_path "path/to/file"
    [ "$status" -eq 0 ]
    
    run is_file_path "/absolute/path"
    [ "$status" -eq 0 ]
    
    run is_file_path "./relative/path"
    [ "$status" -eq 0 ]
}

@test "is_file_path returns true for common text extensions" {
    run is_file_path "README.md"
    [ "$status" -eq 0 ]
    
    run is_file_path "notes.txt"
    [ "$status" -eq 0 ]
    
    run is_file_path "document.rst"
    [ "$status" -eq 0 ]
    
    run is_file_path "notes.org"
    [ "$status" -eq 0 ]
}

@test "is_file_path returns true for prompt and template extensions" {
    run is_file_path "task.prompt"
    [ "$status" -eq 0 ]
    
    run is_file_path "config.tmpl"
    [ "$status" -eq 0 ]
    
    run is_file_path "base.template"
    [ "$status" -eq 0 ]
}

@test "is_file_path returns false for plain strings without indicators" {
    run is_file_path "simple"
    [ "$status" -ne 0 ]
    
    run is_file_path "command"
    [ "$status" -ne 0 ]
    
    run is_file_path "test123"
    [ "$status" -ne 0 ]
    
    run is_file_path "feature-auth"
    [ "$status" -ne 0 ]
}

@test "is_file_path handles edge cases" {
    # Single character paths
    run is_file_path "/"
    [ "$status" -eq 0 ]
    
    # Hidden files with paths
    run is_file_path ".hidden/file"
    [ "$status" -eq 0 ]
    
    # Files with no extension but with path
    run is_file_path "bin/command"
    [ "$status" -eq 0 ]
}

@test "is_file_path correctly identifies URLs as NOT file paths" {
    # HTTPS URLs should not be detected as file paths
    run is_file_path "https://github.com/eyaltoledano/claude-task-master"
    [ "$status" -ne 0 ]
    
    # HTTP URLs should not be detected as file paths
    run is_file_path "http://example.com/page"
    [ "$status" -ne 0 ]
    
    # FTP URLs should not be detected as file paths
    run is_file_path "ftp://files.example.com/data"
    [ "$status" -ne 0 ]
    
    # SSH URLs should not be detected as file paths
    run is_file_path "ssh://user@host/path"
    [ "$status" -ne 0 ]
}

@test "is_file_path correctly handles prompts with URLs" {
    # Long prompts containing URLs should not be detected as file paths
    long_prompt_with_url="Make me a plan how to integrate this tool in a fully automated software development cycle https://github.com/eyaltoledano/claude-task-master where each of the tasks that are being split by Taskmaster can be distributed among multiple agents in parallel"
    run is_file_path "$long_prompt_with_url"
    [ "$status" -ne 0 ]
    
    # Short prompts with URLs should not be detected as file paths
    run is_file_path "Check out https://example.com for details"
    [ "$status" -ne 0 ]
    
    # Prompts with multiple URLs should not be detected as file paths
    run is_file_path "Compare https://site1.com and https://site2.com"
    [ "$status" -ne 0 ]
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
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    write_vscode_autorun_task "$temp_dir"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task file contains expected content (new format uses full command)
    grep -q '"label": "Start Claude Code"' "$temp_dir/.vscode/tasks.json"
    grep -q '"command":.*claude' "$temp_dir/.vscode/tasks.json"
    grep -q '"runOn": "folderOpen"' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task creates correct tasks.json" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    write_cursor_autorun_task "$temp_dir"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task file contains expected content (new format uses full command)
    grep -q '"label": "Start Claude Code"' "$temp_dir/.vscode/tasks.json"
    grep -q '"command":.*claude' "$temp_dir/.vscode/tasks.json"
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

# Tests for configurable branch prefix functionality
@test "get_branch_prefix returns default para prefix" {
    # Clear any existing environment variable
    unset PARA_BRANCH_PREFIX
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(get_branch_prefix)
    [ "$result" = "para" ]
}

@test "get_branch_prefix respects PARA_BRANCH_PREFIX environment variable" {
    export PARA_BRANCH_PREFIX="feature"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(get_branch_prefix)
    [ "$result" = "feature" ]
}

@test "get_branch_prefix handles empty environment variable" {
    export PARA_BRANCH_PREFIX=""
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(get_branch_prefix)
    [ "$result" = "para" ]
}

@test "validate_branch_prefix accepts valid prefixes" {
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    # Valid prefixes
    run validate_branch_prefix "para"
    [ "$status" -eq 0 ]
    
    run validate_branch_prefix "feature"
    [ "$status" -eq 0 ]
    
    run validate_branch_prefix "ai"
    [ "$status" -eq 0 ]
    
    run validate_branch_prefix "dev"
    [ "$status" -eq 0 ]
}

@test "validate_branch_prefix rejects invalid prefixes" {
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    # Invalid prefixes with special characters
    run validate_branch_prefix "para/"
    [ "$status" -ne 0 ]
    
    run validate_branch_prefix "feature*"
    [ "$status" -ne 0 ]
    
    run validate_branch_prefix "ai?"
    [ "$status" -ne 0 ]
    
    run validate_branch_prefix "dev:"
    [ "$status" -ne 0 ]
    
    run validate_branch_prefix "test@"
    [ "$status" -ne 0 ]
}

@test "generate_clean_branch_name creates valid branch names from session names" {
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    # Test basic conversion
    result=$(generate_clean_branch_name "test session")
    [ "$result" = "test-session" ]
    
    # Test special character handling
    result=$(generate_clean_branch_name "user_auth")
    [ "$result" = "user-auth" ]
    
    # Test uppercase conversion
    result=$(generate_clean_branch_name "FeatureAuth")
    [ "$result" = "featureauth" ]
    
    # Test multiple spaces
    result=$(generate_clean_branch_name "test   multiple   spaces")
    [ "$result" = "test-multiple-spaces" ]
}

@test "generate_target_branch_name combines prefix and clean name" {
    export PARA_BRANCH_PREFIX="feature"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_target_branch_name "user auth")
    [ "$result" = "feature/user-auth" ]
}

@test "generate_target_branch_name handles empty session name" {
    export PARA_BRANCH_PREFIX="para"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_target_branch_name "")
    [ "$result" = "para/unnamed" ]
}

# Integration tests for configurable prefix functionality
@test "configurable prefix integration - feature prefix" {
    export PARA_BRANCH_PREFIX="feature"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_target_branch_name "user auth")
    [ "$result" = "feature/user-auth" ]
}

@test "configurable prefix integration - ai prefix" {
    export PARA_BRANCH_PREFIX="ai"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_target_branch_name "implement JWT")
    [ "$result" = "ai/implement-jwt" ]
}

@test "configurable prefix integration - single char prefix" {
    export PARA_BRANCH_PREFIX="x"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_target_branch_name "test")
    [ "$result" = "x/test" ]
}

@test "generate_clean_branch_name handles edge cases" {
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    
    result=$(generate_clean_branch_name "Test_Name-123")
    [ "$result" = "test-name-123" ]

    result=$(generate_clean_branch_name "  invalid chars!@#$%  ")
    [ "$result" = "invalid-chars" ]

    result=$(generate_clean_branch_name "")
    [ "$result" = "unnamed" ]
}

# Tests for new branch validation functions
@test "validate_target_branch_name accepts valid branch names" {
  # Source git functions to test
  . "$LIB_DIR/para-git.sh"
  
  run validate_target_branch_name "feature-auth"
  [ "$status" -eq 0 ]

  run validate_target_branch_name "bugfix/login-issue"
  [ "$status" -eq 0 ]

  run validate_target_branch_name "feature/user-authentication"
  [ "$status" -eq 0 ]

  run validate_target_branch_name "hotfix-123"
  [ "$status" -eq 0 ]
}

@test "validate_target_branch_name rejects invalid branch names" {
  # Source git functions to test
  . "$LIB_DIR/para-git.sh"
  
  # Empty name
  run validate_target_branch_name ""
  [ "$status" -ne 0 ]
  [[ "$output" == *"cannot be empty"* ]]

  # Invalid characters
  run validate_target_branch_name "feature with spaces"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid characters"* ]]

  run validate_target_branch_name "feature~branch"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid characters"* ]]

  run validate_target_branch_name "feature:branch"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid characters"* ]]

  # Cannot start with dash or dot
  run validate_target_branch_name "-feature"
  [ "$status" -ne 0 ]
  [[ "$output" == *"cannot start with"* ]]

  run validate_target_branch_name ".feature"
  [ "$status" -ne 0 ]
  [[ "$output" == *"cannot start with"* ]]

  # Cannot end with slash
  run validate_target_branch_name "feature/"
  [ "$status" -ne 0 ]
  [[ "$output" == *"cannot end with"* ]]

  # Cannot contain /.
  run validate_target_branch_name "feature/.config"
  [ "$status" -ne 0 ]
  [[ "$output" == *"cannot contain '/.' sequence"* ]]
}

@test "generate_unique_branch_name returns original when no conflict" {
  setup_temp_git_repo
  cd "$TEST_REPO"
  
  # Source git functions to test
  . "$LIB_DIR/para-git.sh"

  result=$(generate_unique_branch_name "new-feature")
  [ "$result" = "new-feature" ]
}

@test "generate_unique_branch_name adds suffix when branch exists" {
  setup_temp_git_repo
  cd "$TEST_REPO"
  
  # Source git functions to test
  . "$LIB_DIR/para-git.sh"

  # Get the actual default branch name (could be main or master)
  default_branch=$(git rev-parse --abbrev-ref HEAD)

  # Create a branch that will conflict
  git checkout -b existing-feature
  git checkout "$default_branch"

  result=$(generate_unique_branch_name "existing-feature")
  [ "$result" = "existing-feature-1" ]

  # Create the -1 version too
  git checkout -b existing-feature-1
  git checkout "$default_branch"

  result=$(generate_unique_branch_name "existing-feature")
  [ "$result" = "existing-feature-2" ]
}

# Tests for IDE task generation with argument escaping
@test "write_vscode_autorun_task handles prompts starting with dash" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with prompt that starts with dash (like .actrc content)
    write_vscode_autorun_task "$temp_dir" "-P macos-latest=catthehacker/ubuntu:act-latest"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task contains the -- separator to prevent CLI parsing
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task handles prompts starting with dash" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with prompt that starts with dash
    write_cursor_autorun_task "$temp_dir" "--version"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task contains the -- separator
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task handles normal prompts without separator" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with normal prompt that doesn't start with dash
    write_vscode_autorun_task "$temp_dir" "Create a user authentication system"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the task does NOT contain the -- separator for normal prompts
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task handles normal prompts without separator" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with normal prompt
    write_cursor_autorun_task "$temp_dir" "Implement OAuth2 flow"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify no -- separator for normal prompts
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task handles skip permissions flag with dash prompts" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with skip permissions and dash prompt
    write_vscode_autorun_task "$temp_dir" "-f some-config.yml" "" "true"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify both skip permissions flag and -- separator are present
    # -- separator is added before the last argument (prompt) if it starts with -
    grep -q '"command":.*claude.*--dangerously-skip-permissions.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task handles skip permissions flag with dash prompts" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with skip permissions and dash prompt
    write_cursor_autorun_task "$temp_dir" "-help" "" "true"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify both flags are present in correct order
    # -- separator is added before the last argument (prompt) if it starts with -
    grep -q '"command":.*claude.*--dangerously-skip-permissions.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_vscode_autorun_task handles complex prompts with special characters" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test with complex prompt containing quotes and special chars
    complex_prompt='-P macos-latest="catthehacker/ubuntu:act-latest" --verbose'
    write_vscode_autorun_task "$temp_dir" "$complex_prompt"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # Verify the complex prompt is properly escaped in JSON
    grep -q '"command":.*claude.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

@test "write_cursor_autorun_task handles session resumption with dash prompts" {
    temp_dir=$(mktemp -d)
    
    # Set IDE_CMD for the test
    export IDE_CMD="claude"
    
    # Test session resumption with dash prompt
    write_cursor_autorun_task "$temp_dir" "--config production" "test-session-123"
    
    [ -f "$temp_dir/.vscode/tasks.json" ]
    
    # With updated logic, -- separator is only added before the last argument (prompt) if it starts with -
    grep -q '"command":.*claude.*--resume.*test-session-123.*cat.*claude_prompt_temp' "$temp_dir/.vscode/tasks.json"
    
    rm -rf "$temp_dir"
}

# Tests for finish_session function
@test "finish_session with no changes prints 'no changes to commit'" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Run finish_session with no changes
    result=$(finish_session "$temp_branch" "$worktree_dir" "$(git rev-parse --abbrev-ref HEAD)" "Test commit message")
    
    [[ "$result" =~ "no changes to commit" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
}

@test "finish_session with uncommitted changes stages and commits them" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add uncommitted changes in the worktree
    cd "$worktree_dir"
    echo "new content" > new-file.txt
    echo "modified content" >> test-file.py
    
    cd "$TEST_REPO"
    
    # Run finish_session
    result=$(finish_session "$temp_branch" "$worktree_dir" "$(git rev-parse --abbrev-ref HEAD)" "Test commit message")
    
    [[ "$result" =~ "staging all changes" ]]
    [[ "$result" =~ "committing changes" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify the commit was created
    cd "$worktree_dir"
    [ "$(git log --oneline | head -1)" = "$(git log --oneline | head -1 | grep 'Test commit message')" ]
}

@test "finish_session squashes multiple commits into single commit" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Create multiple commits in the worktree
    cd "$worktree_dir"
    echo "commit 1" > file1.txt
    git add file1.txt
    git commit -m "First commit"
    
    echo "commit 2" > file2.txt
    git add file2.txt
    git commit -m "Second commit"
    
    echo "commit 3" > file3.txt
    git add file3.txt
    git commit -m "Third commit"
    
    cd "$TEST_REPO"
    
    # Run finish_session
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Squashed commit message")
    
    [[ "$result" =~ "squashing commits into single commit" ]]
    [[ "$result" =~ "squashed 3 commits into single commit" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify only one commit exists above base
    cd "$worktree_dir"
    commit_count=$(git rev-list --count HEAD ^"$base_branch")
    [ "$commit_count" -eq 1 ]
    
    # Verify the commit message
    last_commit_msg=$(git log --format=%B -n 1)
    [[ "$last_commit_msg" =~ "Squashed commit message" ]]
}

@test "finish_session with single commit does not squash" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Create a single commit in the worktree
    cd "$worktree_dir"
    echo "single commit" > file1.txt
    git add file1.txt
    git commit -m "Single commit"
    
    cd "$TEST_REPO"
    
    # Run finish_session
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "New commit message")
    
    [[ "$result" =~ "only one commit, no squashing needed" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify only one commit exists above base
    cd "$worktree_dir"
    commit_count=$(git rev-list --count HEAD ^"$base_branch")
    [ "$commit_count" -eq 1 ]
}

@test "finish_session with custom branch name renames branch" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    custom_branch_name="feature/custom-auth"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a commit in the worktree
    cd "$worktree_dir"
    echo "custom feature" > custom.txt
    git add custom.txt
    git commit -m "Custom feature commit"
    
    cd "$TEST_REPO"
    
    # Run finish_session with custom branch name
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Custom commit message" "$custom_branch_name")
    
    [[ "$result" =~ "renaming branch from '$temp_branch' to '$custom_branch_name'" ]]
    [[ "$result" =~ "Your changes are ready on branch: $custom_branch_name" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify the branch was renamed
    git branch --list | grep -q "$custom_branch_name"
    [ $? -eq 0 ]
    
    # Verify the old branch doesn't exist
    ! git branch --list | grep -q "$temp_branch"
}

@test "finish_session with existing custom branch name generates unique name" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create an existing branch with the target name
    custom_branch_name="feature/auth"
    git checkout -b "$custom_branch_name"
    git checkout "$base_branch"
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a commit in the worktree
    cd "$worktree_dir"
    echo "new feature" > new-feature.txt
    git add new-feature.txt
    git commit -m "New feature commit"
    
    cd "$TEST_REPO"
    
    # Run finish_session with existing custom branch name
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "New commit message" "$custom_branch_name")
    
    [[ "$result" =~ "branch '$custom_branch_name' already exists, using '$custom_branch_name-1' instead" ]]
    [[ "$result" =~ "renaming branch from '$temp_branch' to '$custom_branch_name-1'" ]]
    [[ "$result" =~ "Your changes are ready on branch: $custom_branch_name-1" ]]
    
    # Verify the unique branch was created
    git branch --list | grep -q "$custom_branch_name-1"
    [ $? -eq 0 ]
}

@test "finish_session restores original directory on completion" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a test directory and change to it
    test_dir="$TEST_REPO/test-working-dir"
    mkdir -p "$test_dir"
    cd "$test_dir"
    original_pwd="$PWD"
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git -C "$TEST_REPO" worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Run finish_session from test directory
    finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit"
    
    # Verify we're back in the original directory
    [ "$PWD" = "$original_pwd" ]
}

@test "finish_session handles mixed uncommitted and untracked files" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add mixed changes in the worktree
    cd "$worktree_dir"
    echo "untracked file" > untracked.txt          # Untracked file
    echo "modified content" >> test-file.py        # Modified file
    echo "new tracked file" > new-tracked.txt      # New file to stage
    git add new-tracked.txt                        # Stage one file but not others
    
    cd "$TEST_REPO"
    
    # Run finish_session
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Mixed changes commit")
    
    [[ "$result" =~ "staging all changes" ]]
    [[ "$result" =~ "committing changes" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify all files were committed
    cd "$worktree_dir"
    [ -f "untracked.txt" ]
    [ -f "new-tracked.txt" ]
    git ls-files | grep -q "untracked.txt"
    git ls-files | grep -q "new-tracked.txt"
    
    # Verify the working tree is clean
    [ -z "$(git status --porcelain)" ]
} 