#!/usr/bin/env sh
# Session management for para

# Load session information from state file
get_session_info() {
  SESSION_ID="$1"
  STATE_FILE="$STATE_DIR/$SESSION_ID.state"
  [ -f "$STATE_FILE" ] || die "session '$SESSION_ID' not found"

  # Read state file with backward compatibility
  STATE_CONTENT=$(cat "$STATE_FILE")
  case "$STATE_CONTENT" in
  *"|"*"|"*"|"*)
    # New format with merge mode
    IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH MERGE_MODE <"$STATE_FILE"
    ;;
  *)
    # Old format without merge mode, default to squash
    IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH <"$STATE_FILE"
    MERGE_MODE="squash"
    ;;
  esac
}

# Save session state to file
save_session_state() {
  session_id="$1"
  temp_branch="$2"
  worktree_dir="$3"
  base_branch="$4"
  merge_mode="${5:-squash}" # Default to squash if not provided

  mkdir -p "$STATE_DIR"
  echo "$temp_branch|$worktree_dir|$base_branch|$merge_mode" >"$STATE_DIR/$session_id.state"
}

# Update merge mode for existing session
update_session_merge_mode() {
  session_id="$1"
  merge_mode="$2"

  get_session_info "$session_id"
  save_session_state "$session_id" "$TEMP_BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" "$merge_mode"
}

# Remove session state file
remove_session_state() {
  session_id="$1"
  
  # Optimize: Use single rm command with multiple files for efficiency
  rm -f "$STATE_DIR/$session_id.state" "$STATE_DIR/$session_id.prompt" "$STATE_DIR/$session_id.multi" 2>/dev/null || true

  # Optimize: Only try to remove directory if it exists and might be empty
  # Use rmdir instead of more expensive directory checks
  if [ -d "$STATE_DIR" ]; then
    rmdir "$STATE_DIR" 2>/dev/null || true
  fi
}

# Auto-detect session based on current directory
auto_detect_session() {
  CURRENT_DIR="$PWD"

  # Get current configurable prefix for new detection
  current_prefix=$(get_branch_prefix)

  # Try to detect friendly session (new format with configurable prefix)
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/$current_prefix/[a-z_]*[0-9]\{8\}-[0-9]\{6\}"; then
    FRIENDLY_SESSION=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/$current_prefix/\([a-z_]*[0-9]\{8\}-[0-9]\{6\}\).*|\1|p")
    if [ -n "$FRIENDLY_SESSION" ]; then
      CONTEXT_SESSION_ID="$FRIENDLY_SESSION"
      if [ -f "$STATE_DIR/$CONTEXT_SESSION_ID.state" ]; then
        echo "$CONTEXT_SESSION_ID"
        return 0
      fi
    fi
  fi

  # Try to detect legacy friendly session (legacy "pc/" format)
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/pc/[a-z_]*[0-9]\{8\}-[0-9]\{6\}"; then
    FRIENDLY_SESSION=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/pc/\([a-z_]*[0-9]\{8\}-[0-9]\{6\}\).*|\1|p")
    if [ -n "$FRIENDLY_SESSION" ]; then
      CONTEXT_SESSION_ID="$FRIENDLY_SESSION"
      if [ -f "$STATE_DIR/$CONTEXT_SESSION_ID.state" ]; then
        echo "$CONTEXT_SESSION_ID"
        return 0
      fi
    fi
  fi

  # Try to detect legacy timestamp session
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

  # Generic detection - extract any directory under subtrees and match against state files
  if echo "$CURRENT_DIR" | grep -q "/$SUBTREES_DIR_NAME/[^/]*/"; then
    BRANCH_PATH=$(echo "$CURRENT_DIR" | sed -n "s|.*/$SUBTREES_DIR_NAME/\([^/]*\).*|\1|p")
    if [ -n "$BRANCH_PATH" ]; then
      if [ -d "$STATE_DIR" ]; then
        # Try to find matching session by branch name
        for state_file in "$STATE_DIR"/*.state; do
          [ -f "$state_file" ] || continue
          SESSION_ID=$(basename "$state_file" .state)

          # Use backward-compatible state reading
          STATE_CONTENT=$(cat "$state_file")
          case "$STATE_CONTENT" in
          *"|"*"|"*"|"*)
            IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH MERGE_MODE <"$state_file"
            ;;
          *)
            IFS='|' read -r TEMP_BRANCH WORKTREE_DIR BASE_BRANCH <"$state_file"
            ;;
          esac

          # Extract just the branch name from the full branch path
          BRANCH_NAME=$(echo "$TEMP_BRANCH" | sed 's|.*/||')
          if [ "$BRANCH_PATH" = "$BRANCH_NAME" ] || [ "$BRANCH_PATH" = "$TEMP_BRANCH" ]; then
            echo "$SESSION_ID"
            return 0
          fi
        done
      fi
    fi
  fi

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
    die "multiple sessions found; specify which one to use:"
    list_sessions >&2
    exit 1
  fi

  echo "$FOUND_SESSION"
}

# List all active sessions
list_sessions() {
  if [ ! -d "$STATE_DIR" ] && [ ! -d "$SUBTREES_DIR" ]; then
    echo "No active parallel sessions."
    return 0
  fi

  SESSIONS_FOUND=0
  ORPHANED_FOUND=0

  # List sessions with state files
  if [ -d "$STATE_DIR" ]; then
    for state_file in "$STATE_DIR"/*.state; do
      [ -f "$state_file" ] || continue
      SESSIONS_FOUND=1
      session_id=$(basename "$state_file" .state)

      # Use get_session_info for backward compatibility
      get_session_info "$session_id"

      # Make session display more user-friendly
      if echo "$session_id" | grep -q "^pc-[0-9]\{8\}-[0-9]\{6\}$"; then
        timestamp=$(echo "$session_id" | sed 's/pc-//')
        display_name="$session_id (legacy: ${timestamp})"
      elif echo "$session_id" | grep -q "^[a-z_]*[0-9]\{8\}-[0-9]\{6\}$"; then
        display_name="$session_id"
      else
        display_name="$session_id (custom)"
      fi

      # Check if this is part of a multi-instance group
      if load_multi_session_metadata "$session_id"; then
        display_name="$display_name [Group: $GROUP_NAME, Instance: $INSTANCE_NUMBER/$TOTAL_INSTANCES]"
      fi

      echo "Session: $display_name"
      echo "  Branch: $TEMP_BRANCH"
      echo "  Worktree: $WORKTREE_DIR"
      echo "  Base: $BASE_BRANCH"
      echo "  Mode: $MERGE_MODE"
      if [ -d "$WORKTREE_DIR" ]; then
        cd "$WORKTREE_DIR" || die "failed to change to worktree directory"
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
        cd "$REPO_ROOT" || die "failed to change to repository root"
      else
        echo "  Status: ❌ Worktree missing"
      fi

      echo "  Resume: para resume $session_id"
      echo ""
    done
  fi

  # List orphaned worktrees (worktrees without state files)
  if [ -d "$SUBTREES_DIR" ]; then
    echo "Checking for orphaned worktrees..."
    for worktree_dir in "$SUBTREES_DIR"/*; do
      [ -d "$worktree_dir" ] || continue

      # Check if this worktree has a corresponding state file
      found_in_state=false
      if [ -d "$STATE_DIR" ]; then
        for state_file in "$STATE_DIR"/*.state; do
          [ -f "$state_file" ] || continue
          session_id=$(basename "$state_file" .state)
          get_session_info "$session_id"
          if [ "$WORKTREE_DIR" = "$worktree_dir" ]; then
            found_in_state=true
            break
          fi
        done
      fi

      # If not found in state, it's orphaned
      if [ "$found_in_state" = false ]; then
        if [ "$ORPHANED_FOUND" -eq 0 ]; then
          echo "Orphaned worktrees (limited functionality):"
        fi
        ORPHANED_FOUND=1
        worktree_name=$(basename "$worktree_dir")

        echo "Orphaned: $worktree_name"
        echo "  Path: $worktree_dir"
        if [ -d "$worktree_dir" ]; then
          cd "$worktree_dir" || continue
          if git rev-parse --git-dir >/dev/null 2>&1; then
            current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
            echo "  Branch: $current_branch"
            if git status --porcelain | grep -q "^UU\|^AA\|^DD"; then
              echo "  Status: ⚠️  Has merge conflicts"
            elif git diff --quiet --exit-code --cached --ignore-submodules -- 2>/dev/null; then
              if git diff --quiet --exit-code --ignore-submodules -- 2>/dev/null; then
                echo "  Status: ✅ Clean"
              else
                echo "  Status: 📝 Has uncommitted changes"
              fi
            else
              echo "  Status: 📦 Has staged changes"
            fi
          else
            echo "  Status: ❌ Not a valid git worktree"
          fi
          cd "$REPO_ROOT" || die "failed to change to repository root"
        else
          echo "  Status: ❌ Directory missing"
        fi
        echo "  Resume: para resume $worktree_name"
        echo ""
      fi
    done
  fi

  if [ "$SESSIONS_FOUND" -eq 0 ] && [ "$ORPHANED_FOUND" -eq 0 ]; then
    echo "No active parallel sessions or orphaned worktrees found."
  else
    if [ "$SESSIONS_FOUND" -gt 0 ]; then
      echo "💡 Tip: Use 'para resume <session-name>' to reconnect to an active session"
    fi
    if [ "$ORPHANED_FOUND" -gt 0 ]; then
      echo "🔧 Tip: Orphaned worktrees have limited functionality but can still be resumed"
    fi
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

  if [ -n "$session_name" ]; then
    SESSION_ID="$session_name"
    TS=$(generate_timestamp)
    # Use configurable prefix instead of hardcoded "pc/"
    prefix=$(get_branch_prefix)
    TEMP_BRANCH="${prefix}/$session_name-$TS"
  else
    SESSION_ID=$(generate_session_id)
    # Use configurable prefix instead of hardcoded "pc/"
    prefix=$(get_branch_prefix)
    TEMP_BRANCH="${prefix}/$SESSION_ID"
  fi
  WORKTREE_DIR="$SUBTREES_DIR/$TEMP_BRANCH"

  # Check if session already exists
  if session_exists "$SESSION_ID"; then
    die "session '$SESSION_ID' already exists. Use 'para resume $SESSION_ID' or choose a different name."
  fi

  echo "▶ creating session $SESSION_ID: branch $TEMP_BRANCH and worktree $WORKTREE_DIR (base $base_branch)" >&2

  # Create worktree and save state
  create_worktree "$TEMP_BRANCH" "$WORKTREE_DIR"
  save_session_state "$SESSION_ID" "$TEMP_BRANCH" "$WORKTREE_DIR" "$base_branch"

  echo "$SESSION_ID"
}

# Create multiple sessions for multi-instance dispatch
create_multi_session() {
  session_base_name="$1"
  instance_count="$2"
  base_branch="$3"
  
  # Generate group ID if no base name provided
  if [ -z "$session_base_name" ]; then
    session_base_name="multi-$(generate_timestamp)"
  fi
  
  # Create instances
  instance_ids=""
  i=1
  while [ "$i" -le "$instance_count" ]; do
    instance_name="${session_base_name}-${i}"
    instance_id=$(create_session "$instance_name" "$base_branch")
    
    # Save group metadata for this instance
    save_multi_session_metadata "$instance_id" "$session_base_name" "$i" "$instance_count"
    
    if [ "$i" -eq 1 ]; then
      instance_ids="$instance_id"
    else
      instance_ids="$instance_ids $instance_id"
    fi
    i=$((i + 1))
  done
  
  echo "$instance_ids"
}

# Save multi-session group metadata
save_multi_session_metadata() {
  session_id="$1"
  group_name="$2"
  instance_number="$3"
  total_instances="$4"
  
  mkdir -p "$STATE_DIR"
  echo "$group_name|$instance_number|$total_instances" >"$STATE_DIR/$session_id.multi"
}

# Load multi-session group metadata
load_multi_session_metadata() {
  session_id="$1"
  multi_file="$STATE_DIR/$session_id.multi"
  
  if [ -f "$multi_file" ]; then
    IFS='|' read -r GROUP_NAME INSTANCE_NUMBER TOTAL_INSTANCES <"$multi_file"
    return 0
  else
    return 1
  fi
}

# Get all sessions in a multi-instance group
get_multi_session_group() {
  group_name="$1"
  
  if [ ! -d "$STATE_DIR" ]; then
    return 1
  fi
  
  group_sessions=""
  for multi_file in "$STATE_DIR"/*.multi; do
    [ -f "$multi_file" ] || continue
    
    session_id=$(basename "$multi_file" .multi)
    if load_multi_session_metadata "$session_id"; then
      if [ "$GROUP_NAME" = "$group_name" ]; then
        if [ -z "$group_sessions" ]; then
          group_sessions="$session_id"
        else
          group_sessions="$group_sessions $session_id"
        fi
      fi
    fi
  done
  
  echo "$group_sessions"
}

# Remove multi-session metadata
remove_multi_session_metadata() {
  session_id="$1"
  rm -f "$STATE_DIR/$session_id.multi" 2>/dev/null || true
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

    # Use get_session_info for backward compatibility
    get_session_info "$session_id"

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

# Save initial prompt for session
save_session_prompt() {
  session_id="$1"
  prompt="$2"

  mkdir -p "$STATE_DIR"
  echo "$prompt" >"$STATE_DIR/$session_id.prompt"
}

# Load initial prompt for session
load_session_prompt() {
  session_id="$1"
  prompt_file="$STATE_DIR/$session_id.prompt"

  if [ -f "$prompt_file" ]; then
    cat "$prompt_file"
  else
    echo ""
  fi
}

# Remove session prompt file
remove_session_prompt() {
  session_id="$1"
  rm -f "$STATE_DIR/$session_id.prompt"
}

# Enhanced resume discovery - scan worktrees even without state files
discover_all_sessions() {
  ACTIVE_SESSIONS=""
  ORPHANED_SESSIONS=""

  # First, get sessions with state files
  if [ -d "$STATE_DIR" ]; then
    for state_file in "$STATE_DIR"/*.state; do
      [ -f "$state_file" ] || continue
      session_id=$(basename "$state_file" .state)
      ACTIVE_SESSIONS="$ACTIVE_SESSIONS $session_id"
    done
  fi

  # Then, scan for worktree directories that might be orphaned
  if [ -d "$SUBTREES_DIR" ]; then
    for worktree_dir in "$SUBTREES_DIR"/*; do
      [ -d "$worktree_dir" ] || continue

      # Extract potential session info from path
      worktree_path=$(basename "$worktree_dir")

      # Check if this worktree has a corresponding state file
      found_in_state=false
      if [ -d "$STATE_DIR" ]; then
        for state_file in "$STATE_DIR"/*.state; do
          [ -f "$state_file" ] || continue
          session_id=$(basename "$state_file" .state)
          get_session_info "$session_id"
          if [ "$WORKTREE_DIR" = "$worktree_dir" ]; then
            found_in_state=true
            break
          fi
        done
      fi

      # If not found in state, it's orphaned
      if [ "$found_in_state" = false ]; then
        ORPHANED_SESSIONS="$ORPHANED_SESSIONS $worktree_path"
      fi
    done
  fi

  # Output results
  echo "active:$ACTIVE_SESSIONS"
  echo "orphaned:$ORPHANED_SESSIONS"
}

# Enhanced resume - can resume from session ID or auto-discover
enhanced_resume() {
  target_session="$1"

  if [ -n "$target_session" ]; then
    # Specific session requested
    if session_exists "$target_session"; then
      # Session has state file - use normal resume
      get_session_info "$target_session"
      [ -d "$WORKTREE_DIR" ] || die "worktree $WORKTREE_DIR missing for session $target_session"

      # Load initial prompt if it exists for this session
      STORED_PROMPT=$(load_session_prompt "$target_session")

      echo "▶ resuming session $target_session"
      launch_ide "$(get_default_ide)" "$WORKTREE_DIR" "$STORED_PROMPT"
    else
      # Check if it's an orphaned worktree
      prefix=$(get_branch_prefix)
      possible_paths="$SUBTREES_DIR/$prefix/$target_session $SUBTREES_DIR/pc/$target_session $SUBTREES_DIR/$target_session"

      for possible_path in $possible_paths; do
        if [ -d "$possible_path" ]; then
          echo "▶ resuming orphaned session in $possible_path"
          echo "  ⚠️  Note: This session doesn't have para state - some features may be limited"
          launch_ide "$(get_default_ide)" "$possible_path" ""
          return 0
        fi
      done

      die "session '$target_session' not found in active sessions or worktrees"
    fi
  else
    # Auto-discover and present options
    discovery=$(discover_all_sessions)
    active_sessions=$(echo "$discovery" | grep "^active:" | sed 's/active://')
    orphaned_sessions=$(echo "$discovery" | grep "^orphaned:" | sed 's/orphaned://')

    total_active=$(echo "$active_sessions" | wc -w)
    total_orphaned=$(echo "$orphaned_sessions" | wc -w)

    if [ "$total_active" -eq 0 ] && [ "$total_orphaned" -eq 0 ]; then
      die "no sessions found to resume"
    fi

    if [ "$total_active" -eq 1 ] && [ "$total_orphaned" -eq 0 ]; then
      # Single active session - auto-resume
      session_id=$(echo "$active_sessions" | tr -d ' ')
      enhanced_resume "$session_id"
    else
      # Multiple options - let user choose
      echo "Multiple sessions available for resume:"
      echo ""

      if [ "$total_active" -gt 0 ]; then
        echo "Active sessions (with full para state):"
        for session_id in $active_sessions; do
          get_session_info "$session_id"
          echo "  → $session_id (Branch: $TEMP_BRANCH)"
        done
        echo ""
      fi

      if [ "$total_orphaned" -gt 0 ]; then
        echo "Orphaned worktrees (limited functionality):"
        for worktree_path in $orphaned_sessions; do
          echo "  → $worktree_path"
        done
        echo ""
      fi

      echo "Use: para resume <session-name>"
    fi
  fi
}
