#!/usr/bin/env sh
# Configuration and environment setup for para

# Default configuration values
DEFAULT_BASE_BRANCH=""
DEFAULT_SUBTREES_DIR_NAME="subtrees"
DEFAULT_STATE_DIR_NAME=".para_state"
DEFAULT_IDE_NAME="cursor"
DEFAULT_IDE_CMD="cursor"
DEFAULT_IDE_USER_DATA_DIR=".cursor-userdata"

# Load configuration from environment or use defaults
load_config() {
  BASE_BRANCH="${BASE_BRANCH:-$DEFAULT_BASE_BRANCH}"
  SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-$DEFAULT_SUBTREES_DIR_NAME}"
  STATE_DIR_NAME="${STATE_DIR_NAME:-$DEFAULT_STATE_DIR_NAME}"
  
  # IDE configuration - backwards compatible with CURSOR_* variables
  IDE_NAME="${IDE_NAME:-${CURSOR_IDE:-$DEFAULT_IDE_NAME}}"
  IDE_CMD="${IDE_CMD:-${CURSOR_CMD:-$DEFAULT_IDE_CMD}}"
  
  # User data directory is IDE-specific - some IDEs don't support it
  case "$IDE_NAME" in
    cursor|code)
      # Cursor and VS Code support user data directory isolation
      IDE_USER_DATA_DIR="${IDE_USER_DATA_DIR:-${CURSOR_USER_DATA_DIR:-$DEFAULT_IDE_USER_DATA_DIR}}"
      ;;
    claude)
      # Claude Code doesn't support --user-data-dir, so don't set it by default
      IDE_USER_DATA_DIR="${IDE_USER_DATA_DIR:-}"
      ;;
    *)
      # For unknown IDEs, let user explicitly set it if needed
      IDE_USER_DATA_DIR="${IDE_USER_DATA_DIR:-${CURSOR_USER_DATA_DIR:-}}"
      ;;
  esac
  
  # Set CURSOR_* variables for backwards compatibility in functions that still use them
  CURSOR_CMD="$IDE_CMD"
  CURSOR_USER_DATA_DIR="$IDE_USER_DATA_DIR"
}

# Initialize directory paths based on repository root
init_paths() {
  STATE_DIR="$REPO_ROOT/$STATE_DIR_NAME"
  SUBTREES_DIR="$REPO_ROOT/$SUBTREES_DIR_NAME"
  # Use IDE-specific template directory
  IDE_TEMPLATE_NAME=$(echo "$IDE_NAME" | tr '[:upper:]' '[:lower:]')
  TEMPLATE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/para/${IDE_TEMPLATE_NAME}-template"
}

# Get the display name for the current IDE
get_ide_display_name() {
  case "$IDE_NAME" in
    cursor) echo "Cursor" ;;
    claude) echo "Claude Code" ;;
    code) echo "VS Code" ;;
    *) echo "$IDE_NAME" ;;
  esac
} 