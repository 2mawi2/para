#!/usr/bin/env sh
# IDE integration for para - designed for extensibility

# Abstract IDE interface - to be implemented by specific IDE modules

# Launch IDE for a session
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"
  initial_prompt="${3:-}"
  skip_permissions="${4:-false}"

  # Check if IDE wrapper is enabled and we're launching Claude Code
  if [ "$ide_name" = "claude" ] && [ "${IDE_WRAPPER_ENABLED:-false}" = "true" ]; then
    echo "▶ launching Claude Code inside $IDE_WRAPPER_NAME wrapper..."
    launch_ide_with_wrapper "$ide_name" "$worktree_dir" "$initial_prompt" "$skip_permissions"
    return
  fi

  case "$ide_name" in
  cursor)
    launch_cursor "$worktree_dir"
    ;;
  claude)
    launch_claude "$worktree_dir" "$initial_prompt" "$skip_permissions"
    ;;
  code)
    launch_vscode "$worktree_dir"
    ;;
  *)
    die "unsupported IDE: $ide_name"
    ;;
  esac
}

# Launch multiple IDE instances for multi-session dispatch
launch_multi_ide() {
  ide_name="$1"
  session_ids="$2"
  initial_prompt="${3:-}"
  skip_permissions="${4:-false}"

  echo "▶ launching $ide_name for multiple instances..."

  # Launch each instance with a brief delay to avoid overwhelming the system
  for session_id in $session_ids; do
    # Get worktree directory for this session
    get_session_info "$session_id"

    echo "  → launching instance for session $session_id"
    # shellcheck disable=SC2153
    launch_ide "$ide_name" "$WORKTREE_DIR" "$initial_prompt" "$skip_permissions" &

    # Brief delay between launches
    sleep 0.5
  done

  # Wait for all background processes to complete
  wait

  echo "✅ All instances launched successfully"
}

# Launch IDE with wrapper functionality
launch_ide_with_wrapper() {
  ide_name="$1"
  worktree_dir="$2"
  initial_prompt="${3:-}"
  skip_permissions="${4:-false}"

  case "$IDE_WRAPPER_NAME" in
  code)
    write_vscode_autorun_task "$worktree_dir" "$initial_prompt" "" "$skip_permissions"
    launch_vscode_wrapper "$worktree_dir" "$initial_prompt"
    ;;
  cursor)
    write_cursor_autorun_task "$worktree_dir" "$initial_prompt" "" "$skip_permissions"
    launch_cursor_wrapper "$worktree_dir" "$initial_prompt"
    ;;
  *)
    echo "⚠️  Unsupported wrapper IDE: $IDE_WRAPPER_NAME" >&2
    echo "   Falling back to regular Claude Code launch..." >&2
    launch_claude "$worktree_dir" "$initial_prompt" "$skip_permissions"
    ;;
  esac
}

# Launch VS Code as wrapper for Claude Code
launch_vscode_wrapper() {
  worktree_dir="$1"
  initial_prompt="${2:-}"

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
    launch_claude "$worktree_dir" "$initial_prompt"
  fi
}

# Launch Cursor as wrapper for Claude Code
launch_cursor_wrapper() {
  worktree_dir="$1"
  initial_prompt="${2:-}"

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
    launch_claude "$worktree_dir" "$initial_prompt"
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
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  # Skip actual IDE launch in test mode
  if [ "${IDE_CMD:-}" = "true" ]; then
    echo "▶ skipping Claude Code launch (test stub)"
    echo "✅ Claude Code (test stub) opened"
    return 0
  fi

  # If IDE_CMD is a stub echo command, run it instead of opening a new terminal
  case "$IDE_CMD" in
  echo\ *)
    # Build the proper command that would be executed
    claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")
    eval "$IDE_CMD" "\"$claude_cmd\""
    return 0
    ;;
  esac

  if command -v "$IDE_CMD" >/dev/null 2>&1; then
    echo "▶ launching Claude Code in new terminal..."

    # Determine which terminal to use based on CLAUDE_TERMINAL_CMD
    case "$CLAUDE_TERMINAL_CMD" in
    auto)
      # Auto-detect available terminal
      launch_claude_auto_terminal "$worktree_dir" "$initial_prompt" "$skip_permissions"
      ;;
    terminal)
      # Force use of macOS Terminal.app
      launch_claude_terminal_app "$worktree_dir" "$initial_prompt" "$skip_permissions"
      ;;
    warp)
      # Use Warp terminal
      launch_claude_warp "$worktree_dir" "$initial_prompt" "$skip_permissions"
      ;;
    ghostty)
      # Use Ghostty terminal
      launch_claude_ghostty "$worktree_dir" "$initial_prompt" "$skip_permissions"
      ;;
    *)
      # Custom terminal command
      launch_claude_custom_terminal "$worktree_dir" "$CLAUDE_TERMINAL_CMD" "$initial_prompt" "$skip_permissions"
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
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  if command -v warp-cli >/dev/null 2>&1; then
    launch_claude_warp "$worktree_dir" "$initial_prompt" "$skip_permissions"
  elif [ -d "/Applications/Ghostty.app" ] && command -v ghostty >/dev/null 2>&1; then
    launch_claude_ghostty "$worktree_dir" "$initial_prompt" "$skip_permissions"
  elif command -v osascript >/dev/null 2>&1; then
    launch_claude_terminal_app "$worktree_dir" "$initial_prompt" "$skip_permissions"
  else
    # Fallback for non-macOS systems
    launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
  fi
}

# Build Claude Code command for VS Code tasks (JSON format)
build_claude_command() {
  initial_prompt="$1"

  if [ -n "$initial_prompt" ]; then
    # Return the base command - arguments will be handled separately in JSON
    echo "$IDE_CMD"
  else
    echo "$IDE_CMD"
  fi
}

# Build Claude Code command for terminal with proper shell escaping
build_claude_terminal_command() {
  initial_prompt="$1"
  session_id="${2:-}"
  skip_permissions="${3:-false}"

  # Build base command with optional --dangerously-skip-permissions flag
  base_cmd="$IDE_CMD"
  if [ "$skip_permissions" = "true" ]; then
    base_cmd="$base_cmd --dangerously-skip-permissions"
  fi

  if [ -n "$initial_prompt" ]; then
    # Escape the prompt for shell execution using double quotes (safer for nested contexts)
    # Escape backslashes, double quotes, dollar signs, and backticks
    prompt_escaped=$(printf '%s' "$initial_prompt" | sed "s/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\\$/\\\\\\\$/g; s/\`/\\\\\`/g")

    # Use session resumption if session_id is provided
    if [ -n "$session_id" ]; then
      # Resume existing session with new prompt (interactive mode)
      echo "$base_cmd --resume \"$session_id\" \"$prompt_escaped\""
    else
      # Start new session with initial prompt (interactive mode)
      echo "$base_cmd \"$prompt_escaped\""
    fi
  else
    # Use session resumption without prompt if session_id is provided
    if [ -n "$session_id" ]; then
      # Resume existing session interactively
      echo "$base_cmd --resume \"$session_id\""
    else
      # Start new interactive session
      echo "$base_cmd"
    fi
  fi
}

# Launch using macOS Terminal.app
launch_claude_terminal_app() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  if command -v osascript >/dev/null 2>&1; then
    # Build the proper command for terminal
    claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")
    # Use AppleScript to create a new terminal window and run Claude Code
    osascript <<EOF
tell application "Terminal"
    do script "cd '$worktree_dir' && $claude_cmd"
    activate
end tell
EOF
    echo "✅ Claude Code opened in Terminal.app"
  else
    echo "⚠️  AppleScript not available. Cannot launch Terminal.app" >&2
    launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
  fi
}

# Launch using Warp terminal
launch_claude_warp() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  if [ -d "/Applications/Warp.app" ] || command -v warp-terminal >/dev/null 2>&1; then
    # Build the proper command for terminal
    claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")

    # Use Warp URI scheme to open a new tab with the command
    # First open the directory in Warp
    if command -v open >/dev/null 2>&1; then
      # Use macOS open command with URI scheme
      open "warp://action/new_tab?path=$worktree_dir"

      # Wait a moment for the tab to open, then use AppleScript to send the command
      sleep 1

      if command -v osascript >/dev/null 2>&1; then
        osascript <<EOF
tell application "Warp"
    activate
    tell application "System Events"
        tell process "Warp"
            keystroke "$claude_cmd"
            key code 36
        end tell
    end tell
end tell
EOF
      fi
      echo "✅ Claude Code opened in Warp"
    else
      # Fallback to xdg-open for Linux
      if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "warp://action/new_tab?path=$worktree_dir"

        # For Linux, we can't easily send keystrokes, so just open the directory
        # The user will need to manually type the claude command
        echo "✅ Warp opened at $worktree_dir"
        if [ -n "$initial_prompt" ]; then
          echo "💡 Run this command in Warp: $claude_cmd"
        fi
      else
        echo "⚠️  Could not open Warp with URI scheme" >&2
        launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
      fi
    fi
  else
    echo "⚠️  Warp terminal not found. Please install Warp or use a different terminal." >&2
    launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
  fi
}

# Launch using Ghostty terminal
launch_claude_ghostty() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  # Use AppleScript to script Ghostty, fallback if not available
  if [ -d "/Applications/Ghostty.app" ] && command -v osascript >/dev/null 2>&1; then
    # Open Ghostty
    open -a Ghostty.app
    # Wait for the app to be ready
    sleep 0.5
    # Build the command to run
    claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")
    claude_cmd="cd '$worktree_dir' && $claude_cmd"
    # Use AppleScript to open a new tab and run the command
    osascript <<EOF
      tell application "Ghostty"
        activate
      end tell
      tell application "System Events"
        tell process "Ghostty"
          keystroke "t" using {command down}
          delay 0.2
          keystroke "$claude_cmd"
          key code 36
        end tell
      end tell
EOF
    echo "✅ Claude Code opened in Ghostty"
  else
    echo "⚠️  Ghostty scripting not available. Please install Ghostty.app or use a different terminal." >&2
    launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
  fi
}

# Launch using custom terminal command
launch_claude_custom_terminal() {
  worktree_dir="$1"
  custom_cmd="$2"
  initial_prompt="${3:-}"
  skip_permissions="${4:-false}"

  # Build the proper command for terminal
  claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")

  # Replace placeholders in the custom command
  # %d = directory, %c = command
  terminal_cmd=$(echo "$custom_cmd" | sed "s|%d|$worktree_dir|g" | sed "s|%c|$claude_cmd|g")

  if eval "$terminal_cmd"; then
    echo "✅ Claude Code opened in custom terminal"
  else
    echo "⚠️  Failed to launch with custom terminal command: $custom_cmd" >&2
    launch_claude_fallback "$worktree_dir" "$initial_prompt" "$skip_permissions"
  fi
}

# Fallback terminal launcher
launch_claude_fallback() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  skip_permissions="${3:-false}"

  # Build the proper command for terminal
  claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")

  # Try common terminal emulators
  if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --working-directory="$worktree_dir" -- sh -c "$claude_cmd"
    echo "✅ Claude Code opened in gnome-terminal"
  elif command -v xterm >/dev/null 2>&1; then
    (cd "$worktree_dir" && xterm -e sh -c "$claude_cmd") &
    echo "✅ Claude Code opened in xterm"
  elif command -v konsole >/dev/null 2>&1; then
    konsole --workdir "$worktree_dir" -e sh -c "$claude_cmd" &
    echo "✅ Claude Code opened in konsole"
  else
    echo "⚠️  Could not detect terminal emulator. Running in current terminal..."
    cd "$worktree_dir" && eval "$claude_cmd"
    echo "✅ Claude Code session ended"
  fi
}

# Generate VS Code task JSON for Claude Code
generate_claude_task() {
  label="$1"
  full_cmd="$2"

  cat <<EOF
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "$label",
            "type": "shell",
            "command": "$full_cmd",
            "group": "build",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": true,
                "panel": "new",
                "showReuseMessage": false,
                "clear": false
            },
            "runOptions": {
                "runOn": "folderOpen"
            }
        }
    ]
}
EOF
}

# Build Claude Code command for wrapper tasks
build_claude_wrapper_command() {
  initial_prompt="$1"
  session_id="${2:-}"
  skip_permissions="${3:-false}"
  worktree_dir="$4"

  base_cmd="$IDE_CMD"
  if [ "$skip_permissions" = "true" ]; then
    base_cmd="$base_cmd --dangerously-skip-permissions"
  fi

  if [ -n "$initial_prompt" ]; then
    temp_prompt_file="$worktree_dir/.claude_prompt_temp"
    printf '%s' "$initial_prompt" >"$temp_prompt_file"

    if [ -n "$session_id" ]; then
      echo "$base_cmd --resume \\\"$session_id\\\" \\\"\$(cat '$temp_prompt_file'; rm '$temp_prompt_file')\\\""
    else
      echo "$base_cmd \\\"\$(cat '$temp_prompt_file'; rm '$temp_prompt_file')\\\""
    fi
  else
    if [ -n "$session_id" ]; then
      echo "$base_cmd --resume \\\"$session_id\\\""
    else
      echo "$base_cmd"
    fi
  fi
}

# Write VS Code auto-run task for Claude Code
write_vscode_autorun_task() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  session_id="${3:-}"
  skip_permissions="${4:-false}"

  mkdir -p "$worktree_dir/.vscode"

  full_cmd=$(build_claude_wrapper_command "$initial_prompt" "$session_id" "$skip_permissions" "$worktree_dir")

  if [ -n "$initial_prompt" ] && [ -n "$session_id" ]; then
    label="Resume Claude Code Session with Prompt"
  elif [ -n "$initial_prompt" ]; then
    label="Start Claude Code with Prompt"
  elif [ -n "$session_id" ]; then
    label="Resume Claude Code Session"
  else
    label="Start Claude Code"
  fi

  generate_claude_task "$label" "$full_cmd" >"$worktree_dir/.vscode/tasks.json"
}

# Write Cursor auto-run task for Claude Code
write_cursor_autorun_task() {
  worktree_dir="$1"
  initial_prompt="${2:-}"
  session_id="${3:-}"
  skip_permissions="${4:-false}"

  mkdir -p "$worktree_dir/.vscode"

  full_cmd=$(build_claude_wrapper_command "$initial_prompt" "$session_id" "$skip_permissions" "$worktree_dir")

  if [ -n "$initial_prompt" ] && [ -n "$session_id" ]; then
    label="Resume Claude Code Session with Prompt"
  elif [ -n "$initial_prompt" ]; then
    label="Start Claude Code with Prompt"
  elif [ -n "$session_id" ]; then
    label="Resume Claude Code Session"
  else
    label="Start Claude Code"
  fi

  generate_claude_task "$label" "$full_cmd" >"$worktree_dir/.vscode/tasks.json"
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
