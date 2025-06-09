#!/usr/bin/env sh

RUST_BINARY="${1:-./para-rs/target/debug/para}"
shift
COMMAND="$*"

if [ -z "$COMMAND" ]; then
    echo "Usage: $0 [rust-binary-path] <command> [args...]" >&2
    exit 1
fi

if [ ! -x "$RUST_BINARY" ]; then
    echo "Error: Rust binary not found or not executable: $RUST_BINARY" >&2
    exit 1
fi

CONFIG_FILE=""
if command -v para-rs >/dev/null 2>&1; then
    CONFIG_FILE=$("$RUST_BINARY" config path 2>/dev/null || echo "")
fi

if [ -z "$CONFIG_FILE" ]; then
    if [ "$(uname)" = "Darwin" ]; then
        CONFIG_FILE="$HOME/Library/Application Support/para/config.json"
    else
        CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/para/config.json"
    fi
fi

BACKUP_CONFIG=""
BACKUP_EXISTS=false
if [ -f "$CONFIG_FILE" ]; then
    BACKUP_CONFIG=$(cat "$CONFIG_FILE")
    BACKUP_EXISTS=true
fi

generate_test_config() {
    IDE_NAME="${IDE_NAME:-cursor}"
    IDE_CMD="${IDE_CMD:-cursor}"
    CURSOR_CMD="${CURSOR_CMD:-cursor}"
    IDE_USER_DATA_DIR="${IDE_USER_DATA_DIR:-}"
    
    # Set IDE_CMD based on IDE_NAME if not explicitly set to a custom value
    if [ "${IDE_CMD:-}" = "cursor" ] || [ "${IDE_CMD:-}" = "echo 'mock-ide-launched'" ]; then
        case "$IDE_NAME" in
            code)
                IDE_CMD="code"
                ;;
            claude)
                IDE_CMD="claude"
                ;;
            cursor)
                IDE_CMD="cursor"
                ;;
        esac
    fi
    
    IDE_WRAPPER_ENABLED="${IDE_WRAPPER_ENABLED:-false}"
    IDE_WRAPPER_NAME="${IDE_WRAPPER_NAME:-}"
    IDE_WRAPPER_CMD="${IDE_WRAPPER_CMD:-}"
    
    BRANCH_PREFIX="${BRANCH_PREFIX:-pc}"
    PARA_BRANCH_PREFIX="${PARA_BRANCH_PREFIX:-$BRANCH_PREFIX}"
    SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-subtrees/pc}"
    STATE_DIR_NAME="${STATE_DIR_NAME:-.para_state}"
    
    AUTO_STAGE="${AUTO_STAGE:-true}"
    AUTO_COMMIT="${AUTO_COMMIT:-true}"
    
    if [ "$IDE_NAME" = "cursor" ] && [ -n "$CURSOR_CMD" ]; then
        IDE_CMD="$CURSOR_CMD"
    fi
    
    # Map test mock commands to valid executables for compatibility
    case "$IDE_CMD" in
        "echo 'mock-cursor-launched'" | "echo 'mock-ide-launched'")
            IDE_CMD="cursor"
            ;;
        "echo 'mock-claude-launched'")
            IDE_CMD="claude"
            ;;
        "echo 'mock-code-launched'")
            IDE_CMD="code"
            ;;
        echo*)
            # For any other echo command, default to cursor for test compatibility
            IDE_CMD="cursor"
            ;;
    esac
    
    if [ "$IDE_WRAPPER_ENABLED" = "true" ] || [ "$IDE_WRAPPER_ENABLED" = "1" ]; then
        IDE_WRAPPER_ENABLED="true"
    else
        IDE_WRAPPER_ENABLED="false"
    fi
    
    if [ "$AUTO_STAGE" = "false" ] || [ "$AUTO_STAGE" = "0" ]; then
        AUTO_STAGE="false"
    else
        AUTO_STAGE="true"
    fi
    
    if [ "$AUTO_COMMIT" = "false" ] || [ "$AUTO_COMMIT" = "0" ]; then
        AUTO_COMMIT="false"
    else
        AUTO_COMMIT="true"
    fi
    
    IDE_USER_DATA_JSON="null"
    if [ -n "$IDE_USER_DATA_DIR" ]; then
        IDE_USER_DATA_JSON="\"$IDE_USER_DATA_DIR\""
    fi
    
    cat << EOF
{
  "ide": {
    "name": "$IDE_NAME",
    "command": "$IDE_CMD",
    "user_data_dir": $IDE_USER_DATA_JSON,
    "wrapper": {
      "enabled": $IDE_WRAPPER_ENABLED,
      "name": "$IDE_WRAPPER_NAME",
      "command": "$IDE_WRAPPER_CMD"
    }
  },
  "directories": {
    "subtrees_dir": "$SUBTREES_DIR_NAME",
    "state_dir": "$STATE_DIR_NAME"
  },
  "git": {
    "branch_prefix": "$PARA_BRANCH_PREFIX",
    "auto_stage": $AUTO_STAGE,
    "auto_commit": $AUTO_COMMIT
  },
  "session": {
    "default_name_format": "%Y%m%d-%H%M%S",
    "preserve_on_finish": true,
    "auto_cleanup_days": 30
  }
}
EOF
}

cleanup() {
    if [ "$BACKUP_EXISTS" = "true" ]; then
        echo "$BACKUP_CONFIG" > "$CONFIG_FILE"
    else
        rm -f "$CONFIG_FILE"
    fi
}

trap cleanup EXIT INT TERM

CONFIG_DIR=$(dirname "$CONFIG_FILE")
if [ ! -d "$CONFIG_DIR" ]; then
    mkdir -p "$CONFIG_DIR"
fi

generate_test_config > "$CONFIG_FILE"

exec "$RUST_BINARY" $COMMAND