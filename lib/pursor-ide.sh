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
    echo "▶ launching Cursor..."
    "$CURSOR_CMD" "$worktree_dir" &
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