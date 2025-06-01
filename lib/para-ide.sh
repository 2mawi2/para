#!/usr/bin/env sh
# IDE integration for para - designed for extensibility

# Abstract IDE interface - to be implemented by specific IDE modules

# Launch IDE for a session
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"

  # Check if IDE wrapper is enabled and we're launching Claude Code
  if [ "$ide_name" = "claude" ] && [ "${IDE_WRAPPER_ENABLED:-false}" = "true" ]; then
    echo "▶ launching Claude Code inside $IDE_WRAPPER_NAME wrapper..."
    launch_ide_with_wrapper "$ide_name" "$worktree_dir"
    return
  fi

  case "$ide_name" in
  cursor)
    launch_cursor "$worktree_dir"
    ;;
  claude)
    launch_claude "$worktree_dir"
    ;;
  code)
    launch_vscode "$worktree_dir"
    ;;
  *)
    die "unsupported IDE: $ide_name"
    ;;
  esac
}

# Launch IDE with wrapper functionality
launch_ide_with_wrapper() {
  ide_name="$1"
  worktree_dir="$2"

  case "$IDE_WRAPPER_NAME" in
  code)
    write_vscode_autorun_task "$worktree_dir"
    launch_vscode_wrapper "$worktree_dir"
    ;;
  cursor)
    write_cursor_autorun_task "$worktree_dir"
    launch_cursor_wrapper "$worktree_dir"
    ;;
  *)
    echo "⚠️  Unsupported wrapper IDE: $IDE_WRAPPER_NAME" >&2
    echo "   Falling back to regular Claude Code launch..." >&2
    launch_claude "$worktree_dir"
    ;;
  esac
}

# Launch VS Code as wrapper for Claude Code
launch_vscode_wrapper() {
  worktree_dir="$1"

  # Skip actual IDE launch in test mode
  if [ "${IDE_WRAPPER_CMD:-}" = "true" ]; then
    echo "▶ skipping VS Code wrapper launch (test stub)"
    echo "✅ VS Code wrapper (test stub) opened with Claude Code auto-start"
    return 0
  fi

  # If IDE_WRAPPER_CMD is a stub echo command, run it instead of launching
  case "$IDE_WRAPPER_CMD" in
  echo\ *)
    eval "$IDE_WRAPPER_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$IDE_WRAPPER_CMD" >/dev/null 2>&1; then
    echo "▶ launching VS Code wrapper with Claude Code auto-start..."
    "$IDE_WRAPPER_CMD" "$worktree_dir" &
    echo "✅ VS Code opened - Claude Code will start automatically"
  else
    echo "⚠️  VS Code wrapper CLI not found. Please install VS Code CLI or set IDE_WRAPPER_CMD environment variable." >&2
    echo "   Falling back to regular Claude Code launch..." >&2
    launch_claude "$worktree_dir"
  fi
}

# Launch Cursor as wrapper for Claude Code
launch_cursor_wrapper() {
  worktree_dir="$1"

  # Skip actual IDE launch in test mode
  if [ "${IDE_WRAPPER_CMD:-}" = "true" ]; then
    echo "▶ skipping Cursor wrapper launch (test stub)"
    echo "✅ Cursor wrapper (test stub) opened with Claude Code auto-start"
    return 0
  fi

  # If IDE_WRAPPER_CMD is a stub echo command, run it instead of launching
  case "$IDE_WRAPPER_CMD" in
  echo\ *)
    eval "$IDE_WRAPPER_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$IDE_WRAPPER_CMD" >/dev/null 2>&1; then
    echo "▶ launching Cursor wrapper with Claude Code auto-start..."
    "$IDE_WRAPPER_CMD" "$worktree_dir" &
    echo "✅ Cursor opened - Claude Code will start automatically"
  else
    echo "⚠️  Cursor wrapper CLI not found. Please install Cursor CLI or set IDE_WRAPPER_CMD environment variable." >&2
    echo "   Falling back to regular Claude Code launch..." >&2
    launch_claude "$worktree_dir"
  fi
}

# Cursor IDE implementation
launch_cursor() {
  worktree_dir="$1"

  # Skip actual IDE launch and template setup in test mode
  if [ "${CURSOR_CMD:-}" = "true" ]; then
    echo "▶ skipping Cursor launch (test stub)"
    echo "✅ Cursor (test stub) opened"
    return 0
  fi

  # If CURSOR_CMD is a stub starting with 'echo ', run it instead of launching
  case "$CURSOR_CMD" in
  echo\ *)
    eval "$CURSOR_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$CURSOR_CMD" >/dev/null 2>&1; then
    if [ -n "${CURSOR_USER_DATA_DIR:-}" ]; then
      # Use a single global para user data directory
      global_user_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/para/cursor-userdata"

      # Setup template on first use
      if ! template_exists; then
        setup_para_template
      fi

      # Create global para user data directory if it doesn't exist
      if [ ! -d "$global_user_data_dir" ]; then
        echo "🔧 Setting up global para user data directory..."
        mkdir -p "$global_user_data_dir"

        # Copy from template if it exists
        if template_exists; then
          echo "📋 Copying settings from para template to global user data..."
          if command -v rsync >/dev/null 2>&1; then
            rsync -a --exclude='*.sock' --exclude='*.lock' \
              --exclude='Local Storage/' --exclude='Session Storage/' \
              --exclude='blob_storage/' --exclude='Shared Dictionary/' \
              "$TEMPLATE_DIR/" "$global_user_data_dir/"
          else
            cp -r "$TEMPLATE_DIR"/* "$global_user_data_dir/" 2>/dev/null || true
            # Remove problematic files
            rm -rf "$global_user_data_dir"/*.sock "$global_user_data_dir"/*.lock \
              "$global_user_data_dir/Local Storage" "$global_user_data_dir/Session Storage" \
              "$global_user_data_dir/blob_storage" "$global_user_data_dir/Shared Dictionary" \
              2>/dev/null || true
          fi
          echo "✅ Global para user data directory ready"
        fi
      fi

      echo "▶ launching Cursor with global para user data directory..."
      "$CURSOR_CMD" "$worktree_dir" --user-data-dir "$global_user_data_dir" &
    else
      echo "▶ launching Cursor..."
      "$CURSOR_CMD" "$worktree_dir" &
    fi
    echo "✅ Cursor opened"
  else
    echo "⚠️  Cursor CLI not found. Please install Cursor CLI or set IDE_CMD environment variable." >&2
    echo "   Alternatively, manually open: $worktree_dir" >&2
    echo "   💡 Install Cursor CLI: https://cursor.sh/cli" >&2
  fi
}

# Claude Code implementation
launch_claude() {
  worktree_dir="$1"

  # Skip actual IDE launch in test mode
  if [ "${IDE_CMD:-}" = "true" ]; then
    echo "▶ skipping Claude Code launch (test stub)"
    echo "✅ Claude Code (test stub) opened"
    return 0
  fi

  # If IDE_CMD is a stub echo command, run it instead of opening a new terminal
  case "$IDE_CMD" in
  echo\ *)
    eval "$IDE_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$IDE_CMD" >/dev/null 2>&1; then
    echo "▶ launching Claude Code in new terminal..."

    # Determine which terminal to use based on CLAUDE_TERMINAL_CMD
    case "$CLAUDE_TERMINAL_CMD" in
    auto)
      # Auto-detect available terminal
      launch_claude_auto_terminal "$worktree_dir"
      ;;
    terminal)
      # Force use of macOS Terminal.app
      launch_claude_terminal_app "$worktree_dir"
      ;;
    warp)
      # Use Warp terminal
      launch_claude_warp "$worktree_dir"
      ;;
    ghostty)
      # Use Ghostty terminal
      launch_claude_ghostty "$worktree_dir"
      ;;
    iterm2)
      # Use iTerm2
      launch_claude_iterm2 "$worktree_dir"
      ;;
    *)
      # Custom terminal command
      launch_claude_custom_terminal "$worktree_dir" "$CLAUDE_TERMINAL_CMD"
      ;;
    esac
  else
    echo "⚠️  Claude Code CLI not found. Please install Claude Code CLI or set IDE_CMD environment variable." >&2
    echo "   Alternatively, manually open: $worktree_dir" >&2
  fi
}

# Auto-detect and use the best available terminal
launch_claude_auto_terminal() {
  worktree_dir="$1"

  if command -v warp-cli >/dev/null 2>&1; then
    launch_claude_warp "$worktree_dir"
  elif [ -d "/Applications/Ghostty.app" ] && command -v ghostty >/dev/null 2>&1; then
    launch_claude_ghostty "$worktree_dir"
  elif [ -d "/Applications/iTerm.app" ]; then
    launch_claude_iterm2 "$worktree_dir"
  elif command -v osascript >/dev/null 2>&1; then
    launch_claude_terminal_app "$worktree_dir"
  else
    # Fallback for non-macOS systems
    launch_claude_fallback "$worktree_dir"
  fi
}

# Launch using macOS Terminal.app
launch_claude_terminal_app() {
  worktree_dir="$1"

  if command -v osascript >/dev/null 2>&1; then
    # Use AppleScript to create a new terminal window and run Claude Code
    osascript <<EOF
tell application "Terminal"
    do script "cd '$worktree_dir' && '$IDE_CMD'"
    activate
end tell
EOF
    echo "✅ Claude Code opened in Terminal.app"
  else
    echo "⚠️  AppleScript not available. Cannot launch Terminal.app" >&2
    launch_claude_fallback "$worktree_dir"
  fi
}

# Launch using Warp terminal
launch_claude_warp() {
  worktree_dir="$1"

  if command -v warp-cli >/dev/null 2>&1; then
    # Use Warp CLI to create a new session
    warp-cli open "$worktree_dir" --exec "$IDE_CMD"
    echo "✅ Claude Code opened in Warp"
  elif [ -d "/Applications/Warp.app" ]; then
    # Fallback to AppleScript for Warp
    if command -v osascript >/dev/null 2>&1; then
      osascript <<EOF
tell application "Warp"
    activate
    tell application "System Events"
        keystroke "t" using {command down}
        delay 0.5
        keystroke "cd '$worktree_dir' && '$IDE_CMD'"
        keystroke return
    end tell
end tell
EOF
      echo "✅ Claude Code opened in Warp"
    else
      echo "⚠️  Warp CLI not found and AppleScript not available" >&2
      launch_claude_fallback "$worktree_dir"
    fi
  else
    echo "⚠️  Warp terminal not found. Please install Warp or use a different terminal." >&2
    launch_claude_fallback "$worktree_dir"
  fi
}

# Launch using Ghostty terminal
launch_claude_ghostty() {
  worktree_dir="$1"

  if command -v ghostty >/dev/null 2>&1; then
    # Use Ghostty CLI to create a new window
    ghostty --working-directory="$worktree_dir" --command="$IDE_CMD" &
    echo "✅ Claude Code opened in Ghostty"
  elif [ -d "/Applications/Ghostty.app" ]; then
    # Fallback to open command
    open -n -a Ghostty.app --args --working-directory="$worktree_dir" --command="$IDE_CMD"
    echo "✅ Claude Code opened in Ghostty"
  else
    echo "⚠️  Ghostty terminal not found. Please install Ghostty or use a different terminal." >&2
    launch_claude_fallback "$worktree_dir"
  fi
}

# Launch using iTerm2
launch_claude_iterm2() {
  worktree_dir="$1"

  if [ -d "/Applications/iTerm.app" ] && command -v osascript >/dev/null 2>&1; then
    # Use AppleScript to create a new iTerm2 window
    osascript <<EOF
tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "cd '$worktree_dir' && '$IDE_CMD'"
    end tell
    activate
end tell
EOF
    echo "✅ Claude Code opened in iTerm2"
  else
    echo "⚠️  iTerm2 not found or AppleScript not available" >&2
    launch_claude_fallback "$worktree_dir"
  fi
}

# Launch using custom terminal command
launch_claude_custom_terminal() {
  worktree_dir="$1"
  custom_cmd="$2"

  # Replace placeholders in the custom command
  # %d = directory, %c = command
  terminal_cmd=$(echo "$custom_cmd" | sed "s|%d|$worktree_dir|g" | sed "s|%c|$IDE_CMD|g")

  if eval "$terminal_cmd"; then
    echo "✅ Claude Code opened in custom terminal"
  else
    echo "⚠️  Failed to launch with custom terminal command: $custom_cmd" >&2
    launch_claude_fallback "$worktree_dir"
  fi
}

# Fallback terminal launch for non-macOS systems or when other methods fail
launch_claude_fallback() {
  worktree_dir="$1"

  # Try common terminal emulators
  if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --working-directory="$worktree_dir" -- "$IDE_CMD"
    echo "✅ Claude Code opened in gnome-terminal"
  elif command -v xterm >/dev/null 2>&1; then
    (cd "$worktree_dir" && xterm -e "$IDE_CMD") &
    echo "✅ Claude Code opened in xterm"
  elif command -v konsole >/dev/null 2>&1; then
    konsole --workdir "$worktree_dir" -e "$IDE_CMD" &
    echo "✅ Claude Code opened in konsole"
  else
    echo "⚠️  Could not detect terminal emulator. Running in current terminal..."
    cd "$worktree_dir" && "$IDE_CMD"
    echo "✅ Claude Code session ended"
  fi
}

# Write VS Code task configuration for auto-running Claude Code
write_vscode_autorun_task() {
  worktree_dir="$1"
  mkdir -p "$worktree_dir/.vscode"
  cat >"$worktree_dir/.vscode/tasks.json" <<'EOF'
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "claude",
      "type": "shell",
      "command": "claude",
      "options": { "cwd": "${workspaceFolder}" },
      "presentation": { "panel": "dedicated", "focus": true },
      "runOptions": { "runOn": "folderOpen" }
    }
  ]
}
EOF
}

# Write Cursor task configuration for auto-running Claude Code
write_cursor_autorun_task() {
  worktree_dir="$1"
  mkdir -p "$worktree_dir/.vscode"
  cat >"$worktree_dir/.vscode/tasks.json" <<'EOF'
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "claude",
      "type": "shell",
      "command": "claude",
      "options": { "cwd": "${workspaceFolder}" },
      "presentation": { "panel": "dedicated", "focus": true },
      "runOptions": { "runOn": "folderOpen" }
    }
  ]
}
EOF
}

# VS Code implementation (for completeness)
launch_vscode() {
  worktree_dir="$1"

  # Skip actual IDE launch in test mode
  if [ "${IDE_CMD:-}" = "true" ]; then
    echo "▶ skipping VS Code launch (test stub)"
    echo "✅ VS Code (test stub) opened"
    return 0
  fi

  # If IDE_CMD is a stub echo command, run it instead of launching
  case "$IDE_CMD" in
  echo\ *)
    eval "$IDE_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$IDE_CMD" >/dev/null 2>&1; then
    echo "▶ launching VS Code..."
    "$IDE_CMD" "$worktree_dir" &
    echo "✅ VS Code opened"
  else
    echo "⚠️  VS Code CLI not found. Please install VS Code CLI or set IDE_CMD environment variable." >&2
    echo "   Alternatively, manually open: $worktree_dir" >&2
  fi
}

# Get default IDE from configuration
get_default_ide() {
  # shellcheck disable=SC2153
  echo "$IDE_NAME"
}

# Check if IDE is available
is_ide_available() {
  ide_name="$1"

  case "$ide_name" in
  cursor | claude | code)
    command -v "$IDE_CMD" >/dev/null 2>&1
    ;;
  *)
    return 1
    ;;
  esac
}
