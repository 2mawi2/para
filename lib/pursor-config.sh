#!/usr/bin/env sh
# Configuration and environment setup for pursor

# Default configuration values
DEFAULT_BASE_BRANCH=""
DEFAULT_SUBTREES_DIR_NAME="subtrees"
DEFAULT_STATE_DIR_NAME=".pursor_state"
DEFAULT_CURSOR_CMD="cursor"
DEFAULT_CURSOR_USER_DATA_DIR=".cursor-userdata"

# Load configuration from environment or use defaults
load_config() {
  BASE_BRANCH="${BASE_BRANCH:-$DEFAULT_BASE_BRANCH}"
  SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-$DEFAULT_SUBTREES_DIR_NAME}"
  STATE_DIR_NAME="${STATE_DIR_NAME:-$DEFAULT_STATE_DIR_NAME}"
  CURSOR_CMD="${CURSOR_CMD:-$DEFAULT_CURSOR_CMD}"
  CURSOR_USER_DATA_DIR="${CURSOR_USER_DATA_DIR:-$DEFAULT_CURSOR_USER_DATA_DIR}"
}

# Initialize directory paths based on repository root
init_paths() {
  STATE_DIR="$REPO_ROOT/$STATE_DIR_NAME"
  SUBTREES_DIR="$REPO_ROOT/$SUBTREES_DIR_NAME"
  # Use user-global location for cursor template (XDG-compliant)
  TEMPLATE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/pursor/cursor-template"
} 