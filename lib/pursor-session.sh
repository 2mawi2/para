#!/usr/bin/env sh
# Session management for pursor

# Get session information from state file
get_session_info() {
  SESSION_ID="$1"
  STATE_FILE="$STATE_DIR/$SESSION_ID.state"
  [ -f "$STATE_FILE" ] || die "session '$SESSION_ID' not found"
  IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH < "$STATE_FILE"
}

# Save session state to file
save_session_state() {
  session_id="$1"
  temp_branch="$2"
  worktree_dir="$3"
  base_branch="$4"
  
  mkdir -p "$STATE_DIR"
  echo "$temp_branch|$worktree_dir|$base_branch" > "$STATE_DIR/$session_id.state"
}

# Remove session state file
remove_session_state() {
  session_id="$1"
  rm -f "$STATE_DIR/$session_id.state"
  
  # Clean up state directory if empty
  if [ -d "$STATE_DIR" ]; then
    rmdir "$STATE_DIR" 2>/dev/null || true
  fi
}

# Auto-detect current session from working directory
auto_detect_session() {
  CURRENT_DIR="$PWD"
  
  # Pattern 1: Check for regular timestamp-based sessions (pc/YYYYMMDD-HHMMSS)
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/pc/[0-9]\{8\}-[0-9]\{6\}"; then
    TIMESTAMP=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/pc/\([0-9]\{8\}-[0-9]\{6\}\).*|\1|p")
    if [ -n "$TIMESTAMP" ]; then
      CONTEXT_SESSION_ID="pc-$TIMESTAMP"
      if [ -f "$STATE_DIR/$CONTEXT_SESSION_ID.state" ]; then
        echo "$CONTEXT_SESSION_ID"
        return 0
      fi
    fi
  fi
  
  # Pattern 2: Check for custom named sessions
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/pc/"; then
    PC_DIR_NAME=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/pc/\([^/]*\).*|\1|p")
    if [ -n "$PC_DIR_NAME" ]; then
      if [ -d "$STATE_DIR" ]; then
        for state_file in "$STATE_DIR"/*.state; do
          [ -f "$state_file" ] || continue
          SESSION_ID=$(basename "$state_file" .state)
          
          IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH < "$state_file"
          
          if [ "pc/$PC_DIR_NAME" = "$TEMP_BRANCH" ]; then
            echo "$SESSION_ID"
            return 0
          fi
        done
      fi
    fi
  fi

  # Fallback: single session detection
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

# List all active sessions
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
    
    # Make session display more user-friendly
    if echo "$session_id" | grep -q "^pc-[0-9]\{8\}-[0-9]\{6\}$"; then
      timestamp=$(echo "$session_id" | sed 's/pc-//')
      display_name="$session_id (${timestamp})"
    else
      display_name="$session_id"
    fi
    
    echo "Session: $display_name"
    echo "  Branch: $TEMP_BRANCH"
    echo "  Worktree: $WORKTREE_DIR"
    echo "  Base: $BASE_BRANCH"
    if [ -d "$WORKTREE_DIR" ]; then
      cd "$WORKTREE_DIR"
      if git status --porcelain | grep -q "^UU\|^AA\|^DD"; then
        echo "  Status: ⚠️  Has merge conflicts"
      elif git diff --quiet --exit-code --cached --ignore-submodules --; then
        if git diff --quiet --exit-code --ignore-submodules --; then
          echo "  Status: ✅ Clean"
        else
          echo "  Status: 📝 Has uncommitted changes"
        fi
      else
        echo "  Status: 📦 Has staged changes"
      fi
      cd "$REPO_ROOT"
    else
      echo "  Status: ❌ Worktree missing"
    fi
    
    echo "  Resume: pursor resume $session_id"
    echo ""
  done
  
  if [ "$SESSIONS_FOUND" -eq 0 ]; then
    echo "No active parallel sessions."
  else
    echo "💡 Tip: Use 'pursor resume <session-name>' to reconnect to an existing session"
  fi
}

# Check if session exists
session_exists() {
  session_id="$1"
  [ -f "$STATE_DIR/$session_id.state" ]
}

# Create new session
create_session() {
  session_name="$1"
  base_branch="$2"
  
  # Generate session ID and branch name
  TS=$(generate_timestamp)
  if [ -n "$session_name" ]; then
    SESSION_ID="$session_name"
    TEMP_BRANCH="pc/$session_name-$TS"
  else
    SESSION_ID="pc-$TS"
    TEMP_BRANCH="pc/$TS"
  fi
  WORKTREE_DIR="$SUBTREES_DIR/$TEMP_BRANCH"

  # Check if session already exists
  if session_exists "$SESSION_ID"; then
    die "session '$SESSION_ID' already exists. Use 'pursor resume $SESSION_ID' or choose a different name."
  fi

  echo "▶ creating session $SESSION_ID: branch $TEMP_BRANCH and worktree $WORKTREE_DIR (base $base_branch)" >&2
  
  # Create worktree and save state
  create_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
  save_session_state "$SESSION_ID" "$TEMP_BRANCH" "$WORKTREE_DIR" "$base_branch"
  
  echo "$SESSION_ID"
}

# Clean up all sessions
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
    
    remove_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
    remove_session_state "$session_id"
    
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