#!/usr/bin/env sh
# IDE integration for para - designed for extensibility

# Abstract IDE interface - to be implemented by specific IDE modules

# Launch IDE for a session
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"

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

# Cursor IDE implementation
launch_cursor() {
  worktree_dir="$1"

  # Skip actual IDE launch and template setup in test mode
  if [ "${CURSOR_CMD:-}" = "true" ]; then
    echo "▶ skipping Cursor launch (test stub)"
    echo "✅ Cursor (test stub) opened"
    return 0
  fi

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

  if command -v "$IDE_CMD" >/dev/null 2>&1; then
    echo "▶ launching Claude Code in new terminal..."
    # Launch Claude Code in a new terminal window on macOS
    if command -v open >/dev/null 2>&1; then
      open -n -a Terminal.app --args sh -c "cd '$worktree_dir' && '$IDE_CMD'"
    else
      # Fallback for non-macOS systems - try common terminal emulators
      if command -v gnome-terminal >/dev/null 2>&1; then
        gnome-terminal --working-directory="$worktree_dir" -- "$IDE_CMD"
      elif command -v xterm >/dev/null 2>&1; then
        (cd "$worktree_dir" && xterm -e "$IDE_CMD") &
      else
        echo "⚠️  Could not detect terminal emulator. Running in current terminal..."
        cd "$worktree_dir" && "$IDE_CMD"
        echo "✅ Claude Code session ended"
        return 0
      fi
    fi
    echo "✅ Claude Code opened in new terminal"
  else
    echo "⚠️  Claude Code CLI not found. Please install Claude Code CLI or set IDE_CMD environment variable." >&2
    echo "   Alternatively, manually open: $worktree_dir" >&2
  fi
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
