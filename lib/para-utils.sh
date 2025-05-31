#!/usr/bin/env sh
# Utility functions for para

# Usage message
usage() {
  cat <<EOF
para - Parallel Cursor IDE workflow helper

USAGE:
  para                        # create new session (opens Cursor)
  para <name>                 # create named session
  para rebase "message"          # squash all changes into one commit (default)
  para rebase --preserve "message" # rebase individual commits (preserve history)
  para list                   # list all active sessions
  para continue                 # continue rebase after resolving conflicts
  para cancel [session]       # cancel/delete session
  para clean                  # delete all sessions
  para resume <session>       # resume session in Cursor

EXAMPLES:
  para                        # auto-named session (timestamp)
  para feature-auth           # named session  
  para list                   # show all sessions
  para rebase "Add new feature"  # squash all changes into one commit
  para rebase --preserve "Feature" # preserve individual commit history
  para continue               # after resolving conflicts
  para cancel                 # cancel current session
  para clean                  # clean up everything

For more information, see the README.md
EOF
}

# Print error message and exit
die() {
  echo "para: $*" >&2
  exit 1
}

# Check if command is a known command
is_known_command() {
  cmd="$1"
  case "$cmd" in
    list|ls|clean|--help|-h|rebase|continue|cancel|abort|resume|--preserve)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

# Validate session name format
validate_session_name() {
  session_name="$1"
  case "$session_name" in
    *[!a-zA-Z0-9_-]*) 
      die "session name can only contain letters, numbers, dashes, and underscores" 
      ;;
  esac
}

# Generate timestamp for session IDs
generate_timestamp() {
  date +%Y%m%d-%H%M%S
}

# Get the main Cursor user data directory path
get_main_cursor_user_data_dir() {
  case "$(uname)" in
    Darwin)
      echo "$HOME/Library/Application Support/Cursor"
      ;;
    Linux)
      echo "$HOME/.config/Cursor"
      ;;
    *)
      echo "$HOME/.config/Cursor"
      ;;
  esac
}

# Check if para template exists
template_exists() {
  [ -d "$TEMPLATE_DIR" ]
}

# Setup para template by copying main Cursor user data
setup_para_template() {
  main_cursor_dir=$(get_main_cursor_user_data_dir)
  
  if [ ! -d "$main_cursor_dir" ]; then
    echo "⚠️  Main Cursor user data directory not found at: $main_cursor_dir"
    echo "   Starting with fresh Cursor environment for para sessions."
    return 1
  fi
  
  echo "🔧 Setting up para template from your main Cursor configuration..."
  echo "   Copying from: $main_cursor_dir"
  echo "   To template: $TEMPLATE_DIR"
  
  # Create template directory
  mkdir -p "$TEMPLATE_DIR"
  
  # Copy main Cursor user data to template (excluding logs, cache, and problematic files)
  if command -v rsync >/dev/null 2>&1; then
    rsync -a --exclude='logs/' --exclude='Cache/' --exclude='CachedData/' \
          --exclude='GPUCache/' --exclude='Code Cache/' --exclude='DawnWebGPUCache/' \
          --exclude='DawnGraphiteCache/' --exclude='*.lock' --exclude='*.sock' \
          --exclude='Local Storage/' --exclude='Session Storage/' \
          --exclude='blob_storage/' --exclude='Shared Dictionary/' \
          "$main_cursor_dir/" "$TEMPLATE_DIR/"
  else
    # Fallback to cp if rsync is not available
    cp -r "$main_cursor_dir"/* "$TEMPLATE_DIR/" 2>/dev/null || true
    # Remove cache directories and problematic files that shouldn't be copied
    rm -rf "$TEMPLATE_DIR/logs" "$TEMPLATE_DIR/Cache" "$TEMPLATE_DIR/CachedData" \
           "$TEMPLATE_DIR/GPUCache" "$TEMPLATE_DIR/Code Cache" \
           "$TEMPLATE_DIR/DawnWebGPUCache" "$TEMPLATE_DIR/DawnGraphiteCache" \
           "$TEMPLATE_DIR/Local Storage" "$TEMPLATE_DIR/Session Storage" \
           "$TEMPLATE_DIR/blob_storage" "$TEMPLATE_DIR/Shared Dictionary" \
           "$TEMPLATE_DIR"/*.lock "$TEMPLATE_DIR"/*.sock 2>/dev/null || true
  fi
  
  echo "✅ Para template created successfully!"
  echo "   Your extensions and settings will now be available in all para sessions."
} 