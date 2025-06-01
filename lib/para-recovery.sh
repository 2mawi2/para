#!/usr/bin/env sh
# Recovery management for para
# Handles finished/cancelled session recovery and history management

# Configuration
DEFAULT_RECOVERY_RETENTION_DAYS=7

# Get recovery retention days from config, with fallback to default
get_recovery_retention_days() {
  echo "${RECOVERY_RETENTION_DAYS:-$DEFAULT_RECOVERY_RETENTION_DAYS}"
}

# Initialize recovery directories
init_recovery_paths() {
  HISTORY_DIR="$REPO_ROOT/$STATE_DIR_NAME/history"
  mkdir -p "$HISTORY_DIR" 2>/dev/null || true
}

# Save session to history when finished or cancelled
save_session_to_history() {
  session_id="$1"
  status="$2"     # "finished" or "cancelled"
  commit_msg="$3" # Only for finished sessions
  temp_branch="$4"
  worktree_dir="$5"
  base_branch="$6"
  merge_mode="$7"

  init_recovery_paths

  timestamp=$(date '+%Y-%m-%d %H:%M:%S')
  history_file="$HISTORY_DIR/$session_id.history"

  {
    echo "session_id=$session_id"
    echo "status=$status"
    echo "timestamp='$timestamp'"
    echo "temp_branch=$temp_branch"
    echo "worktree_dir=$worktree_dir"
    echo "base_branch=$base_branch"
    echo "merge_mode=$merge_mode"
    if [ -n "$commit_msg" ]; then
      echo "commit_msg='$commit_msg'"
    fi
  } >"$history_file"

  # Also save the current state of files in the worktree for recovery
  if [ -d "$worktree_dir" ]; then
    snapshot_dir="$HISTORY_DIR/${session_id}.snapshot"
    mkdir -p "$snapshot_dir"

    # Create a tarball of the worktree content (excluding .git and .para_state)
    (cd "$worktree_dir" && tar -czf "$snapshot_dir/worktree.tar.gz" --exclude='.git' --exclude='.para_state' .)
  fi
}

# Load session from history
load_session_from_history() {
  session_id="$1"
  init_recovery_paths
  history_file="$HISTORY_DIR/$session_id.history"

  [ -f "$history_file" ] || return 1

  # Source the history file to load variables
  # shellcheck source=/dev/null
  . "$history_file"
}

# Check if session exists in history
session_exists_in_history() {
  session_id="$1"
  init_recovery_paths
  [ -f "$HISTORY_DIR/$session_id.history" ]
}

# Recover a session from history
recover_session() {
  session_id="$1"

  init_recovery_paths

  # Check if session already active
  if session_exists "$session_id"; then
    die "session '$session_id' is already active"
  fi

  # Check if session exists in history
  if ! session_exists_in_history "$session_id"; then
    die "session '$session_id' not found in history"
  fi

  # Load session data from history
  load_session_from_history "$session_id"

  echo "▶ recovering session $session_id from history"

  # Recreate the branch if it doesn't exist
  if ! git branch --list "$temp_branch" | grep -q "$temp_branch"; then
    git branch "$temp_branch" "$base_branch"
    echo "  ↳ recreated branch $temp_branch"
  fi

  # Recreate the worktree
  mkdir -p "$(dirname "$worktree_dir")"
  git worktree add "$worktree_dir" "$temp_branch" 2>/dev/null || {
    # If worktree add fails, it might already exist, try to repair
    git worktree repair "$worktree_dir" 2>/dev/null || true
  }

  # Restore the worktree content from snapshot if available
  snapshot_dir="$HISTORY_DIR/${session_id}.snapshot"
  if [ -f "$snapshot_dir/worktree.tar.gz" ]; then
    (cd "$worktree_dir" && tar -xzf "$snapshot_dir/worktree.tar.gz")
    echo "  ↳ restored worktree content from snapshot"
  fi

  # Recreate session state
  save_session_state "$session_id" "$temp_branch" "$worktree_dir" "$base_branch" "$merge_mode"

  echo "✅ recovered session $session_id"
  echo "  ↳ branch: $temp_branch"
  echo "  ↳ worktree: $worktree_dir"
  echo "  ↳ resume: para resume $session_id"

  return 0
}

# List session history
list_session_history() {
  init_recovery_paths

  if [ ! -d "$HISTORY_DIR" ]; then
    echo "No finished or cancelled sessions in history."
    return 0
  fi

  sessions_found=0
  for history_file in "$HISTORY_DIR"/*.history; do
    [ -f "$history_file" ] || continue
    sessions_found=1

    # Load session data
    # shellcheck source=/dev/null
    . "$history_file"

    echo "Session: $session_id"
    echo "  Status: $status"
    echo "  Date: $timestamp"
    echo "  Branch: $temp_branch"
    echo "  Base: $base_branch"
    echo "  Mode: $merge_mode"
    if [ -n "${commit_msg:-}" ]; then
      echo "  Commit: $commit_msg"
    fi
    echo "  Recover: para recover $session_id"
    echo ""
  done

  if [ "$sessions_found" -eq 0 ]; then
    echo "No finished or cancelled sessions in history."
  else
    echo "💡 Tip: Use 'para recover <session-id>' to restore a session"
  fi
}

# Clean history entries
clean_session_history() {
  older_than_days="${1:-}"

  init_recovery_paths

  if [ ! -d "$HISTORY_DIR" ]; then
    echo "No history to clean."
    return 0
  fi

  cleaned_count=0
  total_count=0

  for history_file in "$HISTORY_DIR"/*.history; do
    [ -f "$history_file" ] || continue
    total_count=$((total_count + 1))

    if [ -n "$older_than_days" ]; then
      # Check if file is older than specified days
      # Special case: 0 days means remove all files
      if [ "$older_than_days" -eq 0 ] || [ "$(find "$history_file" -mtime +$older_than_days 2>/dev/null | wc -l)" -gt 0 ]; then
        session_id=$(basename "$history_file" .history)
        rm -f "$history_file"
        rm -rf "$HISTORY_DIR/${session_id}.snapshot"
        cleaned_count=$((cleaned_count + 1))
      fi
    else
      # Clean all history
      session_id=$(basename "$history_file" .history)
      rm -f "$history_file"
      rm -rf "$HISTORY_DIR/${session_id}.snapshot"
      cleaned_count=$((cleaned_count + 1))
    fi
  done

  # Clean up empty directory
  if [ "$cleaned_count" -eq "$total_count" ] && [ -d "$HISTORY_DIR" ]; then
    rmdir "$HISTORY_DIR" 2>/dev/null || true
    if [ ! -d "$STATE_DIR" ] || [ -z "$(ls -A "$STATE_DIR" 2>/dev/null)" ]; then
      rmdir "$STATE_DIR" 2>/dev/null || true
    fi
  fi

  if [ "$cleaned_count" -gt 0 ]; then
    echo "🧹 Cleaned $cleaned_count session(s) from history"
  else
    echo "No sessions to clean from history"
  fi
}

# Auto-cleanup old history entries
auto_cleanup_history() {
  retention_days=$(get_recovery_retention_days)

  # Initialize paths first
  init_recovery_paths

  # Only auto-cleanup if there's a history directory
  if [ -d "$HISTORY_DIR" ]; then
    # Clean entries older than retention period
    clean_session_history "$retention_days" >/dev/null 2>&1
  fi
}
