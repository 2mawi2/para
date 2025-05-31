#!/usr/bin/env sh
# -----------------------------------------------------------------------------
# pursor.sh
# -----------------------------------------------------------------------------
# Portable POSIX‑sh helper that launches **multiple ephemeral** Cursor IDE sessions on
# temporary Git worktrees and then merges (or discards) them individually.
#
#   pursor                    # create new session & open Cursor
#   pursor list               # list all active sessions (alias: ls)
#   pursor merge "message"    # merge session back & clean
#   pursor continue           # continue merge after resolving conflicts
#   pursor cancel             # cancel session & clean (alias: abort)
#   pursor clean              # cancel ALL sessions & clean everything
# -----------------------------------------------------------------------------
# HOW IT WORKS
#   • **Create**   – create branch pc/<timestamp>, worktree in subtrees/, record
#                    state, open Cursor detached.
#   • **List**     – show all active parallel sessions.
#   • **Merge**    – auto-stage all changes, commit, rebase temp → base, merge locally, prune.
#   • **Continue** – auto-stage resolved conflicts, continue merge after conflicts are resolved.
#   • **Cancel**   – force‑remove worktree & branch, kill Cursor (best‑effort).
#   • **Clean**    – cancel ALL sessions and clean everything.
# -----------------------------------------------------------------------------
# CONFIGURATION (override in env before running)
BASE_BRANCH="${BASE_BRANCH:-}"          # empty → detect current branch on init
SUBTREES_DIR_NAME="${SUBTREES_DIR_NAME:-subtrees}"
STATE_DIR_NAME="${STATE_DIR_NAME:-.pursor_state}"
CURSOR_CMD="${CURSOR_CMD:-cursor}"     # override if cursor CLI named differently
# -----------------------------------------------------------------------------
set -eu

# --- helpers -------------------------------------------------------------
usage() {
  cat >&2 <<EOF
Usage:
  pursor                    # create new session & open Cursor
  pursor merge "message"    # merge current session with commit message
  pursor list               # list all active sessions (alias: ls)
  pursor continue           # continue merge after resolving conflicts
  pursor cancel             # cancel current session (alias: abort)
  pursor clean              # clean up all sessions

Examples:
  pursor                    # start new parallel session
  pursor merge "Add new feature"  # merge with commit message
  pursor continue           # resume after fixing conflicts
  pursor cancel             # discard current session
EOF
  exit 1
}

die() {
  echo "pursor: $*" >&2
  exit 1
}

need_git_repo() {
  # First check if we're in a git repository at all
  git rev-parse --git-dir >/dev/null 2>&1 || die "not in a Git repository"
  
  # Get the common git directory (works for both main repo and worktrees)
  GIT_COMMON_DIR=$(git rev-parse --git-common-dir 2>/dev/null)
  
  # If git-common-dir ends with .git, the parent is the repo root
  # If it ends with .git/worktrees/<name>, we need to go up more levels
  if echo "$GIT_COMMON_DIR" | grep -q "\.git/worktrees/"; then
    # We're in a worktree, git-common-dir points to main .git
    REPO_ROOT=$(dirname "$GIT_COMMON_DIR")
  elif echo "$GIT_COMMON_DIR" | grep -q "\.git$"; then
    # We're in main repo, git-common-dir is the .git directory
    REPO_ROOT=$(dirname "$GIT_COMMON_DIR")
  else
    # Fallback to the traditional method
    REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || die "could not find repository root"
  fi
  
  # Ensure REPO_ROOT is absolute
  case "$REPO_ROOT" in
    /*) ;;
    *) REPO_ROOT="$PWD/$REPO_ROOT" ;;
  esac
}

setup_gitignore() {
  GIT_EXCLUDE_FILE="$REPO_ROOT/.git/info/exclude"
  SUBTREES_GITIGNORE="$SUBTREES_DIR/.gitignore"
  
  # Entries to ensure are ignored
  SUBTREES_ENTRY="$SUBTREES_DIR_NAME/"
  STATE_ENTRY="$STATE_DIR_NAME/"
  
  # Setup git exclude file (local to repository, not tracked)
  # This is non-intrusive and doesn't modify user's .gitignore
  if [ ! -f "$GIT_EXCLUDE_FILE" ]; then
    echo "▶ creating git exclude file for pursor directories"
    mkdir -p "$(dirname "$GIT_EXCLUDE_FILE")"
    cat > "$GIT_EXCLUDE_FILE" <<EOF
# pursor - parallel cursor sessions (local excludes)
$SUBTREES_ENTRY
$STATE_ENTRY
EOF
  else
    # Check and add entries if missing
    NEEDS_UPDATE=0
    if ! grep -q "^$SUBTREES_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null; then
      NEEDS_UPDATE=1
    fi
    if ! grep -q "^$STATE_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null; then
      NEEDS_UPDATE=1
    fi
    
    if [ "$NEEDS_UPDATE" -eq 1 ]; then
      echo "▶ updating git exclude file with pursor entries"
      {
        echo ""
        echo "# pursor - parallel cursor sessions (local excludes)"
        grep -q "^$SUBTREES_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$SUBTREES_ENTRY"
        grep -q "^$STATE_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$STATE_ENTRY"
      } >> "$GIT_EXCLUDE_FILE"
    fi
  fi
  
  # Setup subtrees directory .gitignore to ignore all contents
  if [ ! -f "$SUBTREES_GITIGNORE" ]; then
    echo "▶ creating .gitignore in subtrees directory"
    cat > "$SUBTREES_GITIGNORE" <<EOF
# Ignore all pursor worktree contents
*
!.gitignore
EOF
  fi
}

list_sessions() {
  if [ ! -d "$STATE_DIR" ]; then
    echo "No active parallel sessions."
    return 0
  fi
  
  SESSIONS_FOUND=0
  for state_file in "$STATE_DIR"/*.state; do
    [ -f "$state_file" ] || continue
    SESSIONS_FOUND=1
    session_id=$(basename "$state_file" .state)
    IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH < "$state_file"
    echo "Session: $session_id"
    echo "  Branch: $TEMP_BRANCH"
    echo "  Worktree: $WORKTREE_DIR"
    echo "  Base: $BASE_BRANCH"
    if [ -d "$WORKTREE_DIR" ]; then
      cd "$WORKTREE_DIR"
      if git status --porcelain | grep -q "^UU\|^AA\|^DD"; then
        echo "  Status: ⚠️  Has merge conflicts"
      elif git diff --quiet --exit-code --cached --ignore-submodules --; then
        echo "  Status: Clean"
      else
        echo "  Status: Has uncommitted changes"
      fi
      cd "$REPO_ROOT"
    else
      echo "  Status: ⚠️  Worktree missing"
    fi
    echo ""
  done
  
  if [ "$SESSIONS_FOUND" -eq 0 ]; then
    echo "No active parallel sessions."
  fi
}

get_session_info() {
  SESSION_ID="$1"
  STATE_FILE="$STATE_DIR/$SESSION_ID.state"
  [ -f "$STATE_FILE" ] || die "session '$SESSION_ID' not found"
  IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH < "$STATE_FILE"
}

auto_detect_session() {
  # First, try to detect session from current working directory
  # Check if we're in a worktree that matches our pattern
  CURRENT_DIR="$PWD"
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/pc/[0-9]\{8\}-[0-9]\{6\}"; then
    # Extract the timestamp from the path
    TIMESTAMP=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/pc/\([0-9]\{8\}-[0-9]\{6\}\).*|\1|p")
    if [ -n "$TIMESTAMP" ]; then
      CONTEXT_SESSION_ID="pc-$TIMESTAMP"
      # Verify this session actually exists
      if [ -f "$STATE_DIR/$CONTEXT_SESSION_ID.state" ]; then
        echo "$CONTEXT_SESSION_ID"
        return 0
      fi
    fi
  fi

  # Fallback to original logic if context detection failed
  if [ ! -d "$STATE_DIR" ]; then
    die "no active sessions found"
  fi
  
  SESSIONS_COUNT=0
  FOUND_SESSION=""
  for state_file in "$STATE_DIR"/*.state; do
    [ -f "$state_file" ] || continue
    SESSIONS_COUNT=$((SESSIONS_COUNT + 1))
    FOUND_SESSION=$(basename "$state_file" .state)
  done
  
  if [ "$SESSIONS_COUNT" -eq 0 ]; then
    die "no active sessions found"
  elif [ "$SESSIONS_COUNT" -gt 1 ]; then
    echo "Multiple sessions active. Please specify session ID:" >&2
    list_sessions >&2
    exit 1
  fi
  
  echo "$FOUND_SESSION"
}

continue_merge() {
  SESSION_ID="$1"
  get_session_info "$SESSION_ID"
  [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

  cd "$WORKTREE_DIR"
  
  # Check if we're in the middle of a rebase (works for worktrees)
  GIT_DIR=$(git rev-parse --git-dir)
  if [ -d "$GIT_DIR/rebase-merge" ] || [ -d "$GIT_DIR/rebase-apply" ]; then
    # Check for remaining conflict markers in any files
    if git diff --check 2>/dev/null; then
      # No conflict markers found, auto-stage resolved files
      echo "▶ auto-staging resolved conflicts in session $SESSION_ID"
      git add -u || die "failed to stage resolved conflicts"
    else
      echo "❌ There are still unresolved conflicts in session $SESSION_ID:" >&2
      echo "Files with conflict markers:" >&2
      git diff --check 2>&1 || true
      exit 1
    fi
    
    echo "▶ continuing rebase for session $SESSION_ID"
    # Use GIT_EDITOR=true to avoid hanging on editor prompts
    if ! GIT_EDITOR=true git rebase --continue; then
      echo "❌ rebase continue failed for session $SESSION_ID – check for remaining conflicts" >&2
      exit 1
    fi
    echo "✅ rebase completed for session $SESSION_ID"
  else
    echo "ℹ️  No rebase in progress for session $SESSION_ID"
  fi
  
  cd "$REPO_ROOT"
  
  # Now continue with the merge process
  echo "▶ merging session $SESSION_ID into $BASE_BRANCH"
  git -C "$REPO_ROOT" checkout "$BASE_BRANCH"
  if ! git -C "$REPO_ROOT" merge --ff-only "$TEMP_BRANCH" 2>/dev/null; then
    echo "▶ using non-fast-forward merge"
    COMMIT_MSG="Merge session $SESSION_ID after resolving conflicts"
    if ! git -C "$REPO_ROOT" merge --no-ff -m "$COMMIT_MSG" "$TEMP_BRANCH"; then
      echo "❌ merge conflicts – resolve them and complete the merge manually, then run cleanup:" >&2
      echo "   git worktree remove '$WORKTREE_DIR'" >&2
      echo "   git branch -D '$TEMP_BRANCH'" >&2
      echo "   rm -f '$STATE_DIR/$SESSION_ID.state'" >&2
      exit 1
    fi
  fi

  echo "▶ cleaning up session $SESSION_ID"
  git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR"
  git -C "$REPO_ROOT" branch -D "$TEMP_BRANCH"
  rm -f "$STATE_DIR/$SESSION_ID.state"
  echo "merge complete for session $SESSION_ID ✅"
}

clean_all_sessions() {
  if [ ! -d "$STATE_DIR" ]; then
    echo "No active parallel sessions to clean."
    return 0
  fi
  
  SESSIONS_FOUND=0
  CLEANED_COUNT=0
  
  echo "▶ cleaning up all active sessions..."
  
  for state_file in "$STATE_DIR"/*.state; do
    [ -f "$state_file" ] || continue
    SESSIONS_FOUND=1
    
    session_id=$(basename "$state_file" .state)
    IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH < "$state_file"
    
    echo "  → cleaning session $session_id"
    
    # Remove worktree (force to handle any state)
    if [ -d "$WORKTREE_DIR" ]; then
      git -C "$REPO_ROOT" worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true
    fi
    
    # Delete branch
    git -C "$REPO_ROOT" branch -D "$TEMP_BRANCH" 2>/dev/null || true
    
    # Remove state file
    rm -f "$state_file"
    
    CLEANED_COUNT=$((CLEANED_COUNT + 1))
  done
  
  # Clean up state directory if empty
  if [ -d "$STATE_DIR" ]; then
    rmdir "$STATE_DIR" 2>/dev/null || true
  fi
  
  if [ "$SESSIONS_FOUND" -eq 0 ]; then
    echo "No active parallel sessions to clean."
  else
    echo "✅ cleaned up $CLEANED_COUNT session(s)"
  fi
}

# --- MAIN ---------------------------------------------------------------
# Always find repository root first, regardless of where script is run from
need_git_repo
STATE_DIR="$REPO_ROOT/$STATE_DIR_NAME"
SUBTREES_DIR="$REPO_ROOT/$SUBTREES_DIR_NAME"

# no args  → INIT ---------------------------------------------------------
if [ "$#" -eq 0 ]; then
  # determine base branch (current branch) unless overridden
  if [ -z "$BASE_BRANCH" ]; then
    BASE_BRANCH=$(git -C "$REPO_ROOT" symbolic-ref --quiet --short HEAD) || die "detached HEAD; set BASE_BRANCH env"
  fi

  # ensure working tree is clean before creating new session
  if ! git -C "$REPO_ROOT" diff --quiet --exit-code; then
    echo "❌ You have uncommitted changes in the working tree:" >&2
    git -C "$REPO_ROOT" status --porcelain >&2
    echo "Please commit or stash your changes before creating a new session." >&2
    exit 1
  fi
  
  if ! git -C "$REPO_ROOT" diff --quiet --exit-code --cached; then
    echo "❌ You have staged changes in the working tree:" >&2
    git -C "$REPO_ROOT" status --porcelain >&2
    echo "Please commit your staged changes before creating a new session." >&2
    exit 1
  fi

  TS=$(date +%Y%m%d-%H%M%S)
  SESSION_ID="pc-$TS"
  TEMP_BRANCH="pc/$TS"
  WORKTREE_DIR="$SUBTREES_DIR/$TEMP_BRANCH"

  echo "▶ creating session $SESSION_ID: branch $TEMP_BRANCH and worktree $WORKTREE_DIR (base $BASE_BRANCH)"
  mkdir -p "$SUBTREES_DIR"
  mkdir -p "$STATE_DIR"
  
  # Setup gitignore files to ensure pursor directories are ignored
  setup_gitignore
  
  git -C "$REPO_ROOT" worktree add -b "$TEMP_BRANCH" "$WORKTREE_DIR" HEAD || die "git worktree add failed"

  echo "$TEMP_BRANCH|$WORKTREE_DIR|$BASE_BRANCH" > "$STATE_DIR/$SESSION_ID.state"

  if command -v "$CURSOR_CMD" >/dev/null 2>&1; then
    echo "▶ launching Cursor for session $SESSION_ID..."
    "$CURSOR_CMD" "$WORKTREE_DIR" &
  else
    echo "Cursor CLI not found; open $WORKTREE_DIR manually." >&2
  fi
  echo "initialized session $SESSION_ID. Use 'pursor merge \"msg\"' to merge or 'pursor cancel' to cancel."
  exit 0
fi

# arg parsing ------------------------------------------------------------
case "$1" in
  merge)
    if [ "$#" -eq 1 ]; then
      die "merge requires a commit message"
    elif [ "$#" -eq 2 ]; then
      COMMIT_MSG="$2"
      SESSION_ID=$(auto_detect_session)
    else
      die "merge requires a commit message and optionally a session ID"
    fi

    get_session_info "$SESSION_ID"
    [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $SESSION_ID"

    # Auto-stage and commit all changes if any exist
    cd "$WORKTREE_DIR"
    if ! git diff --quiet --exit-code --ignore-submodules -- || ! git diff --quiet --exit-code --cached --ignore-submodules -- || [ -n "$(git ls-files --others --exclude-standard)" ]; then
      echo "▶ staging all changes in session $SESSION_ID"
      git add -A || die "failed to stage changes"
      echo "▶ committing changes in session $SESSION_ID"
      git commit -m "$COMMIT_MSG" || die "failed to commit changes"
    else
      echo "ℹ️  no changes to commit in session $SESSION_ID"
    fi
    cd "$REPO_ROOT"

    echo "▶ rebasing $TEMP_BRANCH onto $BASE_BRANCH"
    if ! git -C "$WORKTREE_DIR" rebase "$BASE_BRANCH"; then
      echo "❌ rebase conflicts in session $SESSION_ID" >&2
      echo "   → resolve conflicts in $WORKTREE_DIR" >&2
      echo "   → then run: pursor continue" >&2
      exit 1
    fi

    echo "▶ merging session $SESSION_ID into $BASE_BRANCH"
    git -C "$REPO_ROOT" checkout "$BASE_BRANCH"
    if ! git -C "$REPO_ROOT" merge --ff-only "$TEMP_BRANCH" 2>/dev/null; then
      echo "▶ using non-fast-forward merge"
      if ! git -C "$REPO_ROOT" merge --no-ff -m "$COMMIT_MSG" "$TEMP_BRANCH"; then
        echo "❌ merge conflicts – resolve them and complete the merge manually, then run cleanup:" >&2
        echo "   git worktree remove '$WORKTREE_DIR'" >&2
        echo "   git branch -D '$TEMP_BRANCH'" >&2
        echo "   rm -f '$STATE_DIR/$SESSION_ID.state'" >&2
        exit 1
      fi
    fi

    echo "▶ cleaning up session $SESSION_ID"
    git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR"
    git -C "$REPO_ROOT" branch -D "$TEMP_BRANCH"
    rm -f "$STATE_DIR/$SESSION_ID.state"
    echo "merge complete for session $SESSION_ID ✅"
    ;;

  continue)
    if [ "$#" -eq 1 ]; then
      # Auto-detect session
      SESSION_ID=$(auto_detect_session)
    elif [ "$#" -eq 2 ]; then
      # Session ID provided
      SESSION_ID="$2"
    else
      die "continue takes optionally a session ID"
    fi

    continue_merge "$SESSION_ID"
    ;;

  cancel|abort)
    if [ "$#" -eq 1 ]; then
      # Auto-detect session
      SESSION_ID=$(auto_detect_session)
    elif [ "$#" -eq 2 ]; then
      # Session ID provided
      SESSION_ID="$2"
    else
      die "cancel takes optionally a session ID"
    fi

    get_session_info "$SESSION_ID"
    echo "▶ aborting session $SESSION_ID; removing $WORKTREE_DIR & deleting $TEMP_BRANCH"
    git -C "$REPO_ROOT" worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true
    git -C "$REPO_ROOT" branch -D "$TEMP_BRANCH" 2>/dev/null || true
    rm -f "$STATE_DIR/$SESSION_ID.state"
    echo "cancelled session $SESSION_ID"
    ;;

  clean)
    clean_all_sessions
    ;;

  list|ls)
    list_sessions
    ;;

  *)
    usage
    ;;
esac 