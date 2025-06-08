#!/usr/bin/env sh
# IDE integration for para - designed for extensibility

# Abstract IDE interface - to be implemented by specific IDE modules

# Launch IDE for a session
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"
  initial_prompt="${3:-}"
  skip_permissions="${4:-false}"
  session_id="${5:-}"

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
    die_ide_not_available "unsupported IDE: $ide_name"
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
  if [ "${IDE_CMD:-}" = "true" ]; then
    echo "▶ skipping Cursor launch (test stub)"
    echo "✅ Cursor (test stub) opened"
    return 0
  fi

  # If IDE_CMD is a stub starting with 'echo ', run it instead of launching
  case "$IDE_CMD" in
  echo\ *)
    eval "$IDE_CMD" "\"$worktree_dir\""
    return 0
    ;;
  esac

  if command -v "$IDE_CMD" >/dev/null 2>&1; then
    # Auto-detect session from worktree directory for launch tracking
    session_id=$(basename "$worktree_dir")

    # Save launch method as ide for auto-close functionality
    mkdir -p "$REPO_ROOT/.para_state"
    echo "LAUNCH_METHOD=ide" >"$REPO_ROOT/.para_state/$session_id.launch"
    echo "LAUNCH_IDE=cursor" >>"$REPO_ROOT/.para_state/$session_id.launch"

    # TODO: User data directory breaks window detection for auto-close
    # The --user-data-dir flag causes Cursor to reuse windows or create non-detectable windows
    # This needs to be redesigned to work with auto-close functionality
    # See: https://github.com/your-repo/para/issues/cursor-autoclose
    if [ -n "${CURSOR_USER_DATA_DIR:-}" ]; then
      # DISABLED: Use a single global para user data directory
      # This breaks window detection for auto-close functionality
      # global_user_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/para/cursor-userdata"

      # For now, launch without user data dir to ensure auto-close works
      echo "▶ launching Cursor (user data dir disabled for auto-close compatibility)..."
      "$IDE_CMD" "$worktree_dir" &
    else
      echo "▶ launching Cursor..."
      "$IDE_CMD" "$worktree_dir" &
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

  # If IDE_CMD is a stub echo command, run it instead of launching
  case "$IDE_CMD" in
  echo\ *)
    # Build the proper command that would be executed
    claude_cmd=$(build_claude_terminal_command "$initial_prompt" "" "$skip_permissions")
    eval "$IDE_CMD" "\"$claude_cmd\""
    return 0
    ;;
  esac

  # Claude Code requires wrapper mode
  if [ "${IDE_WRAPPER_ENABLED:-false}" != "true" ]; then
    echo "⚠️  Claude Code requires IDE wrapper mode. Please run 'para config' to enable wrapper mode." >&2
    echo "   Available options: VS Code wrapper or Cursor wrapper" >&2
    return 1
  fi

  echo "▶ launching Claude Code inside $IDE_WRAPPER_NAME wrapper..."
  launch_ide_with_wrapper "claude" "$worktree_dir" "$initial_prompt" "$skip_permissions"
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

# Internal helper function for writing autorun tasks
_write_autorun_task_for_claude() {
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

# Write VS Code auto-run task for Claude Code
write_vscode_autorun_task() {
  _write_autorun_task_for_claude "$@"
}

# Write Cursor auto-run task for Claude Code
write_cursor_autorun_task() {
  _write_autorun_task_for_claude "$@"
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
    # Auto-detect session from worktree directory for launch tracking
    session_id=$(basename "$worktree_dir")

    # Save launch method as ide for auto-close functionality
    mkdir -p "$REPO_ROOT/.para_state"
    echo "LAUNCH_METHOD=ide" >"$REPO_ROOT/.para_state/$session_id.launch"
    echo "LAUNCH_IDE=code" >>"$REPO_ROOT/.para_state/$session_id.launch"

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

# Close IDE for a finished session (with session match protection)
close_ide_for_session() {
  finished_session_id="$1"

  # Auto-detect current session from environment
  current_session_id=$(auto_detect_session 2>/dev/null || echo "")

  # Only close if the finished session matches the current session
  if [ "$finished_session_id" != "$current_session_id" ]; then
    return 0
  fi

  _perform_ide_close "$finished_session_id"
}

# Force close IDE for a session (no session match protection - used for cancel)
force_close_ide_for_session() {
  session_id="$1"
  _perform_ide_close "$session_id"
}

# Internal function to perform the actual IDE closing
_perform_ide_close() {
  session_id="$1"

  # Check if launch method file exists
  launch_file="$STATE_DIR/$session_id.launch"
  if [ ! -f "$launch_file" ]; then
    return 0
  fi

  # Read launch method
  LAUNCH_METHOD=$(grep "^LAUNCH_METHOD=" "$launch_file" | cut -d'=' -f2)

  case "$LAUNCH_METHOD" in
  ide | wrapper)
    # Close GUI IDE windows
    close_gui_ide_window "$session_id"
    ;;
  esac
}

# Close GUI IDE window on macOS
close_gui_ide_window() {
  session_id="$1"

  # Only works on macOS
  if ! command -v osascript >/dev/null 2>&1; then
    return 0
  fi

  # Get session info to find worktree path
  get_session_info "$session_id"
  worktree_basename=$(basename "$WORKTREE_DIR")

  # Determine which IDE app to close based on saved launch information
  launch_file="$STATE_DIR/$session_id.launch"
  if [ -f "$launch_file" ]; then
    LAUNCH_METHOD=$(grep "^LAUNCH_METHOD=" "$launch_file" | cut -d'=' -f2)
    if [ "$LAUNCH_METHOD" = "wrapper" ]; then
      # For wrapper mode, use the wrapper IDE that was actually used
      ide_name=$(grep "^WRAPPER_IDE=" "$launch_file" | cut -d'=' -f2)
      [ -z "$ide_name" ] && ide_name="$IDE_WRAPPER_NAME" # Fallback
    else
      # For terminal/ide mode, use the launch IDE that was actually used
      ide_name=$(grep "^LAUNCH_IDE=" "$launch_file" | cut -d'=' -f2)
      [ -z "$ide_name" ] && ide_name="$IDE_NAME" # Fallback
    fi
  else
    # Fallback to main IDE name for older sessions without launch files
    ide_name="$IDE_NAME"
  fi

  case "$ide_name" in
  cursor)
    app_name="Cursor"
    ;;
  code)
    app_name="Code"
    ;;
  *)
    return 0
    ;;
  esac

  # Different search strategies based on IDE
  if [ "$ide_name" = "cursor" ]; then
    # Cursor shows titles like "fish — session-name-20250607-123456"
    # Extract just the session name part (before timestamp) for more flexible matching
    # Session ID format: session-name-YYYYMMDD-HHMMSS
    session_name_part=$(echo "$worktree_basename" | sed 's/-[0-9]\{8\}-[0-9]\{6\}$//')
    search_fragment="$session_name_part"
  else
    # VS Code shows full worktree directory name in title
    search_fragment="$worktree_basename"
  fi

  # Use AppleScript to close the window (based on working implementation)
  result=$(
    osascript - "$app_name" "$search_fragment" <<'EOF'
on run argv
  set appName to item 1 of argv
  set windowTitleFragment to item 2 of argv
  
  log "AppleScript started for app: " & appName & " with title fragment: " & windowTitleFragment
  
  tell application "System Events"
    if not (exists process appName) then
      log "Error: Application process '" & appName & "' is not running."
      return "Application not running."
    end if
    
    tell process appName
      try
        set targetWindows to (every window whose name contains windowTitleFragment)
      on error errMsg
        log "Error: Could not get windows from " & appName & ". " & errMsg
        return "Error getting windows."
      end try

      if (count of targetWindows) is 0 then
        log "Failure: No window found with title containing '" & windowTitleFragment & "'"
        return "No matching window found."
      end if
      
      set targetWindow to item 1 of targetWindows
      
      log "Success: Found window: '" & (name of targetWindow) & "'"
      
      perform action "AXRaise" of targetWindow
      delay 0.2
      
      try
        click (button 1 of targetWindow)
        return "Successfully sent close command to window."
      on error
         log "Error: Could not click the close button. The window may not be standard."
         return "Could not click close button."
      end try

    end tell
  end tell
end run
EOF
  )

  echo "AppleScript result: $result" >&2
}
