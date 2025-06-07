#!/usr/bin/env sh
# Utility functions for para

# Usage message
usage() {
  cat <<EOF
para - Parallel IDE Workflow Helper

Commands:
para start [name]                    # create session with optional name
para dispatch "prompt"               # start Claude Code session with prompt
para dispatch --file path            # start Claude Code session with prompt from file
para dispatch-multi N "prompt"       # start N Claude Code instances with same prompt
para dispatch-multi N --file path    # start N Claude Code instances with prompt from file
para finish "message"                # squash all changes into single commit
para finish "message" --branch <n>   # squash commits + custom branch name
para list | ls                       # list active sessions
para resume [session]                # resume session in IDE
para cancel [session]                # cancel session
para cancel --group <name>           # cancel all sessions in multi-instance group
para clean                           # remove all sessions
para config                          # setup configuration
para completion generate [shell]     # generate shell completion script

Examples:
para start                           # create session with auto-generated name
para start feature-auth              # create named session
para dispatch "Add user auth"        # Claude Code session with prompt
para dispatch --file prompt.txt     # Claude Code session with prompt from file
para dispatch -f ./auth.prompt       # Claude Code session with prompt from file (short form)
para dispatch-multi 3 "Compare approaches"      # 3 Claude instances with same prompt
para dispatch-multi 3 --file prompt.txt         # 3 Claude instances with prompt from file
para dispatch-multi 5 --group task "Refactor"   # 5 instances with custom group name
para start --dangerously-skip-permissions name  # skip IDE permission warnings
para finish "implement user auth"    # squash session changes
para finish "add feature" --branch feature-xyz  # custom branch name
para list                            # see active sessions (shows multi-instance groups)
para cancel                          # cancel current session
para cancel --group task             # cancel all sessions in 'task' group
para clean                           # remove all sessions
para resume session-name             # resume specific session
para config                          # setup IDE preferences
para completion generate bash        # generate bash completion script

For configuration help: para config
EOF
}

# Print error message and exit
die() {
  echo "para: $*" >&2
  exit 1
}

# Assert that paths are properly initialized - prevents silent failures
assert_paths_initialized() {
  if [ -z "$STATE_DIR" ] || [ -z "$SUBTREES_DIR" ]; then
    die "INTERNAL ERROR: Paths not initialized. STATE_DIR='$STATE_DIR', SUBTREES_DIR='$SUBTREES_DIR'. This is a bug - init_paths() must be called first."
  fi

  if [ -z "$REPO_ROOT" ]; then
    die "INTERNAL ERROR: REPO_ROOT not set. This is a bug - need_git_repo() must be called first."
  fi

  # Verify paths look reasonable (not just variables)
  case "$STATE_DIR" in
  */*) ;; # Contains a slash, looks like a path
  *) die "INTERNAL ERROR: STATE_DIR='$STATE_DIR' doesn't look like a valid path. This is a bug." ;;
  esac

  case "$SUBTREES_DIR" in
  */*) ;; # Contains a slash, looks like a path
  *) die "INTERNAL ERROR: SUBTREES_DIR='$SUBTREES_DIR' doesn't look like a valid path. This is a bug." ;;
  esac
}

# Check if command is a known command
is_known_command() {
  cmd="$1"
  case "$cmd" in
  list | ls | clean | --help | -h | start | dispatch | dispatch-multi | finish | cancel | abort | resume | config | completion)
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

# Check if a string looks like a file path
is_file_path() {
  path="$1"

  # Return false if empty
  [ -n "$path" ] || return 1

  # Check if it's an existing file
  [ -f "$path" ] && return 0

  # Check if it looks like a file path (contains / or ends with common extensions)
  case "$path" in
  */*) return 0 ;;                            # Contains path separator
  *.txt | *.md | *.rst | *.org) return 0 ;;   # Common text file extensions
  *.prompt | *.tmpl | *.template) return 0 ;; # Prompt/template extensions
  *) return 1 ;;                              # Doesn't look like a file path
  esac
}

# Read file content with error handling
read_file_content() {
  file_path="$1"

  # Convert relative path to absolute if needed
  case "$file_path" in
  /*) absolute_path="$file_path" ;;
  *) absolute_path="$PWD/$file_path" ;;
  esac

  # Check if file exists and is readable
  if [ ! -f "$absolute_path" ]; then
    die "file not found: $file_path"
  fi

  if [ ! -r "$absolute_path" ]; then
    die "file not readable: $file_path"
  fi

  # Read file content
  cat "$absolute_path" || die "failed to read file: $file_path"
}

# Proper JSON string escaping for use in JSON files
json_escape_string() {
  input="$1"
  # Use printf to properly handle the string, then escape JSON special characters
  # This approach handles newlines, quotes, and other special characters properly
  printf '%s' "$input" | sed 's/\\/\\\\/g; s/"/\\"/g; s/$/\\n/; s/\t/\\t/g' | tr -d '\n' | sed 's/\\n$//'
}

# Get list of active session names for completion
get_session_names() {
  if [ ! -d "$STATE_DIR" ]; then
    return 0
  fi

  for state_file in "$STATE_DIR"/*.state; do
    [ -f "$state_file" ] || continue
    basename "$state_file" .state
  done
}

# Get list of multi-instance group names for completion
get_group_names() {
  if [ ! -d "$STATE_DIR" ]; then
    return 0
  fi

  for state_file in "$STATE_DIR"/*.state; do
    [ -f "$state_file" ] || continue
    session_id=$(basename "$state_file" .state)

    # Check for multi-session metadata
    meta_file="$STATE_DIR/$session_id.meta"
    if [ -f "$meta_file" ]; then
      # Extract group name from metadata
      while IFS='=' read -r key value; do
        case "$key" in
        GROUP_NAME) echo "$value" ;;
        esac
      done <"$meta_file"
    fi
  done | sort -u
}

# Get list of local branches for completion
get_branch_names() {
  git branch --format='%(refname:short)' 2>/dev/null | grep -v '^pc/' || true
}

# Generate shell completion script
generate_completion_script() {
  shell="$1"

  case "$shell" in
  bash)
    cat <<'EOF'
_para_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Complete first argument (commands)
    if [[ ${COMP_CWORD} == 1 ]]; then
        opts="start dispatch dispatch-multi finish cancel abort clean list ls resume config --help -h --version -v"
        COMPREPLY=($(compgen -W "${opts}" -- ${cur}))
        return 0
    fi
    
    # Complete based on command
    case "${COMP_WORDS[1]}" in
        cancel|abort)
            case "$prev" in
                --group)
                    # Complete group names
                    local groups=$(para _completion_groups 2>/dev/null)
                    COMPREPLY=($(compgen -W "${groups}" -- ${cur}))
                    ;;
                cancel|abort)
                    # Complete session names for direct cancel
                    local sessions=$(para _completion_sessions 2>/dev/null)
                    COMPREPLY=($(compgen -W "${sessions}" -- ${cur}))
                    ;;
            esac
            ;;
        resume)
            # Complete session names
            if [[ ${COMP_CWORD} == 2 ]]; then
                local sessions=$(para _completion_sessions 2>/dev/null)
                COMPREPLY=($(compgen -W "${sessions}" -- ${cur}))
            fi
            ;;
        finish)
            case "$prev" in
                --branch)
                    # Complete branch names
                    local branches=$(para _completion_branches 2>/dev/null)
                    COMPREPLY=($(compgen -W "${branches}" -- ${cur}))
                    ;;
            esac
            ;;
        dispatch|dispatch-multi)
            case "$prev" in
                --file|-f)
                    # Complete file paths
                    COMPREPLY=($(compgen -f -- ${cur}))
                    ;;
            esac
            ;;
    esac
}

complete -F _para_completion para
EOF
    ;;
  zsh)
    cat <<'EOF'
#compdef para

_para() {
    local context state line
    
    _arguments -C \
        '1: :_para_commands' \
        '*: :_para_args'
}

_para_commands() {
    local commands
    commands=(
        'start:create session with optional name'
        'dispatch:start Claude Code session with prompt'
        'dispatch-multi:start multiple Claude Code instances'
        'finish:squash all changes into single commit'
        'cancel:cancel session'
        'abort:cancel session'
        'clean:remove all sessions'
        'list:list active sessions'
        'ls:list active sessions'
        'resume:resume session in IDE'
        'config:setup configuration'
        '--help:show help'
        '-h:show help'
        '--version:show version'
        '-v:show version'
    )
    _describe 'commands' commands
}

_para_args() {
    case $words[2] in
        cancel|abort)
            if [[ $words[CURRENT-1] == '--group' ]]; then
                # Complete group names
                local groups=(${(f)"$(para _completion_groups 2>/dev/null)"})
                _describe 'groups' groups
            elif [[ $CURRENT == 3 ]]; then
                # Complete session names
                local sessions=(${(f)"$(para _completion_sessions 2>/dev/null)"})
                _describe 'sessions' sessions
            fi
            ;;
        resume)
            if [[ $CURRENT == 3 ]]; then
                # Complete session names
                local sessions=(${(f)"$(para _completion_sessions 2>/dev/null)"})
                _describe 'sessions' sessions
            fi
            ;;
        finish)
            if [[ $words[CURRENT-1] == '--branch' ]]; then
                # Complete branch names
                local branches=(${(f)"$(para _completion_branches 2>/dev/null)"})
                _describe 'branches' branches
            fi
            ;;
        dispatch|dispatch-multi)
            if [[ $words[CURRENT-1] == '--file' || $words[CURRENT-1] == '-f' ]]; then
                # Complete file paths
                _files
            fi
            ;;
    esac
}

_para "$@"
EOF
    ;;
  fish)
    cat <<'EOF'
# Para completion for fish shell

# Complete commands
complete -c para -f -n '__fish_use_subcommand' -a 'start' -d 'create session with optional name'
complete -c para -f -n '__fish_use_subcommand' -a 'dispatch' -d 'start Claude Code session with prompt'
complete -c para -f -n '__fish_use_subcommand' -a 'dispatch-multi' -d 'start multiple Claude Code instances'
complete -c para -f -n '__fish_use_subcommand' -a 'finish' -d 'squash all changes into single commit'
complete -c para -f -n '__fish_use_subcommand' -a 'cancel' -d 'cancel session'
complete -c para -f -n '__fish_use_subcommand' -a 'abort' -d 'cancel session'
complete -c para -f -n '__fish_use_subcommand' -a 'clean' -d 'remove all sessions'
complete -c para -f -n '__fish_use_subcommand' -a 'list' -d 'list active sessions'
complete -c para -f -n '__fish_use_subcommand' -a 'ls' -d 'list active sessions'
complete -c para -f -n '__fish_use_subcommand' -a 'resume' -d 'resume session in IDE'
complete -c para -f -n '__fish_use_subcommand' -a 'config' -d 'setup configuration'
complete -c para -f -n '__fish_use_subcommand' -s h -l help -d 'show help'
complete -c para -f -n '__fish_use_subcommand' -s v -l version -d 'show version'

# Complete session names for cancel/abort and resume
complete -c para -f -n '__fish_seen_subcommand_from cancel abort' -n 'not __fish_seen_argument -l group' -a '(para _completion_sessions 2>/dev/null)'
complete -c para -f -n '__fish_seen_subcommand_from resume' -a '(para _completion_sessions 2>/dev/null)'

# Complete group names for cancel/abort --group
complete -c para -f -n '__fish_seen_subcommand_from cancel abort' -s g -l group -a '(para _completion_groups 2>/dev/null)'

# Complete branch names for finish --branch
complete -c para -f -n '__fish_seen_subcommand_from finish' -s b -l branch -a '(para _completion_branches 2>/dev/null)'

# Complete file paths for dispatch/dispatch-multi --file
complete -c para -n '__fish_seen_subcommand_from dispatch dispatch-multi' -s f -l file -F
EOF
    ;;
  *)
    echo "Unsupported shell: $shell" >&2
    return 1
    ;;
  esac
}
