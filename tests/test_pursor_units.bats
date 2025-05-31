#!/usr/bin/env bats

# Unit tests for pure functions in pursor
# Tests functions that don't require Git or filesystem operations

setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export LIB_DIR="$TEST_DIR/lib"
    
    # Source the library files
    . "$LIB_DIR/pursor-utils.sh"
    . "$LIB_DIR/pursor-config.sh"
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
    run is_known_command "rebase"
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
    [ "$STATE_DIR" = "/test/repo/.pursor_state" ]
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
    [ "$STATE_DIR" = "/test/repo/.pursor_state" ]
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