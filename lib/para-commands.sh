#!/usr/bin/env sh
# Command handlers for para

# Common argument parsing helper
parse_common_args() {
  command_name="$1"
  shift

  # Initialize output variables
  positional_args=""
  skip_permissions=false
  file_path=""

  # Command-specific parsing
  case "$command_name" in
  "start")
    while [ "$#" -gt 0 ]; do
      case "$1" in
      --dangerously-skip-permissions | -d)
        skip_permissions=true
        shift
        ;;
      -*)
        die_invalid_args "unknown option: $1"
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
    ;;
  "dispatch")
    while [ "$#" -gt 0 ]; do
      case "$1" in
      --file=*)
        file_path="${1#--file=}"
        shift
        ;;
      --file | -f)
        if [ "$#" -lt 2 ]; then
          die_invalid_args "--file requires a file path"
        fi
        file_path="$2"
        shift 2
        ;;
      --dangerously-skip-permissions | -d)
        skip_permissions=true
        shift
        ;;
      -*)
        die_invalid_args "unknown option: $1"
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
    ;;
  esac

  # Count positional arguments
  if [ -n "$positional_args" ]; then
    arg_count=$(echo "$positional_args" | tr '|' '\n' | wc -l)
  else
    arg_count=0
  fi
}

# Handle start command (simplified - no more --prompt support)
handle_start_command() {
  shift # Remove 'start'

  # Parse arguments using common helper
  parse_common_args "start" "$@"

  SESSION_NAME=""
  SKIP_PERMISSIONS="$skip_permissions"

  # Process positional arguments
  if [ "$arg_count" -eq 1 ]; then
    # Only session name provided
    SESSION_NAME="$positional_args"
    validate_session_name "$SESSION_NAME"
  elif [ "$arg_count" -gt 1 ]; then
    die_invalid_args "too many arguments"
  fi

  # Session creation logic - no initial prompt
  create_new_session "$SESSION_NAME" "" "$SKIP_PERMISSIONS"
}

# Handle dispatch command - creates session with prompt
handle_dispatch_command() {
  # Validate that Claude Code is configured (wrapper mode is also supported)
  if [ "$IDE_NAME" != "claude" ]; then
    die_config_invalid "dispatch command only works with Claude Code. Current IDE: $(get_ide_display_name). Run 'para config' to switch to Claude Code."
  fi

  shift # Remove 'dispatch'

  # Parse arguments using common helper
  parse_common_args "dispatch" "$@"

  INITIAL_PROMPT=""
  SESSION_NAME=""
  SKIP_PERMISSIONS="$skip_permissions"
  FILE_PATH="$file_path"

  # Process positional arguments
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
  elif [ "$arg_count" -gt 2 ]; then
    die_invalid_args "too many arguments"
  fi

  # Handle file input if provided
  if [ -n "$FILE_PATH" ]; then
    INITIAL_PROMPT=$(read_file_content "$FILE_PATH")
    if [ -z "$INITIAL_PROMPT" ]; then
      die_file_not_found "file is empty: $FILE_PATH"
    fi
  fi

  # Validate required arguments
  if [ -z "$INITIAL_PROMPT" ]; then
    die_invalid_args "dispatch requires a prompt text or file path"
  fi

  # Session creation logic with initial prompt
  create_new_session "$SESSION_NAME" "$INITIAL_PROMPT" "$SKIP_PERMISSIONS"
}

# Handle integrate command - equivalent to finish with --integrate flag
handle_integrate_command() {
  # Simply delegate to finish command with --integrate flag added
  if [ "$#" -eq 1 ]; then
    die_invalid_args "integrate requires a commit message"
  fi

  # Replace 'integrate' with 'finish' and add --integrate flag
  shift # Remove 'integrate'
  handle_finish_command "finish" "$@" "--integrate"
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
    die_invalid_args "finish requires a commit message"
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
          die_invalid_args "--branch requires a branch name"
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
        die_invalid_args "unknown option: $2"
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
          die_invalid_args "too many arguments"
        fi
        shift
        ;;
      esac
    done

    # Validate that we have a commit message
    if [ -z "$COMMIT_MSG" ]; then
      die_invalid_args "finish requires a commit message"
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
  [ -d "$WORKTREE_DIR" ] || die_session_not_found "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  echo "▶ finishing session $SESSION_ID"
  if finish_session "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$COMMIT_MSG" "$TARGET_BRANCH_NAME" "$INTEGRATE"; then
    echo "▶ cleaning up worktree for session $SESSION_ID"
    # Remove worktree first, then close IDE
    git -C "$REPO_ROOT" worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true

    # Force close IDE/terminal window since session is ending successfully
    force_close_ide_for_session "$SESSION_ID"

    # Clean up feature branch after successful integration (now that worktree is removed)
    if [ "$INTEGRATE" = true ]; then
      # Get all branches and find the one that matches our pattern
      # Either the original TEMP_BRANCH or the TARGET_BRANCH_NAME (with possible suffix)
      cleanup_branch=""
      if [ -n "$TARGET_BRANCH_NAME" ]; then
        # Look for the target branch name (could have suffix like -1, -2, etc.)
        cleanup_branch=$(git -C "$REPO_ROOT" branch | grep -E "^\s*${TARGET_BRANCH_NAME}(-[0-9]+)?$" | head -1 | sed 's/^[* ]*//') || true
      fi

      # If no custom branch found, try the original temp branch
      if [ -z "$cleanup_branch" ] && git -C "$REPO_ROOT" rev-parse --verify "$TEMP_BRANCH" >/dev/null 2>&1; then
        cleanup_branch="$TEMP_BRANCH"
      fi

      # Now we can safely delete the feature branch
      if [ -n "$cleanup_branch" ] && git -C "$REPO_ROOT" branch -D "$cleanup_branch" 2>/dev/null; then
        echo "🧹 cleaned up feature branch"
      fi
    fi

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
  SESSION_ID=""

  # Parse arguments - only accept session ID
  if [ "$#" -gt 2 ]; then
    die_invalid_args "too many arguments"
  elif [ "$#" -eq 2 ]; then
    case "$2" in
    -*)
      die_invalid_args "unknown option: $2"
      ;;
    *)
      SESSION_ID="$2"
      ;;
    esac
  fi

  # Auto-detect session if not provided
  if [ -z "$SESSION_ID" ]; then
    SESSION_ID=$(auto_detect_session)
  fi

  # Get session info and proceed with removal
  ensure_session_loaded "$SESSION_ID"

  echo "▶ aborting session $SESSION_ID"

  # Force close IDE/terminal window for this session (cancel always closes)
  force_close_ide_for_session "$SESSION_ID"

  # Cancel session (moves to archive)
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"

  # Clean up state files
  remove_session_state "$SESSION_ID"

  echo "cancelled session $SESSION_ID"
  echo "💡 Session moved to archive. Use 'para recover $SESSION_ID' to restore."
  echo "🎉 You can safely close this $(get_ide_display_name) session now."
}

# Handle recover command
handle_recover_command() {
  SESSION_NAME=""

  # Parse arguments
  if [ "$#" -eq 1 ]; then
    # No session specified - list available sessions in archive
    list_archive_sessions
    return 0
  elif [ "$#" -eq 2 ]; then
    SESSION_NAME="$2"
  else
    die "recover takes optionally a session name"
  fi

  # Find and recover the specified session
  recover_archive_session "$SESSION_NAME"
}

# Handle clean command
handle_clean_command() {
  CLEAN_BACKUPS=false

  # Parse arguments
  while [ "$#" -gt 1 ]; do
    case "$2" in
    --backups)
      CLEAN_BACKUPS=true
      shift
      ;;
    -*)
      die "unknown option: $2"
      ;;
    *)
      die "clean command does not accept positional arguments"
      ;;
    esac
  done

  if [ "$CLEAN_BACKUPS" = true ]; then
    clean_archive_sessions
  else
    clean_all_sessions
  fi
}

# Handle continue command for conflict resolution
handle_continue_command() {
  if [ "$#" -gt 1 ]; then
    die_invalid_args "continue command takes no arguments"
  fi

  # Load integration state
  if ! load_integration_state; then
    die_repo_state "no integration in progress - nothing to continue"
  fi

  # Check if we're in a rebase state
  if [ ! -d "$REPO_ROOT/.git/rebase-merge" ] && [ ! -d "$REPO_ROOT/.git/rebase-apply" ]; then
    clear_integration_state
    die_repo_state "no rebase conflicts to resolve"
  fi

  # Check for unresolved conflicts
  if git -C "$REPO_ROOT" diff --name-only --diff-filter=U | grep -q .; then
    echo "⚠️  unresolved conflicts remain in:"
    git -C "$REPO_ROOT" diff --name-only --diff-filter=U | sed 's/^/  - /'
    echo ""
    echo "Please resolve all conflicts before running 'para continue'"
    return 1
  fi

  echo "▶ continuing rebase..."

  # Continue the rebase
  if git -C "$REPO_ROOT" rebase --continue; then
    # Rebase completed successfully, now fast-forward merge
    echo "▶ completing integration..."

    # Get the current branch (should be the temporary rebase branch)
    current_branch=$(git -C "$REPO_ROOT" symbolic-ref --short HEAD)
    git -C "$REPO_ROOT" checkout "$BASE_BRANCH" >/dev/null 2>&1 || die_git_operation "failed to checkout $BASE_BRANCH"

    if git -C "$REPO_ROOT" merge --ff-only "$current_branch"; then
      echo "✅ conflicts resolved and rebase completed"
      echo "✅ successfully integrated into $BASE_BRANCH"

      # Clean up feature branch and temporary branch since rebase was successful
      git -C "$REPO_ROOT" branch -D "$FEATURE_BRANCH" 2>/dev/null || true
      git -C "$REPO_ROOT" branch -D "$current_branch" 2>/dev/null || true
      echo "🧹 cleaned up feature branch"

      # Clear integration state
      clear_integration_state

      return 0
    else
      echo "❌ failed to complete fast-forward merge after rebase"
      echo "Please check git status and resolve any remaining issues"
      return 1
    fi
  else
    echo "❌ failed to continue rebase"
    echo "Please check git status and resolve any remaining issues"
    echo "To abort: git rebase --abort"
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
    die_invalid_args "resume takes optionally a session name"
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
    echo "Usage: para completion [sessions|branches|generate]"
    echo ""
    echo "Available commands:"
    echo "  sessions  - List active session names"
    echo "  branches  - List local branch names"
    echo "  generate  - Generate shell completion script"
    echo ""
    echo "For shell completion setup:"
    echo "  para completion generate [bash|zsh|fish]"
    return 1
    ;;
  esac
}
