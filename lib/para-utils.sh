#!/usr/bin/env sh
# Utility functions for para

# Usage message
usage() {
  cat <<EOF
para - Parallel IDE Workflow Helper

Commands:
para start [name] ["prompt"]         # create session with optional name/prompt
para dispatch ["prompt"]             # start Claude Code session with prompt
para dispatch-multi N ["prompt"]     # start N Claude Code instances with same prompt
para finish "message"                # squash all changes into single commit
para finish "message" --branch <n>   # squash commits + custom branch name
para list | ls                       # list active sessions
para resume [session]                # resume session in IDE
para cancel [session]                # cancel session
para cancel --group <name>           # cancel all sessions in multi-instance group
para clean                           # remove all sessions
para config                          # setup configuration

Examples:
para start                           # create session with auto-generated name
para start feature-auth              # create named session
para dispatch "Add user auth"        # Claude Code session with prompt
para dispatch-multi 3 "Compare approaches"      # 3 Claude instances with same prompt
para dispatch-multi 5 --group task "Refactor"   # 5 instances with custom group name
para finish "implement user auth"    # squash session changes
para finish "add feature" --branch feature-xyz  # custom branch name
para list                            # see active sessions (shows multi-instance groups)
para cancel                          # cancel current session
para cancel --group task             # cancel all sessions in 'task' group
para clean                           # remove all sessions
para resume session-name             # resume specific session
para config                          # setup IDE preferences

For configuration help: para config
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
  list | ls | clean | --help | -h | start | dispatch | dispatch-multi | finish | cancel | abort | resume | config)
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

# Generate friendly name like Docker Compose (adjective_noun)
generate_friendly_name() {
  adjectives="
    agile bold calm deep eager fast keen neat 
    quick smart swift wise zesty bright clever 
    active brave clean crisp fresh happy 
    light rapid ready sharp sunny
  "

  nouns="
    alpha beta gamma delta omega
    aurora cosmos nebula quasar pulsar
    phoenix dragon falcon eagle hawk
    maple cedar birch pine oak
    ruby amber coral jade pearl
    atlas mercury venus mars jupiter
    river ocean stream creek lake
    spark flame ember blaze torch
    prism crystal silver golden bronze
  "

  adj_list=$(echo $adjectives | tr ' ' '\n' | grep -v '^$')
  noun_list=$(echo $nouns | tr ' ' '\n' | grep -v '^$')

  timestamp=$(date +%s)

  adj_count=$(echo "$adj_list" | wc -l)
  noun_count=$(echo "$noun_list" | wc -l)

  adj_index=$((timestamp % adj_count + 1))
  noun_index=$(((timestamp / adj_count) % noun_count + 1))

  adjective=$(echo "$adj_list" | sed -n "${adj_index}p")
  noun=$(echo "$noun_list" | sed -n "${noun_index}p")

  echo "${adjective}_${noun}"
}

# Generate unique session identifier (friendly name with timestamp suffix)
generate_session_id() {
  friendly=$(generate_friendly_name)
  timestamp=$(generate_timestamp)
  echo "${friendly}_${timestamp}"
}

# Get the main IDE user data directory path
get_main_ide_user_data_dir() {
  case "$IDE_NAME" in
  cursor)
    case "$(uname)" in
    Darwin) echo "$HOME/Library/Application Support/Cursor" ;;
    Linux) echo "$HOME/.config/Cursor" ;;
    *) echo "$HOME/.config/Cursor" ;;
    esac
    ;;
  claude)
    case "$(uname)" in
    Darwin) echo "$HOME/Library/Application Support/Claude" ;;
    Linux) echo "$HOME/.config/Claude" ;;
    *) echo "$HOME/.config/Claude" ;;
    esac
    ;;
  code)
    case "$(uname)" in
    Darwin) echo "$HOME/Library/Application Support/Code" ;;
    Linux) echo "$HOME/.config/Code" ;;
    *) echo "$HOME/.config/Code" ;;
    esac
    ;;
  *)
    # Generic fallback
    case "$(uname)" in
    Darwin) echo "$HOME/Library/Application Support/$IDE_NAME" ;;
    Linux) echo "$HOME/.config/$IDE_NAME" ;;
    *) echo "$HOME/.config/$IDE_NAME" ;;
    esac
    ;;
  esac
}

# Backwards compatibility alias
get_main_cursor_user_data_dir() {
  get_main_ide_user_data_dir
}

# Check if para template exists
template_exists() {
  [ -d "$TEMPLATE_DIR" ]
}

# Setup para template by copying main IDE user data
setup_para_template() {
  ide_display_name=$(get_ide_display_name)
  main_ide_dir=$(get_main_ide_user_data_dir)

  if [ ! -d "$main_ide_dir" ]; then
    echo "⚠️  Main $ide_display_name user data directory not found at: $main_ide_dir"
    echo "   Starting with fresh $ide_display_name environment for para sessions."
    return 1
  fi

  echo "🔧 Setting up para template from your main $ide_display_name configuration..."
  echo "   Copying from: $main_ide_dir"
  echo "   To template: $TEMPLATE_DIR"

  # Create template directory
  mkdir -p "$TEMPLATE_DIR"

  # Copy main IDE user data to template (excluding logs, cache, and problematic files)
  if command -v rsync >/dev/null 2>&1; then
    rsync -a --exclude='logs/' --exclude='Cache/' --exclude='CachedData/' \
      --exclude='GPUCache/' --exclude='Code Cache/' --exclude='DawnWebGPUCache/' \
      --exclude='DawnGraphiteCache/' --exclude='*.lock' --exclude='*.sock' \
      --exclude='Local Storage/' --exclude='Session Storage/' \
      --exclude='blob_storage/' --exclude='Shared Dictionary/' \
      "$main_ide_dir/" "$TEMPLATE_DIR/"
  else
    # Fallback to cp if rsync is not available
    cp -r "$main_ide_dir"/* "$TEMPLATE_DIR/" 2>/dev/null || true
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
