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
. "$LIB_DIR/para-recovery.sh"

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

  finish)
    handle_finish_command "$@"
    ;;

  continue)
    handle_continue_command "$@"
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

  recover)
    handle_recover_command "$@"
    ;;

  history)
    handle_history_command "$@"
    ;;

  clean-history)
    handle_clean_history_command "$@"
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

  # Auto-cleanup old history entries
  auto_cleanup_history

  # Session creation logic - no initial prompt
  create_new_session "$SESSION_NAME" ""
}

# Handle dispatch command - creates session with prompt
handle_dispatch_command() {
  # Validate that Claude Code is configured
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

  # Auto-cleanup old history entries
  auto_cleanup_history

  # Session creation logic with initial prompt
  create_new_session "$SESSION_NAME" "$INITIAL_PROMPT"
}

# Handle finish command
handle_finish_command() {
  REBASE_MODE="squash" # Default to squash mode
  COMMIT_MSG=""

  if [ "$#" -eq 1 ]; then
    die "finish requires a commit message"
  elif [ "$#" -eq 2 ]; then
    COMMIT_MSG="$2"
    SESSION_ID=$(auto_detect_session)
  elif [ "$#" -eq 3 ] && [ "$2" = "--preserve" ]; then
    REBASE_MODE="rebase"
    COMMIT_MSG="$3"
    SESSION_ID=$(auto_detect_session)
  else
    die "finish requires a commit message, optionally with --preserve flag"
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  # Store the rebase mode in session state for potential continue operations
  update_session_merge_mode "$SESSION_ID" "$REBASE_MODE"

  echo "▶ finishing session $SESSION_ID (mode: $REBASE_MODE)"
  if merge_session "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$COMMIT_MSG" "$REBASE_MODE"; then
    echo "▶ cleaning up session $SESSION_ID"
    # Save to history before cleanup
    save_session_to_history "$SESSION_ID" "finished" "$COMMIT_MSG" "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$REBASE_MODE"
    remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$SESSION_ID"
    echo "finish complete for session $SESSION_ID ✅"
    echo "🎉 You can safely close this $(get_ide_display_name) session now."
    return 0
  else
    return 1
  fi
}

# Handle continue command
handle_continue_command() {
  if [ "$#" -eq 1 ]; then
    SESSION_ID=$(auto_detect_session)
  elif [ "$#" -eq 2 ]; then
    SESSION_ID="$2"
  else
    die "continue takes optionally a session ID"
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  echo "▶ continuing finish for session $SESSION_ID (mode: $MERGE_MODE)"
  if continue_merge "$WORKTREE_DIR" "$TEMP_BRANCH" "$BASE_BRANCH" "$MERGE_MODE"; then
    echo "▶ cleaning up session $SESSION_ID"
    # Save to history before cleanup
    save_session_to_history "$SESSION_ID" "finished" "" "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$MERGE_MODE"
    remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$SESSION_ID"
    echo "finish complete for session $SESSION_ID ✅"
    echo "🎉 You can safely close this $(get_ide_display_name) session now."
    return 0
  else
    return 1
  fi
}

# Handle cancel command
handle_cancel_command() {
  if [ "$#" -eq 1 ]; then
    SESSION_ID=$(auto_detect_session)
  elif [ "$#" -eq 2 ]; then
    SESSION_ID="$2"
  else
    die "cancel takes optionally a session ID"
  fi

  get_session_info "$SESSION_ID"
  echo "▶ aborting session $SESSION_ID; removing $WORKTREE_DIR & deleting $TEMP_BRANCH"
  # Save to history before cleanup
  save_session_to_history "$SESSION_ID" "cancelled" "" "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$MERGE_MODE"
  remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID"
  echo "cancelled session $SESSION_ID"
  echo "🎉 You can safely close this $(get_ide_display_name) session now."
}

# Handle resume command
handle_resume_command() {
  if [ "$#" -eq 1 ]; then
    die "resume requires a session name"
  elif [ "$#" -eq 2 ]; then
    SESSION_ID="$2"
  else
    die "resume takes a session name"
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  # Load initial prompt if it exists for this session
  STORED_PROMPT=$(load_session_prompt "$SESSION_ID")

  echo "▶ resuming session $SESSION_ID"
  launch_ide "$(get_default_ide)" "$WORKTREE_DIR" "$STORED_PROMPT"
}

# Handle recover command
handle_recover_command() {
  if [ "$#" -eq 1 ]; then
    die "recover requires a session name"
  elif [ "$#" -eq 2 ]; then
    SESSION_ID="$2"
  else
    die "recover takes a session name"
  fi

  recover_session "$SESSION_ID"
}

# Handle history command
handle_history_command() {
  if [ "$#" -gt 1 ]; then
    die "history takes no arguments"
  fi

  list_session_history
}

# Handle clean history command
handle_clean_history_command() {
  if [ "$#" -eq 1 ]; then
    # Clean all history
    clean_session_history
  elif [ "$#" -eq 3 ] && [ "$2" = "--older-than" ]; then
    # Clean history older than specified days
    older_than_days="$3"
    if ! echo "$older_than_days" | grep -q '^[0-9]\+$'; then
      die "clean-history --older-than requires a number of days"
    fi
    clean_session_history "$older_than_days"
  else
    die "clean-history usage: clean-history [--older-than DAYS]"
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

# Execute main function if script is run directly
if [ "$0" != "${0#*/}" ] || [ "$0" = "./para.sh" ] || [ "$0" = "para.sh" ]; then
  main "$@"
fi
