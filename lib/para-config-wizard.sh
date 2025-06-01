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

    # Configure how Claude Code should be launched
    echo ""
    echo "How would you like to run Claude Code?"
    echo ""
    echo "🖥️  IDE Integration (Recommended):"
    echo "1) Inside VS Code (auto-starts in integrated terminal)"
    echo "2) Inside Cursor (auto-starts in integrated terminal)"
    echo ""
    echo "🖥️  Terminal Options:"
    echo "3) Auto-detect terminal (recommended)"
    echo "4) Terminal.app (macOS default)"
    echo "5) Warp"
    echo "6) Ghostty"
    echo "7) iTerm2"
    echo "8) Custom terminal command"
    echo ""
    printf "Choose option [1]: "
    read -r claude_mode_choice

    case "$claude_mode_choice" in
    1 | vscode | code)
      # IDE Wrapper: VS Code
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="auto" # Not used in wrapper mode, but set for consistency
      echo "  Mode: IDE Wrapper (VS Code)"
      ;;
    2 | cursor)
      # IDE Wrapper: Cursor
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="cursor"
      export IDE_WRAPPER_CMD="cursor"
      export CLAUDE_TERMINAL_CMD="auto" # Not used in wrapper mode, but set for consistency
      echo "  Mode: IDE Wrapper (Cursor)"
      ;;
    3 | auto)
      # Terminal mode: Auto-detect
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="auto"
      echo "  Mode: Terminal (auto-detect)"
      ;;
    4 | terminal)
      # Terminal mode: Terminal.app
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="terminal"
      echo "  Mode: Terminal (Terminal.app)"
      ;;
    5 | warp)
      # Terminal mode: Warp
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="warp"
      echo "  Mode: Terminal (Warp)"
      ;;
    6 | ghostty)
      # Terminal mode: Ghostty
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="ghostty"
      echo "  Mode: Terminal (Ghostty)"
      ;;
    7 | iterm | iterm2)
      # Terminal mode: iTerm2
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="iterm2"
      echo "  Mode: Terminal (iTerm2)"
      ;;
    8 | custom)
      # Terminal mode: Custom
      export IDE_WRAPPER_ENABLED="false"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      printf "Enter custom terminal command (use %%d for directory, %%c for command): "
      read -r custom_terminal
      if [ -n "$custom_terminal" ]; then
        export CLAUDE_TERMINAL_CMD="$custom_terminal"
        echo "  Mode: Terminal (custom)"
      else
        echo "Invalid input. Using auto-detect."
        export CLAUDE_TERMINAL_CMD="auto"
        echo "  Mode: Terminal (auto-detect)"
      fi
      ;;
    *)
      # Default to VS Code wrapper
      export IDE_WRAPPER_ENABLED="true"
      export IDE_WRAPPER_NAME="code"
      export IDE_WRAPPER_CMD="code"
      export CLAUDE_TERMINAL_CMD="auto"
      echo "  Mode: IDE Wrapper (VS Code - default)"
      ;;
    esac
    ;;
  3 | code | vscode)
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    export CLAUDE_TERMINAL_CMD="auto"  # Set default for consistency
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
    export CLAUDE_TERMINAL_CMD="auto"  # Set default for consistency
    export IDE_WRAPPER_ENABLED="false" # Set default for non-Claude IDEs
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    ;;
  *)
    # Default to Cursor
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    export CLAUDE_TERMINAL_CMD="auto"  # Set default for consistency
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
    else
      echo "  Terminal: $CLAUDE_TERMINAL_CMD"
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
    else
      echo "  Terminal: $CLAUDE_TERMINAL_CMD"
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
    export CLAUDE_TERMINAL_CMD="auto"
    export IDE_WRAPPER_ENABLED="false"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found Cursor"
  elif command -v claude >/dev/null 2>&1; then
    IDE_NAME="claude"
    IDE_CMD="claude"
    export IDE_USER_DATA_DIR=""
    export CLAUDE_TERMINAL_CMD="auto"
    # Default to VS Code wrapper for Claude Code
    export IDE_WRAPPER_ENABLED="true"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found Claude Code (configured with VS Code wrapper)"
  elif command -v code >/dev/null 2>&1; then
    IDE_NAME="code"
    IDE_CMD="code"
    export IDE_USER_DATA_DIR=".vscode-userdata"
    export CLAUDE_TERMINAL_CMD="auto"
    export IDE_WRAPPER_ENABLED="false"
    export IDE_WRAPPER_NAME="code"
    export IDE_WRAPPER_CMD="code"
    echo "✅ Found VS Code"
  else
    echo "⚠️  No IDE found. Using Cursor as default."
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    export IDE_USER_DATA_DIR=".cursor-userdata"
    export CLAUDE_TERMINAL_CMD="auto"
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
