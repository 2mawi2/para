#!/usr/bin/env bats

# Test the new archive namespace backup and recovery system

setup() {
  # Create temporary directory for tests
  export TEST_DIR=$(mktemp -d)
  cd "$TEST_DIR"
  
  # Initialize git repository
  git init
  git config user.email "test@example.com"
  git config user.name "Test User"
  
  # Create initial commit
  echo "initial" > file.txt
  git add file.txt
  git commit -m "initial commit"
  
  # Set up para environment
  export REPO_ROOT="$TEST_DIR"
  export SUBTREES_DIR_NAME="subtrees"
  export STATE_DIR_NAME=".para_state"
  export SUBTREES_DIR="$TEST_DIR/$SUBTREES_DIR_NAME"
  export STATE_DIR="$TEST_DIR/$STATE_DIR_NAME"
  export IDE_NAME="claude"
  export BRANCH_PREFIX="para"
  
  # Source para libraries
  SCRIPT_DIR="$(dirname "$BATS_TEST_FILENAME")/.."
  . "$SCRIPT_DIR/lib/para-config.sh"
  . "$SCRIPT_DIR/lib/para-utils.sh"
  . "$SCRIPT_DIR/lib/para-git.sh"
  . "$SCRIPT_DIR/lib/para-session.sh"
  . "$SCRIPT_DIR/lib/para-commands.sh"
}

teardown() {
  cd /
  rm -rf "$TEST_DIR"
}

@test "session creation uses para/wip/ namespace" {
  # Create a new session
  SESSION_ID=$(create_session "test-session" "master")
  
  # Verify branch was created in wip namespace
  run git branch --list "para/wip/*"
  [ "$status" -eq 0 ]
  [[ "$output" =~ para/wip/test-session- ]]
}

@test "cancel command moves branch to archive namespace" {
  # Create and cancel a session
  SESSION_ID=$(create_session "test-session" "master")
  get_session_info "$SESSION_ID"
  
  # Cancel the session (should move to archive)
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  
  # Verify original branch no longer exists
  run git rev-parse --verify "$TEMP_BRANCH"
  [ "$status" -ne 0 ]
  
  # Verify branch was moved to archive
  ARCHIVE_BRANCH=$(echo "$TEMP_BRANCH" | sed 's|/wip/|/archive/|')
  run git rev-parse --verify "$ARCHIVE_BRANCH"
  [ "$status" -eq 0 ]
}

@test "recover command lists archive sessions when no argument provided" {
  # Create and cancel a session
  SESSION_ID=$(create_session "test-session" "master")
  get_session_info "$SESSION_ID"
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID"
  
  # List archive sessions
  run list_archive_sessions
  [ "$status" -eq 0 ]
  [[ "$output" =~ "test-session" ]]
  [[ "$output" =~ "para recover" ]]
}

@test "recover command restores session from archive" {
  # Create and cancel a session
  SESSION_ID=$(create_session "test-session" "master")
  get_session_info "$SESSION_ID"
  ORIGINAL_BRANCH="$TEMP_BRANCH"
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID"
  
  # Extract session name for recovery
  SESSION_NAME=$(echo "$ORIGINAL_BRANCH" | sed 's|para/wip/||')
  
  # Recover the session
  run recover_archive_session "$SESSION_NAME"
  [ "$status" -eq 0 ]
  
  # Verify branch was moved back to wip
  WIP_BRANCH="para/wip/$SESSION_NAME"
  run git rev-parse --verify "$WIP_BRANCH"
  [ "$status" -eq 0 ]
  
  # Verify archive branch no longer exists
  ARCHIVE_BRANCH="para/archive/$SESSION_NAME"
  run git rev-parse --verify "$ARCHIVE_BRANCH"
  [ "$status" -ne 0 ]
  
  # Verify worktree was recreated
  WORKTREE_PATH="$SUBTREES_DIR/$WIP_BRANCH"
  [ -d "$WORKTREE_PATH" ]
}

@test "recover fails if session already exists in wip" {
  # Create a session and move it to archive manually
  git checkout -b "para/archive/test-session-20240101-120000"
  git checkout master
  
  # Create a conflicting branch in wip namespace 
  git checkout -b "para/wip/test-session-20240101-120000"
  git checkout master
  
  # Try to recover - should fail because wip branch already exists
  run recover_archive_session "test-session-20240101-120000"
  [ "$status" -ne 0 ]
  [[ "$output" =~ "already exists in active sessions" ]]
}

@test "clean --backups removes all archive branches" {
  # Create and cancel multiple sessions
  SESSION_ID1=$(create_session "test-session1" "master")
  get_session_info "$SESSION_ID1"
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID1"
  
  SESSION_ID2=$(create_session "test-session2" "master")
  get_session_info "$SESSION_ID2"
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID2"
  
  # Verify archive branches exist
  run git branch --list "para/archive/*"
  [ "$status" -eq 0 ]
  [[ "$output" =~ "para/archive/test-session1" ]]
  [[ "$output" =~ "para/archive/test-session2" ]]
  
  # Clean archive
  run clean_archive_sessions
  [ "$status" -eq 0 ]
  [[ "$output" =~ "cleaned up 2 cancelled session" ]]
  
  # Verify no archive branches remain
  run git branch --list "para/archive/*"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}


@test "list command only shows active sessions, not archive" {
  # Create an active session
  SESSION_ID1=$(create_session "active-session" "master")
  
  # Create and cancel another session
  SESSION_ID2=$(create_session "cancelled-session" "master")
  get_session_info "$SESSION_ID2"
  cancel_session "$TEMP_BRANCH" "$WORKTREE_DIR"
  remove_session_state "$SESSION_ID2"
  
  # List sessions should only show active one
  run list_sessions
  [ "$status" -eq 0 ]
  [[ "$output" =~ "active-session" ]]
  [[ ! "$output" =~ "cancelled-session" ]]
}

@test "recover fails gracefully for non-existent session" {
  # Try to recover a session that doesn't exist
  run recover_archive_session "non-existent-session"
  [ "$status" -ne 0 ]
  [[ "$output" =~ "Session 'non-existent-session' not found in archive" ]]
  [[ "$output" =~ "Available sessions:" ]]
}

@test "cancel command handles legacy branches correctly" {
  # Create a legacy branch manually (without wip namespace)
  git checkout -b "para/legacy-session-20240101-120000"
  git checkout master
  
  # Test cancel with legacy branch format
  run cancel_session "para/legacy-session-20240101-120000" "/fake/worktree"
  [ "$status" -eq 0 ]
  
  # Should be moved to archive
  run git rev-parse --verify "para/archive/legacy-session-20240101-120000"
  [ "$status" -eq 0 ]
  run git rev-parse --verify "para/legacy-session-20240101-120000"
  [ "$status" -ne 0 ]
}

@test "clean --backups with no archive sessions shows appropriate message" {
  # Clean archive when empty
  run clean_archive_sessions
  [ "$status" -eq 0 ]
  [[ "$output" =~ "No cancelled sessions found in archive to clean" ]]
}

@test "recover with empty archive shows helpful message" {
  # List archive when empty
  run list_archive_sessions
  [ "$status" -eq 0 ]
  [[ "$output" =~ "No cancelled sessions found in archive" ]]
  [[ "$output" =~ "para cancel" ]]
  [[ "$output" =~ "para recover" ]]
}