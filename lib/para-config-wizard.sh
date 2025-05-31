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
    2|claude) 
      IDE_NAME="claude"
      IDE_CMD="claude"
      IDE_USER_DATA_DIR=""
      ;;
    3|code|vscode)
      IDE_NAME="code"
      IDE_CMD="code" 
      IDE_USER_DATA_DIR=".vscode-userdata"
      ;;
    4|other)
      printf "Enter IDE command: "
      read -r custom_cmd
      if [ -n "$custom_cmd" ]; then
        IDE_NAME="$custom_cmd"
        IDE_CMD="$custom_cmd"
        IDE_USER_DATA_DIR=".${custom_cmd}-userdata"
      else
        echo "Invalid input. Using Cursor as default."
        IDE_NAME="cursor"
        IDE_CMD="cursor"
        IDE_USER_DATA_DIR=".cursor-userdata"
      fi
      ;;
    *)
      # Default to Cursor
      IDE_NAME="cursor"
      IDE_CMD="cursor"
      IDE_USER_DATA_DIR=".cursor-userdata"
      ;;
  esac
  
  # Use sensible defaults for everything else
  SUBTREES_DIR_NAME="subtrees"
  STATE_DIR_NAME=".para_state"
  BASE_BRANCH=""
  
  echo ""
  echo "✅ Configuration set:"
  echo "  IDE: $(get_ide_display_name)"
  echo "  Command: $IDE_CMD"
  echo ""
  
  save_config
  echo "💾 Saved to $CONFIG_FILE"
  echo ""
  echo "You're all set! Run 'para start' to create your first session."
}

# Show brief current config
show_current_config_brief() {
  echo "  IDE: $(get_ide_display_name) ($IDE_NAME)"
  echo "  Command: $IDE_CMD"
}

# Auto-detect IDE and create quick config
auto_setup() {
  echo "🔍 Auto-detecting your IDE..."
  
  if command -v cursor >/dev/null 2>&1; then
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    IDE_USER_DATA_DIR=".cursor-userdata"
    echo "✅ Found Cursor"
  elif command -v claude >/dev/null 2>&1; then
    IDE_NAME="claude"
    IDE_CMD="claude"
    IDE_USER_DATA_DIR=""
    echo "✅ Found Claude Code"
  elif command -v code >/dev/null 2>&1; then
    IDE_NAME="code"
    IDE_CMD="code"
    IDE_USER_DATA_DIR=".vscode-userdata"
    echo "✅ Found VS Code"
  else
    echo "⚠️  No IDE found. Using Cursor as default."
    IDE_NAME="cursor"
    IDE_CMD="cursor"
    IDE_USER_DATA_DIR=".cursor-userdata"
  fi
  
  SUBTREES_DIR_NAME="subtrees"
  STATE_DIR_NAME=".para_state"
  BASE_BRANCH=""
  
  save_config
  echo "✅ Auto-configuration complete!"
  echo "  Using: $(get_ide_display_name)"
} 