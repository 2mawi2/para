#!/usr/bin/env sh
# IDE integration for pursor - designed for extensibility

# Abstract IDE interface - to be implemented by specific IDE modules

# Launch IDE for a session
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"
  
  case "$ide_name" in
    cursor)
      launch_cursor "$worktree_dir"
      ;;
    *)
      die "unsupported IDE: $ide_name"
      ;;
  esac
}

# Cursor IDE implementation
launch_cursor() {
  worktree_dir="$1"
  
  if command -v "$CURSOR_CMD" >/dev/null 2>&1; then
    if [ -n "${CURSOR_USER_DATA_DIR:-}" ]; then
      # Create isolated user data directory in the worktree
      user_data_path="$worktree_dir/$CURSOR_USER_DATA_DIR"
      
      # Setup template on first use
      if ! template_exists; then
        setup_pursor_template
      fi
      
      # Create session user data directory
      mkdir -p "$user_data_path"
      
      # Copy from template if it exists
      if template_exists; then
        echo "📋 Copying settings from pursor template..."
        if command -v rsync >/dev/null 2>&1; then
          rsync -a --exclude='*.sock' --exclude='*.lock' \
                --exclude='Local Storage/' --exclude='Session Storage/' \
                --exclude='blob_storage/' --exclude='Shared Dictionary/' \
                "$TEMPLATE_DIR/" "$user_data_path/"
        else
          cp -r "$TEMPLATE_DIR"/* "$user_data_path/" 2>/dev/null || true
          # Remove problematic files
          rm -rf "$user_data_path"/*.sock "$user_data_path"/*.lock \
                 "$user_data_path/Local Storage" "$user_data_path/Session Storage" \
                 "$user_data_path/blob_storage" "$user_data_path/Shared Dictionary" \
                 2>/dev/null || true
        fi
        echo "✅ Settings copied to session"
      fi
      
      echo "▶ launching Cursor with isolated user data directory..."
      "$CURSOR_CMD" "$worktree_dir" --user-data-dir "$user_data_path" &
    else
      echo "▶ launching Cursor..."
      "$CURSOR_CMD" "$worktree_dir" &
    fi
    echo "✅ Cursor opened"
  else
    echo "⚠️  Cursor CLI not found. Please install Cursor CLI or set CURSOR_CMD environment variable." >&2
    echo "   Alternatively, manually open: $worktree_dir" >&2
    echo "   💡 Install Cursor CLI: https://cursor.sh/cli" >&2
  fi
}

# Get default IDE (currently always Cursor, but designed for future configuration)
get_default_ide() {
  echo "cursor"
}

# Check if IDE is available
is_ide_available() {
  ide_name="$1"
  
  case "$ide_name" in
    cursor)
      command -v "$CURSOR_CMD" >/dev/null 2>&1
      ;;
    *)
      return 1
      ;;
  esac
} 