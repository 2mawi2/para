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
    printf "Quick setup your IDE? [Y/n]: "
    read -r setup_choice
    case "$setup_choice" in
      n|N|no|No)
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

  config)
    handle_config_command "$@"
    ;;

  *)
    usage
    ;;
  esac
}

# Handle start command
handle_start_command() {
  if [ "$#" -eq 1 ]; then
    # No custom name provided
    SESSION_NAME=""
  elif [ "$#" -eq 2 ]; then
    # Custom name provided
    SESSION_NAME="$2"
    validate_session_name "$SESSION_NAME"
  else
    die "start takes optionally a custom session name"
  fi

  # Session creation logic
  create_new_session "$SESSION_NAME"
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

  echo "▶ resuming session $SESSION_ID"
  launch_ide "$(get_default_ide)" "$WORKTREE_DIR"
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
        n|N|no|No)
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

  # Determine base branch
  if [ -z "$BASE_BRANCH" ]; then
    BASE_BRANCH=$(get_current_branch)
  fi

  # Check for uncommitted changes and warn, but don't block
  check_uncommitted_changes

  # Create session
  SESSION_ID=$(create_session "$session_name" "$BASE_BRANCH")
  get_session_info "$SESSION_ID"

  # Launch IDE
  launch_ide "$(get_default_ide)" "$WORKTREE_DIR"

  echo "initialized session $SESSION_ID. Use 'para finish \"msg\"' to finish or 'para cancel' to cancel."
}

# Execute main function if script is run directly
if [ "$0" != "${0#*/}" ] || [ "$0" = "./para.sh" ] || [ "$0" = "para.sh" ]; then
  main "$@"
fi
