#!/usr/bin/env sh
# Configuration and environment setup for pursor

# Default configuration values
DEFAULT_BASE_BRANCH=""
DEFAULT_SUBTREES_DIR_NAME="subtrees"
DEFAULT_STATE_DIR_NAME=".pursor_state"
DEFAULT_CURSOR_CMD="cursor"

# Load configuration from environment or use defaults
load_config() {
  BASE_BRANCH="${BASE_BRANCH:-$DEFAULT_BASE_BRANCH}"
  SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-$DEFAULT_SUBTREES_DIR_NAME}"
  STATE_DIR_NAME="${STATE_DIR_NAME:-$DEFAULT_STATE_DIR_NAME}"
  CURSOR_CMD="${CURSOR_CMD:-$DEFAULT_CURSOR_CMD}"
}

# Initialize directory paths based on repository root
init_paths() {
  STATE_DIR="$REPO_ROOT/$STATE_DIR_NAME"
  SUBTREES_DIR="$REPO_ROOT/$SUBTREES_DIR_NAME"
} 