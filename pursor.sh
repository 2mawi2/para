#!/usr/bin/env sh
# pursor.sh - Parallel Cursor Workflow Helper
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
. "$LIB_DIR/pursor-config.sh"
. "$LIB_DIR/pursor-utils.sh"
. "$LIB_DIR/pursor-git.sh"
. "$LIB_DIR/pursor-session.sh"
. "$LIB_DIR/pursor-ide.sh"

# Initialize environment
need_git_repo
load_config
init_paths

# Command dispatch logic
main() {
  # Determine if we should create a new session or handle a command
  if [ "$#" -eq 0 ]; then
    # No arguments - create new session with auto-generated name
    SESSION_NAME=""
  elif [ "$#" -eq 1 ] && ! is_known_command "$1"; then
    # Single argument that's not a known command - treat as session name
    SESSION_NAME="$1"
    validate_session_name "$SESSION_NAME"
  else
    # Handle known commands
    handle_command "$@"
    return $?
  fi

  # Session creation logic
  create_new_session "$SESSION_NAME"
}

# Handle known commands
handle_command() {
  case "$1" in
    --help|-h)
      usage
      ;;

    rebase)
      handle_rebase_command "$@"
      ;;

    continue)
      handle_continue_command "$@"
      ;;

    cancel|abort)
      handle_cancel_command "$@"
      ;;

    clean)
      clean_all_sessions
      ;;

    list|ls)
      list_sessions
      ;;

    resume)
      handle_resume_command "$@"
      ;;

    *)
      usage
      ;;
  esac
}

# Handle rebase command
handle_rebase_command() {
  REBASE_MODE="squash"  # Default to squash mode
  COMMIT_MSG=""
  
  if [ "$#" -eq 1 ]; then
    die "rebase requires a commit message"
  elif [ "$#" -eq 2 ]; then
    COMMIT_MSG="$2"
    SESSION_ID=$(auto_detect_session)
  elif [ "$#" -eq 3 ] && [ "$2" = "--preserve" ]; then
    REBASE_MODE="rebase"
    COMMIT_MSG="$3"
    SESSION_ID=$(auto_detect_session)
  else
    die "rebase requires a commit message, optionally with --preserve flag"
  fi

  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  # Store the rebase mode in session state for potential continue operations
  update_session_merge_mode "$SESSION_ID" "$REBASE_MODE"

  echo "▶ rebasing session $SESSION_ID (mode: $REBASE_MODE)"
  if merge_session "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$COMMIT_MSG" "$REBASE_MODE"; then
    echo "▶ cleaning up session $SESSION_ID"
    remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$SESSION_ID"
    echo "rebase complete for session $SESSION_ID ✅"
    echo "🎉 You can safely close this Cursor session now."
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

  echo "▶ continuing rebase for session $SESSION_ID (mode: $MERGE_MODE)"
  if continue_merge "$WORKTREE_DIR" "$TEMP_BRANCH" "$BASE_BRANCH" "$MERGE_MODE"; then
    echo "▶ cleaning up session $SESSION_ID"
    remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$SESSION_ID"
    echo "rebase complete for session $SESSION_ID ✅"
    echo "🎉 You can safely close this Cursor session now."
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
  echo "🎉 You can safely close this Cursor session now."
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
  
  echo "initialized session $SESSION_ID. Use 'pursor rebase \"msg\"' to rebase or 'pursor cancel' to cancel."
}

# Execute main function
main "$@" 