#!/usr/bin/env sh
# Command handlers for para

# Handle start command (simplified - no more --prompt support)
handle_start_command() {
  SESSION_NAME=""
  SKIP_PERMISSIONS=false

  # Parse arguments
  shift # Remove 'start'

  # Collect positional arguments separately from flags
  positional_args=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --dangerously-skip-permissions | -d)
      SKIP_PERMISSIONS=true
      shift
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      if [ -z "$positional_args" ]; then
        positional_args="$1"
      else
        positional_args="$positional_args|$1"
      fi
      shift
      ;;
    esac
  done

  # Process positional arguments
  if [ -n "$positional_args" ]; then
    # Count the number of positional arguments
    arg_count=$(echo "$positional_args" | tr '|' '\n' | wc -l)

    if [ "$arg_count" -eq 1 ]; then
      # Only session name provided
      SESSION_NAME="$positional_args"
      validate_session_name "$SESSION_NAME"
    else
      die "too many arguments"
    fi
  fi

  # Session creation logic - no initial prompt
  create_new_session "$SESSION_NAME" "" "$SKIP_PERMISSIONS"
}

# Handle dispatch command - creates session with prompt
handle_dispatch_command() {
  # Validate that Claude Code is configured (wrapper mode is also supported)
  if [ "$IDE_NAME" != "claude" ]; then
    die "dispatch command only works with Claude Code. Current IDE: $(get_ide_display_name). Run 'para config' to switch to Claude Code."
  fi

  INITIAL_PROMPT=""
  SESSION_NAME=""
  SKIP_PERMISSIONS=false
  FILE_PATH=""

  # Parse arguments
  shift # Remove 'dispatch'

  # Collect positional arguments separately from flags
  positional_args=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --file=*)
      FILE_PATH="${1#--file=}"
      shift
      ;;
    --file | -f)
      if [ "$#" -lt 2 ]; then
        die "--file requires a file path"
      fi
      FILE_PATH="$2"
      shift 2
      ;;
    --dangerously-skip-permissions | -d)
      SKIP_PERMISSIONS=true
      shift
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      if [ -z "$positional_args" ]; then
        positional_args="$1"
      else
        positional_args="$positional_args|$1"
      fi
      shift
      ;;
    esac
  done

  # Process positional arguments
  if [ -n "$positional_args" ]; then
    # Count the number of positional arguments
    arg_count=$(echo "$positional_args" | tr '|' '\n' | wc -l)

    if [ "$arg_count" -eq 1 ]; then
      # Check if single argument is a file path or prompt text
      if [ -z "$FILE_PATH" ] && is_file_path "$positional_args"; then
        # Auto-detect file path
        FILE_PATH="$positional_args"
      else
        # Only prompt provided
        INITIAL_PROMPT="$positional_args"
      fi
    elif [ "$arg_count" -eq 2 ]; then
      # Session name and prompt provided
      SESSION_NAME=$(echo "$positional_args" | cut -d'|' -f1)
      prompt_or_file=$(echo "$positional_args" | cut -d'|' -f2)

      # Check if second argument is a file path
      if [ -z "$FILE_PATH" ] && is_file_path "$prompt_or_file"; then
        FILE_PATH="$prompt_or_file"
      else
        INITIAL_PROMPT="$prompt_or_file"
      fi

      validate_session_name "$SESSION_NAME"
    else
      die "too many arguments"
    fi
  fi

  # Handle file input if provided
  if [ -n "$FILE_PATH" ]; then
    INITIAL_PROMPT=$(read_file_content "$FILE_PATH")
    if [ -z "$INITIAL_PROMPT" ]; then
      die "file is empty: $FILE_PATH"
    fi
  fi

  # Validate required arguments
  if [ -z "$INITIAL_PROMPT" ]; then
    die "dispatch requires a prompt text or file path"
  fi

  # Session creation logic with initial prompt
  create_new_session "$SESSION_NAME" "$INITIAL_PROMPT" "$SKIP_PERMISSIONS"
}

# Handle dispatch-multi command - creates multiple sessions with same prompt
handle_dispatch_multi_command() {
  # Validate that Claude Code is configured
  if [ "$IDE_NAME" != "claude" ]; then
    die "dispatch-multi command only works with Claude Code. Current IDE: $(get_ide_display_name). Run 'para config' to switch to Claude Code."
  fi

  INSTANCE_COUNT=""
  INITIAL_PROMPT=""
  SESSION_BASE_NAME=""
  SKIP_PERMISSIONS=false
  FILE_PATH=""

  # Skip the command name (dispatch-multi)
  shift

  # Parse arguments with --group, --file, and --dangerously-skip-permissions flag support
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --group=*)
      SESSION_BASE_NAME="${1#--group=}"
      validate_session_name "$SESSION_BASE_NAME"
      shift
      ;;
    --group)
      if [ "$#" -lt 2 ]; then
        die "--group requires a group name"
      fi
      SESSION_BASE_NAME="$2"
      validate_session_name "$SESSION_BASE_NAME"
      shift 2
      ;;
    --file=*)
      FILE_PATH="${1#--file=}"
      shift
      ;;
    --file | -f)
      if [ "$#" -lt 2 ]; then
        die "--file requires a file path"
      fi
      FILE_PATH="$2"
      shift 2
      ;;
    --dangerously-skip-permissions | -d)
      SKIP_PERMISSIONS=true
      shift
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      # First positional argument should be instance count
      if [ -z "$INSTANCE_COUNT" ]; then
        INSTANCE_COUNT="$1"
        shift
      # Second positional argument should be prompt or file path
      elif [ -z "$INITIAL_PROMPT" ] && [ -z "$FILE_PATH" ]; then
        # Check if argument is a file path
        if is_file_path "$1"; then
          FILE_PATH="$1"
        else
          INITIAL_PROMPT="$1"
        fi
        shift
      else
        die "too many arguments"
      fi
      ;;
    esac
  done

  # Handle file input if provided
  if [ -n "$FILE_PATH" ]; then
    INITIAL_PROMPT=$(read_file_content "$FILE_PATH")
    if [ -z "$INITIAL_PROMPT" ]; then
      die "file is empty: $FILE_PATH"
    fi
  fi

  # Validate required arguments
  if [ -z "$INSTANCE_COUNT" ]; then
    die "dispatch-multi usage: 'para dispatch-multi N \"prompt\"' or 'para dispatch-multi N --group name \"prompt\"' or 'para dispatch-multi N --file path'"
  fi

  if [ -z "$INITIAL_PROMPT" ]; then
    die "dispatch-multi requires a prompt text or file path"
  fi

  # Validate instance count
  if ! echo "$INSTANCE_COUNT" | grep -q "^[1-9][0-9]*$"; then
    die "instance count must be a positive integer"
  fi

  # Reasonable limit to prevent system overload
  if [ "$INSTANCE_COUNT" -gt 10 ]; then
    die "instance count limited to 10 to prevent system overload"
  fi

  # Multi-session creation logic with initial prompt
  create_new_multi_session "$INSTANCE_COUNT" "$SESSION_BASE_NAME" "$INITIAL_PROMPT" "$SKIP_PERMISSIONS"
}

# Handle finish command
handle_finish_command() {
  COMMIT_MSG=""
  TARGET_BRANCH_NAME=""
  BRANCH_FLAG_PROVIDED=false
  SESSION_ID=""
  INTEGRATE=false

  # Parse arguments supporting --branch and --integrate flags and optional session_id
  if [ "$#" -eq 1 ]; then
    die "finish requires a commit message"
  else
    # Parse all arguments to handle flags and optional session_id in any order
    while [ "$#" -gt 1 ]; do
      case "$2" in
      --branch=*)
        TARGET_BRANCH_NAME="${2#--branch=}"
        BRANCH_FLAG_PROVIDED=true
        shift
        ;;
      --branch)
        if [ "$#" -lt 3 ]; then
          die "--branch requires a branch name"
        fi
        TARGET_BRANCH_NAME="$3"
        BRANCH_FLAG_PROVIDED=true
        shift 2
        ;;
      --integrate | -i)
        INTEGRATE=true
        shift
        ;;
      -*)
        die "unknown option: $2"
        ;;
      *)
        # Could be session_id or commit message
        if [ -z "$SESSION_ID" ] && [ -z "$COMMIT_MSG" ]; then
          # First positional argument - could be session_id or commit message
          # Check if it looks like a session_id (has timestamp pattern)
          if echo "$2" | grep -q "[0-9]\{8\}-[0-9]\{6\}"; then
            SESSION_ID="$2"
          else
            COMMIT_MSG="$2"
          fi
        elif [ -z "$COMMIT_MSG" ]; then
          # Second positional argument must be commit message
          COMMIT_MSG="$2"
        else
          die "too many arguments"
        fi
        shift
        ;;
      esac
    done

    # Validate that we have a commit message
    if [ -z "$COMMIT_MSG" ]; then
      die "finish requires a commit message"
    fi

    # Auto-detect session if not provided
    if [ -z "$SESSION_ID" ]; then
      SESSION_ID=$(auto_detect_session)
    fi

    # Validate target branch name if --branch flag was explicitly provided
    if [ "$BRANCH_FLAG_PROVIDED" = true ]; then
      validate_target_branch_name "$TARGET_BRANCH_NAME"
    fi
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  echo "▶ finishing session $SESSION_ID"
  if finish_session "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$COMMIT_MSG" "$TARGET_BRANCH_NAME" "$INTEGRATE"; then
    echo "▶ cleaning up worktree for session $SESSION_ID"
    # Remove worktree first, then close IDE
    git -C "$REPO_ROOT" worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true

    # Force close IDE/terminal window since session is ending successfully
    force_close_ide_for_session "$SESSION_ID"

    remove_session_state "$SESSION_ID"
    if [ "$INTEGRATE" = true ]; then
      echo "✅ Session finished and integrated into $BASE_BRANCH"
    else
      echo "✅ Session finished - branch ready for manual merge"
      echo "💡 Worktree cleaned up, but branch preserved for merging"
    fi
    return 0
  else
    return 1
  fi
}

# Handle cancel command
handle_cancel_command() {
  GROUP_NAME=""
  SESSION_ID=""

  # Parse arguments with --group flag support
  while [ "$#" -gt 1 ]; do
    case "$2" in
    --group=*)
      GROUP_NAME="${2#--group=}"
      shift
      ;;
    --group)
      if [ "$#" -lt 3 ]; then
        die "--group requires a group name"
      fi
      GROUP_NAME="$3"
      shift 2
      ;;
    -*)
      die "unknown option: $2"
      ;;
    *)
      # Positional argument should be session ID
      if [ -z "$SESSION_ID" ]; then
        SESSION_ID="$2"
        shift
      else
        die "too many arguments"
      fi
      ;;
    esac
  done

  # Handle group cancellation
  if [ -n "$GROUP_NAME" ]; then
    # Get all sessions in the group
    GROUP_SESSIONS=$(get_multi_session_group "$GROUP_NAME")

    if [ -z "$GROUP_SESSIONS" ]; then
      die "no multi-instance group found with name '$GROUP_NAME'"
    fi

    # Count sessions
    SESSION_COUNT=0
    for session_id in $GROUP_SESSIONS; do
      SESSION_COUNT=$((SESSION_COUNT + 1))
    done

    echo "▶ aborting $SESSION_COUNT sessions in group '$GROUP_NAME'..."

    # Cancel each session in the group
    for session_id in $GROUP_SESSIONS; do
      echo "  → cancelling session $session_id"

      # Get session info first (needed by close function)
      get_session_info "$session_id"

      # Force close IDE/terminal window since session is being cancelled
      # Capture and display close results for visibility
      echo "    ▶ closing IDE/terminal window..."
      force_close_ide_for_session "$session_id" 2>&1 | sed 's/^/    /'

      # Remove worktree and session state
      remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
      remove_session_state "$session_id"
    done

    echo "✅ cancelled all $SESSION_COUNT sessions in group '$GROUP_NAME'"
    echo "🎉 You can safely close all $(get_ide_display_name) sessions now."
    return
  fi

  # Handle single session cancellation
  if [ -z "$SESSION_ID" ]; then
    SESSION_ID=$(auto_detect_session)
  fi

  # Optimize: Get session info and immediately proceed with removal
  get_session_info "$SESSION_ID"

  # Optimize: Reduce verbose output during operations for speed
  echo "▶ aborting session $SESSION_ID"

  # Force close IDE/terminal window for this session (cancel always closes)
  force_close_ide_for_session "$SESSION_ID"

  # Optimize: Run git operations and state cleanup in parallel where possible
  # Remove worktree and branch first (heavier operations)
  remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"

  # Then clean up state files (lighter operation)
  remove_session_state "$SESSION_ID"

  echo "cancelled session $SESSION_ID"
  echo "🎉 You can safely close this $(get_ide_display_name) session now."
}

# Handle continue command for conflict resolution
handle_continue_command() {
  if [ "$#" -gt 1 ]; then
    die "continue command takes no arguments"
  fi

  # Load integration state
  if ! load_integration_state; then
    die "no integration in progress - nothing to continue"
  fi

  # Check if we're in a merge state
  if [ ! -f "$REPO_ROOT/.git/MERGE_HEAD" ]; then
    clear_integration_state
    die "no merge conflicts to resolve"
  fi

  # Check for unresolved conflicts
  if git -C "$REPO_ROOT" diff --name-only --diff-filter=U | grep -q .; then
    echo "⚠️  unresolved conflicts remain in:"
    git -C "$REPO_ROOT" diff --name-only --diff-filter=U | sed 's/^/  - /'
    echo ""
    echo "Please resolve all conflicts before running 'para continue'"
    return 1
  fi

  echo "▶ completing merge..."

  # Complete the merge
  if git -C "$REPO_ROOT" commit --no-edit; then
    echo "✅ conflicts resolved and merge completed"
    echo "✅ successfully integrated into $BASE_BRANCH"

    # Clean up feature branch
    git -C "$REPO_ROOT" branch -D "$FEATURE_BRANCH" 2>/dev/null || true
    echo "🧹 cleaned up feature branch"

    # Clear integration state
    clear_integration_state

    return 0
  else
    echo "❌ failed to complete merge"
    echo "Please check git status and resolve any remaining issues"
    return 1
  fi
}

# Handle resume command
handle_resume_command() {
  if [ "$#" -eq 1 ]; then
    # No session specified - use enhanced auto-discovery
    enhanced_resume ""
  elif [ "$#" -eq 2 ]; then
    SESSION_ID="$2"
    enhanced_resume "$SESSION_ID"
  else
    die "resume takes optionally a session name"
  fi
}

# Handle completion commands
handle_completion_command() {
  # Handle direct completion calls (e.g., para _completion_sessions)
  case "$1" in
  _completion_sessions)
    get_session_names
    return 0
    ;;
  _completion_groups)
    get_group_names
    return 0
    ;;
  _completion_branches)
    get_branch_names
    return 0
    ;;
  esac

  # Handle regular completion subcommands
  subcommand="${2:-}"
  case "$subcommand" in
  sessions | _completion_sessions)
    get_session_names
    ;;
  groups | _completion_groups)
    get_group_names
    ;;
  branches | _completion_branches)
    get_branch_names
    ;;
  generate)
    if [ "$#" -lt 3 ]; then
      echo "Usage: para completion generate [bash|zsh|fish]"
      echo ""
      echo "Generate completion script for your shell and save it to the appropriate location:"
      echo ""
      echo "For bash:"
      echo "  para completion generate bash > ~/.local/share/bash-completion/completions/para"
      echo "  # or on some systems:"
      echo "  para completion generate bash > /usr/local/etc/bash_completion.d/para"
      echo ""
      echo "For zsh:"
      echo "  para completion generate zsh > ~/.local/share/zsh/site-functions/_para"
      echo "  # or add to your fpath and then:"
      echo "  para completion generate zsh > /path/to/your/fpath/_para"
      echo ""
      echo "For fish:"
      echo "  para completion generate fish > ~/.config/fish/completions/para.fish"
      echo ""
      echo "After saving, restart your shell or source the completion file."
      return 1
    fi
    shell="$3"
    generate_completion_script "$shell"
    ;;
  *)
    echo "Usage: para completion [sessions|groups|branches|generate]"
    echo ""
    echo "Available commands:"
    echo "  sessions  - List active session names"
    echo "  groups    - List multi-instance group names"
    echo "  branches  - List local branch names"
    echo "  generate  - Generate shell completion script"
    echo ""
    echo "For shell completion setup:"
    echo "  para completion generate [bash|zsh|fish]"
    return 1
    ;;
  esac
}
