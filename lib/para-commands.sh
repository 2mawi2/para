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
    --dangerously-skip-permissions)
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

  # Parse arguments
  shift # Remove 'dispatch'
  
  # Collect positional arguments separately from flags
  positional_args=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --dangerously-skip-permissions)
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
      # Only prompt provided
      INITIAL_PROMPT="$positional_args"
    elif [ "$arg_count" -eq 2 ]; then
      # Session name and prompt provided
      SESSION_NAME=$(echo "$positional_args" | cut -d'|' -f1)
      INITIAL_PROMPT=$(echo "$positional_args" | cut -d'|' -f2)
      validate_session_name "$SESSION_NAME"
    else
      die "too many arguments"
    fi
  fi

  # Validate required arguments
  if [ -z "$INITIAL_PROMPT" ]; then
    die "dispatch requires a prompt text"
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

  # Skip the command name (dispatch-multi)
  shift

  # Parse arguments with --group and --dangerously-skip-permissions flag support
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
    --dangerously-skip-permissions)
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
      # Second positional argument should be prompt
      elif [ -z "$INITIAL_PROMPT" ]; then
        INITIAL_PROMPT="$1"
        shift
      else
        die "too many arguments"
      fi
      ;;
    esac
  done

  # Validate required arguments
  if [ -z "$INSTANCE_COUNT" ]; then
    die "dispatch-multi usage: 'para dispatch-multi N \"prompt\"' or 'para dispatch-multi N --group name \"prompt\"'"
  fi

  if [ -z "$INITIAL_PROMPT" ]; then
    die "dispatch-multi requires a prompt text"
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

  # Parse arguments supporting --branch flag
  if [ "$#" -eq 1 ]; then
    die "finish requires a commit message"
  else
    # Parse all arguments to handle flags in any order
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
      -*)
        die "unknown option: $2"
        ;;
      *)
        # This must be the commit message
        if [ -n "$COMMIT_MSG" ]; then
          die "multiple commit messages provided"
        fi
        COMMIT_MSG="$2"
        shift
        ;;
      esac
    done

    # Validate that we have a commit message
    if [ -z "$COMMIT_MSG" ]; then
      die "finish requires a commit message"
    fi

    # Validate target branch name if --branch flag was explicitly provided
    if [ "$BRANCH_FLAG_PROVIDED" = true ]; then
      validate_target_branch_name "$TARGET_BRANCH_NAME"
    fi

    SESSION_ID=$(auto_detect_session)
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  echo "▶ finishing session $SESSION_ID"
  if finish_session "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$COMMIT_MSG" "$TARGET_BRANCH_NAME"; then
    echo "▶ cleaning up worktree for session $SESSION_ID"
    # Remove worktree and session state, but keep the branch for manual merging
    git -C "$REPO_ROOT" worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true
    remove_session_state "$SESSION_ID"
    echo "✅ Session finished - branch ready for manual merge"
    echo "💡 Worktree cleaned up, but branch preserved for merging"
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
      
      # Get session info and remove
      get_session_info "$session_id"
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
  
  # Optimize: Run git operations and state cleanup in parallel where possible
  # Remove worktree and branch first (heavier operations)
  remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
  
  # Then clean up state files (lighter operation)
  remove_session_state "$SESSION_ID"
  
  echo "cancelled session $SESSION_ID"
  echo "🎉 You can safely close this $(get_ide_display_name) session now."
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