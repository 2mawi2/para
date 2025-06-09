#!/usr/bin/env bats

# Test wrapper for Rust binary compatibility
# Demonstrates that the wrapper allows legacy tests to run against Rust implementation

# Source common test functions  
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "RW-1: Wrapper maps environment variables to JSON config" {
    # Test basic IDE configuration mapping
    run run_para_rust config show
    [ "$status" -eq 0 ]
    [[ "$output" == *'"name": "cursor"'* ]]
    [[ "$output" == *'"command": "cursor"'* ]]
    [[ "$output" == *'"branch_prefix": "para"'* ]]
}

@test "RW-2: Wrapper handles different IDE configurations" {
    # Test IDE_NAME override
    IDE_NAME=code run run_para_rust config show
    [ "$status" -eq 0 ]
    [[ "$output" == *'"name": "code"'* ]]
    [[ "$output" == *'"command": "code"'* ]]
}

@test "RW-3: Wrapper handles wrapper mode configuration" {
    # Test wrapper mode enabled
    IDE_WRAPPER_ENABLED=true IDE_WRAPPER_NAME=cursor IDE_WRAPPER_CMD=cursor run run_para_rust config show
    [ "$status" -eq 0 ]
    [[ "$output" == *'"enabled": true'* ]]
    [[ "$output" == *'"name": "cursor"'* ]]
}

@test "RW-4: Wrapper works with list command" {
    # Test basic command functionality
    run run_para_rust list
    [ "$status" -eq 0 ]
    [[ "$output" == *"No active sessions found"* ]]
}

@test "RW-5: Wrapper preserves config isolation" {
    # Save original config state by running show command
    original_config=$(run_para_rust config show)
    
    # Run with different environment variables
    IDE_NAME=code BRANCH_PREFIX=test run run_para_rust config show
    [ "$status" -eq 0 ]
    [[ "$output" == *'"name": "code"'* ]]
    [[ "$output" == *'"branch_prefix": "test"'* ]]
    
    # Verify original config is restored after test
    restored_config=$(run_para_rust config show)
    # Should return to default values
    [[ "$restored_config" == *'"name": "cursor"'* ]]
    [[ "$restored_config" == *'"branch_prefix": "para"'* ]]
}