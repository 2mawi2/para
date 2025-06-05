#!/usr/bin/env sh
# para.sh - Parallel IDE Workflow Helper
# Main entry point that orchestrates the modular components

set -eu

# Determine script directory for sourcing libraries
SCRIPT_DIR="$(dirname "$0")"
case "$SCRIPT_DIR" in
/*) ;;
*) SCRIPT_DIR="$PWD/$SCRIPT_DIR" ;;
esac

# Source library modules
LIB_DIR="$SCRIPT_DIR/lib"
. "$LIB_DIR/para-config.sh"
# Early intercept: handle 'config edit' directly, bypass loading or validating the config file
if [ "$#" -ge 2 ] && [ "$1" = "config" ] && [ "$2" = "edit" ]; then
  # Inline 'config edit' without loading full config
  cmd="${EDITOR:-vi}"
  if [ -f "$CONFIG_FILE" ]; then
    # Support commands with arguments by using eval
    eval "$cmd \"$CONFIG_FILE\""
  else
    echo "No config file found. Run 'para config' to create one."
  fi
  exit 0
fi
. "$LIB_DIR/para-config-wizard.sh"
. "$LIB_DIR/para-utils.sh"
. "$LIB_DIR/para-git.sh"
. "$LIB_DIR/para-session.sh"
. "$LIB_DIR/para-ide.sh"

# Initialize environment
need_git_repo
load_config
init_paths

# Check for first run and prompt configuration
check_first_run() {
  if is_first_run; then
    echo "👋 Welcome to para!"
    echo ""

    # Check if running in non-interactive mode (CI environment)
    if [ "${PARA_NON_INTERACTIVE:-false}" = "true" ] || [ -n "${CI:-}" ] || [ -n "${GITHUB_ACTIONS:-}" ]; then
      echo "Running in non-interactive mode, using default configuration."
      create_default_config
      load_config
      return
    fi

    printf "Quick setup your IDE? [Y/n]: "
    read -r setup_choice
    case "$setup_choice" in
    n | N | no | No)
      echo "Skipped setup. Run 'para config' anytime to configure."
      create_default_config
      load_config
      ;;
    *)
      auto_setup
      load_config
      echo ""
      ;;
    esac
  fi
}

# Command dispatch logic
main() {
  # Check for first run before handling commands (but skip for config commands)
  if [ "$#" -gt 0 ] && [ "$1" != "config" ]; then
    check_first_run
  fi

  # Handle commands or show usage
  if [ "$#" -eq 0 ]; then
    # No arguments - show usage
    usage
    return 0
  else
    # Handle known commands
    handle_command "$@"
    return $?
  fi
}

# Handle known commands
handle_command() {
  case "$1" in
  --help | -h)
    usage
    ;;

  start)
    handle_start_command "$@"
    ;;

  dispatch)
    handle_dispatch_command "$@"
    ;;

  dispatch-multi)
    handle_dispatch_multi_command "$@"
    ;;

  finish)
    handle_finish_command "$@"
    ;;

  cancel | abort)
    handle_cancel_command "$@"
    ;;

  clean)
    clean_all_sessions
    ;;

  list | ls)
    list_sessions
    ;;

  resume)
    handle_resume_command "$@"
    ;;

  config)
    handle_config_command "$@"
    ;;

  *)
    usage
    ;;
  esac
}

# Handle start command (simplified - no more --prompt support)
handle_start_command() {
  SESSION_NAME=""

  # Parse arguments - only session name now
  if [ "$#" -eq 1 ]; then
    # No session name provided - generate one
    SESSION_NAME=""
  elif [ "$#" -eq 2 ]; then
    # Session name provided
    SESSION_NAME="$2"
    validate_session_name "$SESSION_NAME"
  else
    die "start takes optionally one session name (for prompts, use 'para dispatch')"
  fi

  # Session creation logic - no initial prompt
  create_new_session "$SESSION_NAME" ""
}

# Handle dispatch command - creates session with prompt
handle_dispatch_command() {
  # Validate that Claude Code is configured (wrapper mode is also supported)
  if [ "$IDE_NAME" != "claude" ]; then
    die "dispatch command only works with Claude Code. Current IDE: $(get_ide_display_name). Run 'para config' to switch to Claude Code."
  fi

  INITIAL_PROMPT=""
  SESSION_NAME=""

  if [ "$#" -eq 1 ]; then
    die "dispatch requires a prompt text"
  elif [ "$#" -eq 2 ]; then
    # Just prompt provided
    INITIAL_PROMPT="$2"
  elif [ "$#" -eq 3 ]; then
    # Session name and prompt provided
    SESSION_NAME="$2"
    INITIAL_PROMPT="$3"
    validate_session_name "$SESSION_NAME"
  else
    die "dispatch usage: 'para dispatch \"prompt\"' or 'para dispatch session-name \"prompt\"'"
  fi

  # Session creation logic with initial prompt
  create_new_session "$SESSION_NAME" "$INITIAL_PROMPT"
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

  # Skip the command name (dispatch-multi)
  shift

  # Parse arguments with --group flag support
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
  create_new_multi_session "$INSTANCE_COUNT" "$SESSION_BASE_NAME" "$INITIAL_PROMPT"
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

# Handle config command
handle_config_command() {
  if [ "$#" -eq 1 ]; then
    # No subcommand - run simple setup
    run_config_setup
  else
    case "$2" in
    show)
      show_config
      ;;
    auto)
      auto_setup
      ;;
    quick)
      # Quick setup with user confirmation
      printf "Quick Setup your IDE? [Y/n]: "
      read -r setup_choice
      case "$setup_choice" in
      n | N | no | No)
        # Cancel quick setup
        return 1
        ;;
      *)
        auto_setup
        ;;
      esac
      ;;
    wizard)
      # Alias for interactive setup wizard
      run_config_setup
      ;;
    edit)
      if [ -f "$CONFIG_FILE" ]; then
        # Use robust eval for multi-word EDITOR commands
        cmd="${EDITOR:-vi}"
        eval "$cmd \"$CONFIG_FILE\""
      else
        echo "No config file found. Run 'para config' to create one."
      fi
      ;;
    *)
      # Handle unknown subcommands
      echo "Unknown config command: $2"
      echo "Usage: para config [show|auto|quick|wizard|edit]"
      echo ""
      echo "  para config         # Interactive setup"
      echo "  para config show    # Show current settings"
      echo "  para config auto    # Auto-detect IDE"
      echo "  para config quick   # Quick auto-detect with confirmation"
      echo "  para config wizard  # Interactive setup wizard"
      echo "  para config edit    # Edit config file"
      ;;
    esac
  fi
}

# Create new session
create_new_session() {
  session_name="$1"
  initial_prompt="$2"

  # Determine base branch
  if [ -z "$BASE_BRANCH" ]; then
    BASE_BRANCH=$(get_current_branch)
  fi

  # Check for uncommitted changes and warn, but don't block
  check_uncommitted_changes

  # Create session
  SESSION_ID=$(create_session "$session_name" "$BASE_BRANCH")
  get_session_info "$SESSION_ID"

  # Store initial prompt in session state if provided
  if [ -n "$initial_prompt" ]; then
    save_session_prompt "$SESSION_ID" "$initial_prompt"
  fi

  # Launch IDE with optional initial prompt
  launch_ide "$(get_default_ide)" "$WORKTREE_DIR" "$initial_prompt"

  echo "initialized session $SESSION_ID. Use 'para finish \"msg\"' to finish or 'para cancel' to cancel."
}

# Create new multi-instance session group
create_new_multi_session() {
  instance_count="$1"
  session_base_name="$2"
  initial_prompt="$3"

  # Determine base branch
  if [ -z "$BASE_BRANCH" ]; then
    BASE_BRANCH=$(get_current_branch)
  fi

  # Check for uncommitted changes and warn, but don't block
  check_uncommitted_changes

  # Generate group name if not provided
  if [ -z "$session_base_name" ]; then
    session_base_name="multi-$(generate_timestamp)"
  fi

  # Create multiple sessions
  echo "▶ creating $instance_count instances for group '$session_base_name'..."
  SESSION_IDS=$(create_multi_session "$session_base_name" "$instance_count" "$BASE_BRANCH")

  # Store initial prompt for each session if provided
  if [ -n "$initial_prompt" ]; then
    for session_id in $SESSION_IDS; do
      save_session_prompt "$session_id" "$initial_prompt"
    done
  fi

  # Launch IDEs for all instances
  launch_multi_ide "$(get_default_ide)" "$SESSION_IDS" "$initial_prompt"

  # Show summary
  echo ""
  echo "✅ Initialized group '$session_base_name' with $instance_count instances:"
  for session_id in $SESSION_IDS; do
    echo "  → $session_id"
  done
  echo ""
  echo "💡 Use 'para list' to see all instances"
  echo "💡 Use 'para finish \"msg\"' in any instance to finish that session"
  echo "💡 Use 'para cancel --group $session_base_name' to cancel all instances"
}

# Execute main function if script is run directly
if [ "$0" != "${0#*/}" ] || [ "$0" = "./para.sh" ] || [ "$0" = "para.sh" ]; then
  main "$@"
fi
