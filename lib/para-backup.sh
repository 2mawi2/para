#!/usr/bin/env sh
# Session backup management for para
# Handles last 3 cancelled sessions backup and recovery

# Maximum number of cancelled sessions to keep as backup
MAX_BACKUP_SESSIONS=3

# Initialize backup directories
init_backup_paths() {
  BACKUP_DIR="$STATE_DIR/backups"
  mkdir -p "$BACKUP_DIR" 2>/dev/null || true
}

# Save cancelled session to backup
save_cancelled_session_backup() {
  session_id="$1"
  temp_branch="$2"
  worktree_dir="$3"
  base_branch="$4"
  merge_mode="$5"

  init_backup_paths

  timestamp=$(date '+%Y-%m-%d %H:%M:%S')
  backup_file="$BACKUP_DIR/$session_id.backup"

  {
    echo "session_id=$session_id"
    echo "timestamp='$timestamp'"
    echo "temp_branch=$temp_branch"
    echo "worktree_dir=$worktree_dir"
    echo "base_branch=$base_branch"
    echo "merge_mode=$merge_mode"
  } >"$backup_file"

  # Cleanup old backups to maintain only the last 3
  cleanup_old_backups
}

# Cleanup old backups keeping only the most recent 3
cleanup_old_backups() {
  init_backup_paths

  backup_count=$(find "$BACKUP_DIR" -name "*.backup" 2>/dev/null | wc -l)

  if [ "$backup_count" -gt "$MAX_BACKUP_SESSIONS" ]; then
    # Remove oldest backups, keep only the newest ones
    old_backups=$(find "$BACKUP_DIR" -name "*.backup" -print0 2>/dev/null |
      xargs -0 ls -t |
      tail -n +$((MAX_BACKUP_SESSIONS + 1)))

    # Remove backup files and their corresponding branches
    for backup_file in $old_backups; do
      if [ -f "$backup_file" ]; then
        # Load backup to get branch name
        # shellcheck source=/dev/null
        . "$backup_file"

        # Remove the backup file
        rm -f "$backup_file"

        # Remove the preserved branch
        git -C "$REPO_ROOT" branch -D "$temp_branch" 2>/dev/null || true
      fi
    done
  fi
}

# Check if session exists in backup
session_exists_in_backup() {
  session_id="$1"
  init_backup_paths
  [ -f "$BACKUP_DIR/$session_id.backup" ]
}

# Load session from backup
load_session_from_backup() {
  session_id="$1"
  init_backup_paths
  backup_file="$BACKUP_DIR/$session_id.backup"

  [ -f "$backup_file" ] || return 1

  # Source the backup file to load variables
  # shellcheck source=/dev/null
  . "$backup_file"
}

# Recover a cancelled session from backup
recover_cancelled_session() {
  session_id="$1"

  init_backup_paths

  # Check if session already active
  if session_exists "$session_id"; then
    die "session '$session_id' is already active"
  fi

  # Check if session exists in backup
  if ! session_exists_in_backup "$session_id"; then
    die "session '$session_id' not found in backups"
  fi

  # Load session data from backup
  load_session_from_backup "$session_id"

  echo "▶ recovering cancelled session $session_id from backup"

  # Check if branch still exists
  if ! git branch --list "$temp_branch" | grep -q "$temp_branch"; then
    die "branch '$temp_branch' no longer exists - cannot recover session"
  fi

  # Recreate the worktree
  mkdir -p "$(dirname "$worktree_dir")"
  if ! git worktree add "$worktree_dir" "$temp_branch" 2>/dev/null; then
    # If worktree add fails, try to repair or remove existing worktree
    if [ -d "$worktree_dir" ]; then
      git worktree remove "$worktree_dir" --force 2>/dev/null || true
      git worktree add "$worktree_dir" "$temp_branch" 2>/dev/null ||
        die "failed to recreate worktree for session '$session_id'"
    else
      die "failed to create worktree for session '$session_id'"
    fi
  fi

  # Recreate session state
  save_session_state "$session_id" "$temp_branch" "$worktree_dir" "$base_branch" "$merge_mode"

  # Remove the backup file since session is now active again
  rm -f "$BACKUP_DIR/$session_id.backup"

  echo "✅ recovered cancelled session $session_id"
  echo "  ↳ branch: $temp_branch"
  echo "  ↳ worktree: $worktree_dir"
  echo "  ↳ resume: para resume $session_id"

  return 0
}

# List available backups
list_cancelled_session_backups() {
  init_backup_paths

  if [ ! -d "$BACKUP_DIR" ]; then
    echo "No cancelled sessions in backup."
    return 0
  fi

  backups_found=0
  for backup_file in "$BACKUP_DIR"/*.backup; do
    [ -f "$backup_file" ] || continue
    backups_found=1

    # Load backup data
    # shellcheck source=/dev/null
    . "$backup_file"

    echo "Session: $session_id"
    echo "  Cancelled: $timestamp"
    echo "  Branch: $temp_branch"
    echo "  Base: $base_branch"
    echo "  Mode: $merge_mode"
    echo "  Recover: para recover $session_id"
    echo ""
  done

  if [ "$backups_found" -eq 0 ]; then
    echo "No cancelled sessions in backup."
  else
    echo "💡 Tip: Use 'para recover <session-id>' to restore a cancelled session"
    echo "📝 Note: Only the last $MAX_BACKUP_SESSIONS cancelled sessions are kept"
  fi
}

# Clean all backup entries
clean_all_backups() {
  init_backup_paths

  if [ ! -d "$BACKUP_DIR" ]; then
    echo "No backups to clean."
    return 0
  fi

  cleaned_count=0
  for backup_file in "$BACKUP_DIR"/*.backup; do
    [ -f "$backup_file" ] || continue
    rm -f "$backup_file"
    cleaned_count=$((cleaned_count + 1))
  done

  # Clean up empty directory
  if [ "$cleaned_count" -gt 0 ] && [ -d "$BACKUP_DIR" ]; then
    rmdir "$BACKUP_DIR" 2>/dev/null || true
    if [ ! -d "$STATE_DIR" ] || [ -z "$(ls -A "$STATE_DIR" 2>/dev/null)" ]; then
      rmdir "$STATE_DIR" 2>/dev/null || true
    fi
  fi

  if [ "$cleaned_count" -gt 0 ]; then
    echo "🧹 Cleaned $cleaned_count backup(s)"
  else
    echo "No backups to clean"
  fi
}
