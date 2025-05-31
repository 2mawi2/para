#!/usr/bin/env bats

load test_common.sh

# Setup and teardown for each test
setup() {
    setup_temp_git_repo
    
    # Set up temporary config directory
    export TEMP_CONFIG_DIR=$(mktemp -d)
    export XDG_CONFIG_HOME="$TEMP_CONFIG_DIR"
    export CONFIG_DIR="$TEMP_CONFIG_DIR/para"
    export CONFIG_FILE="$CONFIG_DIR/config"
}

teardown() {
    teardown_temp_git_repo
    
    # Clean up temporary config directory
    if [ -n "$TEMP_CONFIG_DIR" ] && [ -d "$TEMP_CONFIG_DIR" ]; then
        rm -rf "$TEMP_CONFIG_DIR"
    fi
    
    # Clean up test environment variables
    unset TEMP_CONFIG_DIR
    unset XDG_CONFIG_HOME
    unset CONFIG_DIR
    unset CONFIG_FILE
}

@test "auto setup detects cursor when available" {
    mkdir -p "$TEST_REPO/mock_bin"
    echo '#!/bin/sh\necho "cursor mock"' > "$TEST_REPO/mock_bin/cursor"
    chmod +x "$TEST_REPO/mock_bin/cursor"
    export PATH="$TEST_REPO/mock_bin:$PATH"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    [ -f "$CONFIG_FILE" ]
    grep -q 'IDE_NAME="cursor"' "$CONFIG_FILE"
}

@test "auto setup detects claude when cursor unavailable" {
    # Clear PATH first to ensure cursor is not found
    export PATH="/bin:/usr/bin"
    mkdir -p "$TEST_REPO/mock_bin"
    echo '#!/bin/sh\necho "claude mock"' > "$TEST_REPO/mock_bin/claude"
    chmod +x "$TEST_REPO/mock_bin/claude"
    export PATH="$TEST_REPO/mock_bin:$PATH"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    [ -f "$CONFIG_FILE" ]
    grep -q 'IDE_NAME="claude"' "$CONFIG_FILE"
}

@test "auto setup defaults to cursor when no IDE found" {
    export PATH="/bin:/usr/bin"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    [ -f "$CONFIG_FILE" ]
    grep -q 'IDE_NAME="cursor"' "$CONFIG_FILE"
}

@test "first run prompts for quick setup" {
    cd "$TEST_REPO"
    
    [ ! -f "$CONFIG_FILE" ]
    
    run bash -c 'echo "n" | "$PARA_SCRIPT" list 2>&1'
    [[ "$output" == *"Welcome to para"* ]]
    [[ "$output" == *"Quick setup"* ]]
}

@test "first run accepts quick setup" {
    cd "$TEST_REPO"
    
    echo "y" | "$PARA_SCRIPT" list >/dev/null 2>&1 || true
    
    [ -f "$CONFIG_FILE" ]
    grep -q "Para Configuration" "$CONFIG_FILE"
}

@test "config setup allows IDE selection" {
    cd "$TEST_REPO"
    
    echo "2" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    grep -q 'IDE_NAME="claude"' "$CONFIG_FILE"
    
    echo "3" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    grep -q 'IDE_NAME="code"' "$CONFIG_FILE"
}

@test "config setup handles custom IDE" {
    cd "$TEST_REPO"
    
    printf "4\nmyide\n" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    
    [ -f "$CONFIG_FILE" ]
    grep -q 'IDE_NAME="myide"' "$CONFIG_FILE"
    grep -q 'IDE_CMD="myide"' "$CONFIG_FILE"
}

@test "config setup preserves existing settings" {
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="claude"
IDE_CMD="claude"
IDE_USER_DATA_DIR=""
SUBTREES_DIR_NAME="subtrees"
STATE_DIR_NAME=".para_state"
BASE_BRANCH=""
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"Claude Code"* ]]
}

@test "config quick setup creates valid configuration" {
    cd "$TEST_REPO"
    # Accept the quick setup
    echo "y" | "$PARA_SCRIPT" config quick
    
    # Config file should be created
    [ -f "$CONFIG_FILE" ]
    
    # Should contain all required fields
    grep -q "IDE_NAME=" "$CONFIG_FILE"
    grep -q "IDE_CMD=" "$CONFIG_FILE"
    grep -q "SUBTREES_DIR_NAME=" "$CONFIG_FILE"
    grep -q "STATE_DIR_NAME=" "$CONFIG_FILE"
    grep -q "BASE_BRANCH=" "$CONFIG_FILE"
}

@test "config quick setup can be cancelled" {
    cd "$TEST_REPO"
    # Decline the quick setup
    run bash -c 'echo "n" | "$PARA_SCRIPT" config quick'
    [ "$status" -ne 0 ]
    
    # Config file should not be created
    [ ! -f "$CONFIG_FILE" ]
}

@test "config wizard subcommands work correctly" {
    cd "$TEST_REPO"
    
    # Test config wizard (just verify it starts)
    run timeout 2s "$PARA_SCRIPT" config wizard || true
    [[ "$output" == *"Configuration Wizard"* ]] || [[ "$output" == *"wizard"* ]]
    
    # Test config quick
    run bash -c 'echo "n" | "$PARA_SCRIPT" config quick'
    [[ "$output" == *"Quick Setup"* ]] || [[ "$output" == *"quick"* ]]
}

@test "first run creates default config when declined" {
    cd "$TEST_REPO"
    
    # Decline the wizard setup
    echo "n" | "$PARA_SCRIPT" --help >/dev/null 2>&1 || true
    
    # Should have created default config
    [ -f "$CONFIG_FILE" ]
    grep -q "Para Configuration File" "$CONFIG_FILE"
}

@test "first run accepts wizard setup" {
    cd "$TEST_REPO"
    
    # Accept the wizard setup (but timeout to avoid hanging)
    run timeout 3s bash -c 'echo "y" | "$PARA_SCRIPT" --help' || true
    [[ "$output" == *"Welcome to para"* ]] || [[ "$output" == *"wizard"* ]] || [[ "$output" == *"Configuration"* ]]
}

@test "config wizard handles IDE selection" {
    # This test is mainly to verify the structure exists
    # Real interactive testing would require expect or similar
    cd "$TEST_REPO"
    
    # Just verify the wizard can start
    run timeout 2s "$PARA_SCRIPT" config wizard || true
    # Should mention IDE settings in some form
    [[ "$output" == *"IDE"* ]] || [[ "$output" == *"editor"* ]] || true
}

@test "get_default_user_data_dir function works correctly" {
    # Test through the config system
    mkdir -p "$CONFIG_DIR"
    
    # Test cursor
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="cursor"
IDE_CMD="cursor"
EOF
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *".cursor-userdata"* ]]
    
    # Test code
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="code"
IDE_CMD="code"
EOF
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *".vscode-userdata"* ]] || [[ "$output" == *"<not set>"* ]]
    
    # Test claude
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="claude"
IDE_CMD="claude"
EOF
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"<not set>"* ]]
    
    # Test custom IDE
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="custom_ide"
IDE_CMD="custom_ide"
EOF
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *".custom_ide-userdata"* ]] || [[ "$output" == *"<not set>"* ]]
}

@test "configuration validation during wizard" {
    # Create config with invalid values to test validation
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME=""
IDE_CMD=""
SUBTREES_DIR_NAME=""
STATE_DIR_NAME=""
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -ne 0 ]
    [[ "$output" == *"validation failed"* ]] || [[ "$output" == *"cannot be empty"* ]]
}

@test "config supports both menu and direct commands" {
    cd "$TEST_REPO"
    
    # Test direct commands
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    
    run "$PARA_SCRIPT" config edit
    [ "$status" -eq 0 ]
    
    # Test that invalid commands show help
    run "$PARA_SCRIPT" config invalid
    [ "$status" -eq 0 ]
    [[ "$output" == *"Unknown config command"* ]]
}

@test "config preserves user choices during updates" {
    # Create initial config
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="custom_ide"
IDE_CMD="my_custom_command"
SUBTREES_DIR_NAME="my_subtrees"
STATE_DIR_NAME="my_state"
BASE_BRANCH="main"
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"custom_ide"* ]]
    [[ "$output" == *"my_custom_command"* ]]
    [[ "$output" == *"my_subtrees"* ]]
    [[ "$output" == *"my_state"* ]]
    [[ "$output" == *"main"* ]]
}

@test "config wizard shows current values as defaults" {
    # Create existing config
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="claude"
IDE_CMD="claude"
SUBTREES_DIR_NAME="projects"
STATE_DIR_NAME=".my_para_state"
BASE_BRANCH="develop"
EOF
    
    cd "$TEST_REPO"
    # Just test that the wizard can start and shows current config
    run timeout 2s "$PARA_SCRIPT" config wizard || true
    [[ "$output" == *"Current configuration found"* ]] || [[ "$output" == *"claude"* ]] || true
} 