#!/usr/bin/env sh
# Simple configuration for para

# Simple configuration setup
run_config_setup() {
  echo "🔧 Para Configuration"
  echo "===================="
  echo ""

  if [ -f "$CONFIG_FILE" ]; then
    echo "Current settings:"
    show_current_config_brief
    echo ""
  fi

  echo "Let's set up your IDE preference:"
  echo ""
  echo "1) Cursor (default)"
  echo "2) Claude Code"
  echo "3) VS Code"
  echo "4) Other (specify command)"
  echo ""
  printf "Choose your IDE [1]: "
  read -r ide_choice

  case "$ide_choice" in
  2 | claude)
    # Preserve existing IDE_CMD if we're already using Claude, otherwise use default
    if [ "${IDE_NAME:-}" != "claude" ] || [ -z "${IDE_CMD:-}" ]; then
      IDE_CMD="claude"
    fi
    IDE_NAME="claude"
    export IDE_USER_DATA_DIR=""

    # Configure how Claude Code should be launched
    echo ""
    echo "🖥️  IDE Integration (Required for Claude Code):"
    echo "1) Inside VS Code (auto-starts in integrated terminal)"
    echo "2) Inside Cursor (auto-starts in integrated terminal)"
    echo "3) Back to main menu"
    echo ""
    printf "Choose option [1]: "
    read -r claude_mode_choice

    case "$claude_mode_choice" in
    1 | vscode | code)
      # IDE Wrapper: VS Code
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      echo "  Mode: IDE Wrapper (VS Code)"
      ;;
    2 | cursor)
      # IDE Wrapper: Cursor
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="cursor"
      export IDE_WRAPPER_CMD="cursor"
      echo "  Mode: IDE Wrapper (Cursor)"
      ;;
    3 | back)
      # Go back to main menu
      return
      ;;
    *)
      # Default to VS Code wrapper
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      echo "  Mode: IDE Wrapper (VS Code - default)"
      ;;
    esac
    ;;
  3 | code | vscode)
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    export IDE_WRAPPER_ENABLED="false" # Set default for non-Claude IDEs
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    ;;
  4 | other)
    printf "Enter IDE command: "
    read -r custom_cmd
    if [ -n "$custom_cmd" ]; then
      IDE_NAME="$custom_cmd"
      IDE_CMD="$custom_cmd"
      export IDE_USER_DATA_DIR=".${custom_cmd}-userdata"
    else
      echo "Invalid input. Using Cursor as default."
      IDE_NAME="cursor"
      IDE_CMD="cursor"
      export IDE_USER_DATA_DIR=".cursor-userdata"
    fi
    export IDE_WRAPPER_ENABLED="false" # Set default for non-Claude IDEs
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    ;;
  *)
    # Default to Cursor
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    export IDE_WRAPPER_ENABLED="false" # Set default for non-Claude IDEs
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    ;;
  esac

  # Use sensible defaults for everything else
  export SUBTREES_DIR_NAME="subtrees"
  export STATE_DIR_NAME=".para_state"
  export BASE_BRANCH=""

  echo ""
  echo "✅ Configuration set:"
  echo "  IDE: $(get_ide_display_name)"
  echo "  Command: $IDE_CMD"
  if [ "$IDE_NAME" = "claude" ]; then
    if [ "$IDE_WRAPPER_ENABLED" = "true" ]; then
      echo "  Mode: IDE Wrapper ($IDE_WRAPPER_NAME)"
    fi
  fi
  echo ""

  save_config
  load_config # Reload configuration into memory after saving
  echo "💾 Saved to $CONFIG_FILE"
  echo ""
  echo "You're all set! Run 'para start' to create your first session."
}

# Show brief current config
show_current_config_brief() {
  echo "  IDE: $(get_ide_display_name) ($IDE_NAME)"
  echo "  Command: $IDE_CMD"
  if [ "$IDE_NAME" = "claude" ]; then
    if [ "$IDE_WRAPPER_ENABLED" = "true" ]; then
      echo "  Mode: IDE Wrapper ($IDE_WRAPPER_NAME)"
    fi
  fi
}

# Auto-detect IDE and create quick config
auto_setup() {
  echo "🔍 Auto-detecting your IDE..."

  if command -v cursor >/dev/null 2>&1; then
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    export IDE_WRAPPER_ENABLED="false"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found Cursor"
  elif command -v claude >/dev/null 2>&1; then
    IDE_NAME="claude"
    IDE_CMD="claude"
    export IDE_USER_DATA_DIR=""
    export IDE_WRAPPER_ENABLED="true"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found Claude Code (configured with VS Code wrapper)"
  elif command -v code >/dev/null 2>&1; then
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    export IDE_WRAPPER_ENABLED="false"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found VS Code"
  else
    echo "⚠️  No IDE found. Using Cursor as default."
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    export IDE_WRAPPER_ENABLED="false"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
  fi

  export SUBTREES_DIR_NAME="subtrees"
  export STATE_DIR_NAME=".para_state"
  export BASE_BRANCH=""

  save_config
  load_config # Reload configuration into memory after saving
  echo "✅ Auto-configuration complete!"
  echo "  Using: $(get_ide_display_name)"
}
