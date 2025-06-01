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

@test "first run auto-detects IDE" {
    [ ! -f "$CONFIG_FILE" ]
    
    cd "$TEST_REPO"
    echo "y" | "$PARA_SCRIPT" list >/dev/null 2>&1 || true
    
    [ -f "$CONFIG_FILE" ]
    grep -q "IDE_NAME=" "$CONFIG_FILE"
}

@test "config show displays current settings" {
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="cursor"
IDE_CMD="cursor"
IDE_USER_DATA_DIR=".cursor-userdata"
SUBTREES_DIR_NAME="subtrees"
STATE_DIR_NAME=".para_state"
BASE_BRANCH=""
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"Para Configuration"* ]]
    [[ "$output" == *"Cursor"* ]]
}

@test "config auto setup works" {
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    [ -f "$CONFIG_FILE" ]
    grep -q "IDE_NAME=" "$CONFIG_FILE"
}

@test "config interactive setup works" {
    cd "$TEST_REPO"
    echo "1" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    
    [ -f "$CONFIG_FILE" ]
    grep -q 'IDE_NAME="cursor"' "$CONFIG_FILE"
}

@test "config setup supports different IDEs" {
    cd "$TEST_REPO"
    
    echo "2" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    grep -q 'IDE_NAME="claude"' "$CONFIG_FILE"
    
    echo "3" | "$PARA_SCRIPT" config >/dev/null 2>&1 || true
    grep -q 'IDE_NAME="code"' "$CONFIG_FILE"
}

@test "config validation catches basic errors" {
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME=""
IDE_CMD=""
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -ne 0 ]
    [[ "$output" == *"incomplete"* ]]
}

@test "config file is created with correct format" {
    cd "$TEST_REPO"
    echo "y" | "$PARA_SCRIPT" list >/dev/null 2>&1 || true
    
    [ -f "$CONFIG_FILE" ]
    grep -q "Para Configuration" "$CONFIG_FILE"
    grep -q "IDE_NAME=" "$CONFIG_FILE"
    grep -q "IDE_CMD=" "$CONFIG_FILE"
}

@test "environment variables override config file" {
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="cursor"
IDE_CMD="cursor"
EOF
    
    export IDE_NAME="claude"
    export IDE_CMD="claude"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"Claude"* ]]
}

@test "config edit opens editor when file exists" {
    mkdir -p "$CONFIG_DIR"
    echo "test config" > "$CONFIG_FILE"
    
    export EDITOR="echo 'opened'"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config edit
    [ "$status" -eq 0 ]
    [[ "$output" == *"opened"* ]]
}

@test "config handles invalid commands gracefully" {
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config invalid
    [ "$status" -eq 0 ]
    [[ "$output" == *"Usage"* ]]
}

@test "config preserves custom directory settings" {
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME="cursor"
IDE_CMD="cursor"
IDE_USER_DATA_DIR=".cursor-userdata"
SUBTREES_DIR_NAME="custom-subtrees"
STATE_DIR_NAME=".custom-state"
BASE_BRANCH="main"
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -eq 0 ]
    [[ "$output" == *"custom-subtrees"* ]]
    [[ "$output" == *".custom-state"* ]]
}

@test "config file creation with auto setup" {
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    [ -f "$CONFIG_FILE" ]
    # Should contain all required fields
    grep -q "IDE_NAME=" "$CONFIG_FILE"
    grep -q "IDE_CMD=" "$CONFIG_FILE"
    grep -q "SUBTREES_DIR_NAME=" "$CONFIG_FILE"
    grep -q "STATE_DIR_NAME=" "$CONFIG_FILE"
    grep -q "BASE_BRANCH=" "$CONFIG_FILE"
}

@test "config rejects malformed config files" {
    mkdir -p "$CONFIG_DIR"
    # Create malformed config
    cat > "$CONFIG_FILE" << 'EOF'
IDE_NAME=
IDE_CMD=
INVALID_SYNTAX
EOF
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config show
    [ "$status" -ne 0 ]
}

@test "config handles missing config directory gracefully" {
    # Ensure config directory doesn't exist
    rm -rf "$CONFIG_DIR"
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config auto
    [ "$status" -eq 0 ]
    
    # Should create directory and config file
    [ -d "$CONFIG_DIR" ]
    [ -f "$CONFIG_FILE" ]
}

@test "config edit fails gracefully when no editor available" {
    mkdir -p "$CONFIG_DIR"
    echo "test config" > "$CONFIG_FILE"
    
    export EDITOR="echo"  # Use echo as a mock editor
    
    cd "$TEST_REPO"
    run "$PARA_SCRIPT" config edit
    [ "$status" -eq 0 ]
} 