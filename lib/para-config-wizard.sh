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
    IDE_NAME="claude"
    IDE_CMD="claude"
    export IDE_USER_DATA_DIR=""

    # Configure terminal for Claude Code
    echo ""
    echo "Claude Code runs in a terminal. Choose your terminal:"
    echo ""
    echo "1) Auto-detect (recommended)"
    echo "2) Terminal.app (macOS default)"
    echo "3) Warp"
    echo "4) Ghostty"
    echo "5) iTerm2"
    echo "6) Custom terminal command"
    echo ""
    printf "Choose terminal [1]: "
    read -r terminal_choice

    case "$terminal_choice" in
    2 | terminal)
      CLAUDE_TERMINAL_CMD="terminal"
      ;;
    3 | warp)
      CLAUDE_TERMINAL_CMD="warp"
      ;;
    4 | ghostty)
      CLAUDE_TERMINAL_CMD="ghostty"
      ;;
    5 | iterm | iterm2)
      CLAUDE_TERMINAL_CMD="iterm2"
      ;;
    6 | custom)
      printf "Enter custom terminal command (use %%d for directory, %%c for command): "
      read -r custom_terminal
      if [ -n "$custom_terminal" ]; then
        CLAUDE_TERMINAL_CMD="$custom_terminal"
      else
        echo "Invalid input. Using auto-detect."
        CLAUDE_TERMINAL_CMD="auto"
      fi
      ;;
    *)
      # Default to auto-detect
      CLAUDE_TERMINAL_CMD="auto"
      ;;
    esac
    ;;
  3 | code | vscode)
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    CLAUDE_TERMINAL_CMD="auto" # Set default for consistency
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
    CLAUDE_TERMINAL_CMD="auto" # Set default for consistency
    ;;
  *)
    # Default to Cursor
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    CLAUDE_TERMINAL_CMD="auto" # Set default for consistency
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
    echo "  Terminal: $CLAUDE_TERMINAL_CMD"
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
    echo "  Terminal: $CLAUDE_TERMINAL_CMD"
  fi
}

# Auto-detect IDE and create quick config
auto_setup() {
  echo "🔍 Auto-detecting your IDE..."

  if command -v cursor >/dev/null 2>&1; then
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    CLAUDE_TERMINAL_CMD="auto"
    echo "✅ Found Cursor"
  elif command -v claude >/dev/null 2>&1; then
    IDE_NAME="claude"
    IDE_CMD="claude"
    export IDE_USER_DATA_DIR=""
    CLAUDE_TERMINAL_CMD="auto"
    echo "✅ Found Claude Code"
  elif command -v code >/dev/null 2>&1; then
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    CLAUDE_TERMINAL_CMD="auto"
    echo "✅ Found VS Code"
  else
    echo "⚠️  No IDE found. Using Cursor as default."
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    CLAUDE_TERMINAL_CMD="auto"
  fi

  export SUBTREES_DIR_NAME="subtrees"
  export STATE_DIR_NAME=".para_state"
  export BASE_BRANCH=""

  save_config
  load_config # Reload configuration into memory after saving
  echo "✅ Auto-configuration complete!"
  echo "  Using: $(get_ide_display_name)"
}
