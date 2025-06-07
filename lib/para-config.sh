#!/usr/bin/env sh
# Configuration and environment setup for para

# Default configuration values
DEFAULT_BASE_BRANCH=""
DEFAULT_SUBTREES_DIR_NAME="subtrees"
DEFAULT_STATE_DIR_NAME=".para_state"
DEFAULT_BRANCH_PREFIX="para"
DEFAULT_IDE_NAME="cursor"
DEFAULT_IDE_CMD="cursor"
export DEFAULT_IDE_USER_DATA_DIR=".cursor-userdata"
DEFAULT_IDE_WRAPPER_ENABLED="false"
DEFAULT_IDE_WRAPPER_NAME="code"
DEFAULT_IDE_WRAPPER_CMD="code"

# Configuration file paths
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/para"
CONFIG_FILE="$CONFIG_DIR/config"

# Load configuration from home directory config file
load_home_config() {
  if [ -f "$CONFIG_FILE" ]; then
    # shellcheck disable=SC1090
    . "$CONFIG_FILE"
  fi
}

# Create default configuration file
create_default_config() {
  mkdir -p "$CONFIG_DIR"
  cat >"$CONFIG_FILE" <<'EOF'
# Para Configuration File
# Simple settings for your para IDE workflow

# IDE settings
IDE_NAME="cursor"
IDE_CMD="cursor"
IDE_USER_DATA_DIR=".cursor-userdata"

# IDE Wrapper settings - allows opening Claude Code inside another IDE
# Set to "true" to enable launching Claude Code inside a wrapper IDE
IDE_WRAPPER_ENABLED="false"
# The wrapper IDE to use (e.g., "code", "cursor")
IDE_WRAPPER_NAME="code"
# Command to launch the wrapper IDE
IDE_WRAPPER_CMD="code"

# Directory settings (usually don't need to change these)
SUBTREES_DIR_NAME="subtrees"
STATE_DIR_NAME=".para_state"
BASE_BRANCH=""

# Branch naming settings
BRANCH_PREFIX="para"
EOF
}

# Check if this is the first run (no config exists)
is_first_run() {
  [ ! -f "$CONFIG_FILE" ]
}

# Validate IDE name
validate_ide_name() {
  ide_name="$1"
  case "$ide_name" in
  cursor | claude | code) return 0 ;;
  *)
    if [ -n "$ide_name" ]; then
      return 0 # Allow custom IDE names
    else
      return 1 # Empty IDE name is invalid
    fi
    ;;
  esac
}

# Validate configuration values (simplified)
validate_config() {
  # Only check the essential things
  if [ -z "$IDE_NAME" ] || [ -z "$IDE_CMD" ]; then
    echo "Error: IDE configuration is incomplete: IDE_NAME and IDE_CMD cannot be empty. Run 'para config' to fix." >&2
    return 1
  fi

  # Basic safety checks for directory names
  case "$SUBTREES_DIR_NAME$STATE_DIR_NAME" in
  */* | *\\*)
    echo "Error: Directory names cannot contain path separators. Run 'para config' to fix." >&2
    return 1
    ;;
  esac

  return 0
}

# Check for first run and prompt configuration
check_first_run() {
  if is_first_run; then
    echo "👋 Welcome to para!"
    echo ""

    # Check if running in non-interactive mode (CI environment)
    if [ "${PARA_NON_INTERACTIVE:-false}" = "true" ] || [ -n "${CI:-}" ] || [ -n "${GITHUB_ACTIONS:-}" ]; then
      echo "Running in non-interactive mode, using default configuration."
      create_default_config
      load_config
      return
    fi

    printf "Quick setup your IDE? [Y/n]: "
    read -r setup_choice
    case "$setup_choice" in
    n | N | no | No)
      echo "Skipped setup. Run 'para config' anytime to configure."
      create_default_config
      load_config
      ;;
    *)
      auto_setup
      load_config
      echo ""
      ;;
    esac
  fi
}

# Get IDE-specific default user data directory
get_default_user_data_dir() {
  ide_name="$1"
  case "$ide_name" in
  cursor) echo ".cursor-userdata" ;;
  code) echo ".vscode-userdata" ;;
  claude) echo "" ;; # Claude doesn't support user data dir
  *) echo ".${ide_name}-userdata" ;;
  esac
}

# Load configuration from environment or use defaults
load_config() {
  # Store any existing EXTERNAL environment variables before loading config file
  # Only capture variables that were set externally, not by para itself
  if [ -z "${_PARA_CONFIG_LOADED:-}" ]; then
    # First time loading - capture any external environment variables
    ENV_IDE_NAME="${IDE_NAME:-}"
    ENV_IDE_CMD="${IDE_CMD:-}"
    ENV_IDE_USER_DATA_DIR="${IDE_USER_DATA_DIR:-}"
    ENV_BASE_BRANCH="${BASE_BRANCH:-}"
    ENV_SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-}"
    ENV_STATE_DIR_NAME="${STATE_DIR_NAME:-}"
    ENV_BRANCH_PREFIX="${BRANCH_PREFIX:-}"
    _PARA_CONFIG_LOADED=1
  fi
  # On subsequent loads, don't re-capture ENV_* variables

  # First load from home directory config
  load_home_config

  # Validate what was loaded from file before applying defaults
  if [ -f "$CONFIG_FILE" ]; then
    if [ -z "${IDE_NAME:-}" ] || [ -z "${IDE_CMD:-}" ]; then
      echo "Error: IDE configuration is incomplete: IDE_NAME and IDE_CMD cannot be empty. Run 'para config' to fix." >&2
      return 1
    fi
  fi

  # Then apply environment overrides or defaults
  BASE_BRANCH="${ENV_BASE_BRANCH:-${BASE_BRANCH:-$DEFAULT_BASE_BRANCH}}"
  SUBTREES_DIR_NAME="${ENV_SUBTREES_DIR_NAME:-${SUBTREES_DIR_NAME:-$DEFAULT_SUBTREES_DIR_NAME}}"
  STATE_DIR_NAME="${ENV_STATE_DIR_NAME:-${STATE_DIR_NAME:-$DEFAULT_STATE_DIR_NAME}}"
  BRANCH_PREFIX="${ENV_BRANCH_PREFIX:-${PARA_BRANCH_PREFIX:-${BRANCH_PREFIX:-$DEFAULT_BRANCH_PREFIX}}}"

  # IDE configuration - environment takes priority, then backwards compatibility, then config, then defaults
  IDE_NAME="${ENV_IDE_NAME:-${CURSOR_IDE:-${IDE_NAME:-$DEFAULT_IDE_NAME}}}"
  IDE_CMD="${ENV_IDE_CMD:-${CURSOR_CMD:-${IDE_CMD:-$DEFAULT_IDE_CMD}}}"

  # IDE Wrapper configuration
  IDE_WRAPPER_ENABLED="${ENV_IDE_WRAPPER_ENABLED:-${IDE_WRAPPER_ENABLED:-$DEFAULT_IDE_WRAPPER_ENABLED}}"
  IDE_WRAPPER_NAME="${ENV_IDE_WRAPPER_NAME:-${IDE_WRAPPER_NAME:-$DEFAULT_IDE_WRAPPER_NAME}}"
  IDE_WRAPPER_CMD="${ENV_IDE_WRAPPER_CMD:-${IDE_WRAPPER_CMD:-$DEFAULT_IDE_WRAPPER_CMD}}"

  # User data directory is IDE-specific - some IDEs don't support it
  case "$IDE_NAME" in
  cursor | code)
    # Cursor and VS Code support user data directory isolation
    IDE_USER_DATA_DIR="${ENV_IDE_USER_DATA_DIR:-${CURSOR_USER_DATA_DIR:-${IDE_USER_DATA_DIR:-$(get_default_user_data_dir "$IDE_NAME")}}}"
    ;;
  claude)
    # Claude Code doesn't support --user-data-dir, so don't set it by default
    IDE_USER_DATA_DIR="${ENV_IDE_USER_DATA_DIR:-${IDE_USER_DATA_DIR:-}}"
    ;;
  *)
    # For unknown IDEs, use a sensible default or let user set it
    IDE_USER_DATA_DIR="${ENV_IDE_USER_DATA_DIR:-${CURSOR_USER_DATA_DIR:-${IDE_USER_DATA_DIR:-$(get_default_user_data_dir "$IDE_NAME")}}}"
    ;;
  esac

  # Set CURSOR_* variables for backwards compatibility in functions that still use them
  CURSOR_CMD="$IDE_CMD"
  CURSOR_USER_DATA_DIR="$IDE_USER_DATA_DIR"

  # Final validation after everything is loaded
  validate_config
}

# Save configuration to file (simplified)
save_config() {
  mkdir -p "$CONFIG_DIR"
  cat >"$CONFIG_FILE" <<EOF
# Para Configuration File
# Simple settings for your para IDE workflow

# IDE settings
IDE_NAME="$IDE_NAME"
IDE_CMD="$IDE_CMD"
IDE_USER_DATA_DIR="$IDE_USER_DATA_DIR"

# IDE Wrapper settings - allows opening Claude Code inside another IDE
# Set to "true" to enable launching Claude Code inside a wrapper IDE
IDE_WRAPPER_ENABLED="$IDE_WRAPPER_ENABLED"
# The wrapper IDE to use (e.g., "code", "cursor")
IDE_WRAPPER_NAME="$IDE_WRAPPER_NAME"
# Command to launch the wrapper IDE
IDE_WRAPPER_CMD="$IDE_WRAPPER_CMD"

# Directory settings (usually don't need to change these)
SUBTREES_DIR_NAME="$SUBTREES_DIR_NAME"
STATE_DIR_NAME="$STATE_DIR_NAME"
BASE_BRANCH="$BASE_BRANCH"

# Branch naming settings
BRANCH_PREFIX="$BRANCH_PREFIX"
EOF
}

# Initialize directory paths based on repository root
init_paths() {
  export STATE_DIR="$REPO_ROOT/$STATE_DIR_NAME"
  export SUBTREES_DIR="$REPO_ROOT/$SUBTREES_DIR_NAME"
  # Use IDE-specific template directory
  IDE_TEMPLATE_NAME=$(echo "$IDE_NAME" | tr '[:upper:]' '[:lower:]')
  export TEMPLATE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/para/${IDE_TEMPLATE_NAME}-template"
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

# Show current configuration (simplified)
show_config() {
  echo "Para Configuration"
  echo "=================="
  echo ""
  echo "IDE: $(get_ide_display_name)"
  echo "Command: $IDE_CMD"
  if [ -n "$IDE_USER_DATA_DIR" ]; then
    echo "User data: $IDE_USER_DATA_DIR"
  fi
  if [ "$IDE_WRAPPER_ENABLED" = "true" ]; then
    echo ""
    echo "IDE Wrapper: Enabled"
    echo "Wrapper IDE: $IDE_WRAPPER_NAME"
    echo "Wrapper command: $IDE_WRAPPER_CMD"
  fi
  echo ""
  echo "Subtrees directory name: $SUBTREES_DIR_NAME"
  echo "State directory name: $STATE_DIR_NAME"
  echo "Base branch: $BASE_BRANCH"
  echo "Branch prefix: $BRANCH_PREFIX"
  echo ""
  echo "Config file: $CONFIG_FILE"
  echo ""
  echo "Run 'para config' to change these settings."
}

# Handle config command
handle_config_command() {
  if [ "$#" -eq 1 ]; then
    # No subcommand - run simple setup
    run_config_setup
  else
    case "$2" in
    show)
      show_config
      ;;
    auto)
      auto_setup
      ;;
    quick)
      # Quick setup with user confirmation
      printf "Quick Setup your IDE? [Y/n]: "
      read -r setup_choice
      case "$setup_choice" in
      n | N | no | No)
        # Cancel quick setup
        return 1
        ;;
      *)
        auto_setup
        ;;
      esac
      ;;
    wizard)
      # Alias for interactive setup wizard
      run_config_setup
      ;;
    edit)
      if [ -f "$CONFIG_FILE" ]; then
        # Use robust eval for multi-word EDITOR commands
        cmd="${EDITOR:-vi}"
        eval "$cmd \"$CONFIG_FILE\""
      else
        echo "No config file found. Run 'para config' to create one."
      fi
      ;;
    *)
      # Handle unknown subcommands
      echo "Unknown config command: $2"
      echo "Usage: para config [show|auto|quick|wizard|edit]"
      echo ""
      echo "  para config         # Interactive setup"
      echo "  para config show    # Show current settings"
      echo "  para config auto    # Auto-detect IDE"
      echo "  para config quick   # Quick auto-detect with confirmation"
      echo "  para config wizard  # Interactive setup wizard"
      echo "  para config edit    # Edit config file"
      ;;
    esac
  fi
}
