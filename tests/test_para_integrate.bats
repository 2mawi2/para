#!/usr/bin/env bats

# Test para integrate functionality
# Tests for the --integrate flag on para finish and para continue command

setup() {
  # Create isolated test repository
  TEST_REPO_ROOT=$(mktemp -d)
  cd "$TEST_REPO_ROOT"
  
  # Initialize git repo
  git init
  git config user.name "Test User"
  git config user.email "test@example.com"
  
  # Create initial content
  echo "initial content" > file1.txt
  git add file1.txt
  git commit -m "initial commit"
  
  # Source para functions
  export PARA_CONFIG_SKIP_SETUP=1
  export SUBTREES_DIR="$TEST_REPO_ROOT/subtrees"
  export STATE_DIR="$TEST_REPO_ROOT/.para_state"
  export REPO_ROOT="$TEST_REPO_ROOT"
  export IDE_NAME="claude"
  export IDE_CMD="echo"
  export IDE_WRAPPER_ENABLED="true"  # Enable wrapper mode for tests
  export IDE_WRAPPER_NAME="code"  # Use VS Code as wrapper
  export IDE_WRAPPER_CMD="echo"  # Mock wrapper command
  
  # Source required libraries
  . "$BATS_TEST_DIRNAME/../lib/para-config.sh"
  . "$BATS_TEST_DIRNAME/../lib/para-utils.sh"
  . "$BATS_TEST_DIRNAME/../lib/para-git.sh"
  . "$BATS_TEST_DIRNAME/../lib/para-session.sh"
  . "$BATS_TEST_DIRNAME/../lib/para-ide.sh"
  . "$BATS_TEST_DIRNAME/../lib/para-commands.sh"
  
  # Initialize paths
  init_paths
}

teardown() {
  cd /
  rm -rf "$TEST_REPO_ROOT"
}

@test "finish with --integrate flag should merge changes cleanly" {
  # Create a session and make changes
  create_new_session "test-session" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "new content" > new_file.txt
  git add new_file.txt
  git commit -m "add new file"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test integration
  run handle_finish_command "finish" "test commit" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify changes are on master branch
  git checkout master
  [ -f "new_file.txt" ]
  [ "$(cat new_file.txt)" = "new content" ]
}

@test "finish with -i flag should work as shorthand for --integrate" {
  # Create a session and make changes
  create_new_session "test-session" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "shorthand test" > shorthand.txt
  git add shorthand.txt
  git commit -m "add shorthand test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test integration with short flag
  run handle_finish_command "finish" "shorthand commit" "-i"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify changes are on master branch
  git checkout master
  [ -f "shorthand.txt" ]
  [ "$(cat shorthand.txt)" = "shorthand test" ]
}

@test "finish without --integrate should not merge automatically" {
  # Create a session and make changes
  create_new_session "test-session" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "manual merge test" > manual.txt
  git add manual.txt
  git commit -m "add manual test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test without integration
  run handle_finish_command "finish" "manual commit"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "branch ready for manual merge" ]]
  [[ ! "$output" =~ "successfully integrated" ]]
  
  # Verify changes are NOT on master branch yet
  git checkout master
  [ ! -f "manual.txt" ]
  
  # But branch should exist
  git branch | grep -q "para/test-session"
}

@test "continue command should complete rebase after conflict resolution" {
  # Create a conflicting situation
  echo "master branch content" > conflict.txt
  git add conflict.txt
  git commit -m "master branch version"
  
  # Create feature branch with conflicting change
  git checkout -b test-feature
  echo "feature branch content" > conflict.txt
  git add conflict.txt
  git commit -m "feature branch version"
  
  # Switch back to master and create another conflicting change
  git checkout master
  echo "another master change" > conflict.txt
  git add conflict.txt
  git commit -m "another master change"
  
  # Simulate conflict state by attempting rebase
  git checkout test-feature
  git rebase master || true  # This will fail with conflicts
  
  # Save integration state manually (normally done by finish --integrate)
  mkdir -p "$STATE_DIR"
  cat > "$STATE_DIR/integration_conflict.state" <<EOF
FEATURE_BRANCH=test-feature
BASE_BRANCH=master
COMMIT_MSG=test rebase
EOF
  
  # Resolve conflicts manually
  echo "resolved content" > conflict.txt
  git add conflict.txt
  
  # Test continue command
  run handle_continue_command "continue"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "conflicts resolved and rebase completed" ]]
  [[ "$output" =~ "successfully integrated into master" ]]
  
  # Verify rebase was completed (no rebase state)
  [ ! -d "$REPO_ROOT/.git/rebase-merge" ]
  [ ! -d "$REPO_ROOT/.git/rebase-apply" ]
  [ "$(cat conflict.txt)" = "resolved content" ]
}

@test "continue command should fail if no integration in progress" {
  run handle_continue_command "continue"
  
  [ "$status" -eq 1 ]
  [[ "$output" =~ "no integration in progress" ]]
}

@test "continue command should fail if conflicts remain unresolved" {
  # Create a conflicting situation
  echo "master content" > conflict2.txt
  git add conflict2.txt
  git commit -m "master version"
  
  # Create feature branch with conflict
  git checkout -b test-feature2
  echo "feature content" > conflict2.txt
  git add conflict2.txt
  git commit -m "feature version"
  
  # Switch back and create conflict
  git checkout master
  echo "other master content" > conflict2.txt
  git add conflict2.txt
  git commit -m "other master version"
  
  # Start rebase (will fail)
  git checkout test-feature2
  git rebase master || true
  
  # Save integration state
  mkdir -p "$STATE_DIR"
  cat > "$STATE_DIR/integration_conflict.state" <<EOF
FEATURE_BRANCH=test-feature2
BASE_BRANCH=master
COMMIT_MSG=test rebase 2
EOF
  
  # Don't resolve conflicts - try continue
  run handle_continue_command "continue"
  
  [ "$status" -eq 1 ]
  [[ "$output" =~ "unresolved conflicts remain" ]]
  [[ "$output" =~ "conflict2.txt" ]]
}

@test "integration should update base branch from remote if available" {
  # Create a bare remote repository
  REMOTE_REPO=$(mktemp -d)
  cd "$REMOTE_REPO"
  git init --bare
  
  # Add remote to test repo
  cd "$TEST_REPO_ROOT"
  git remote add origin "$REMOTE_REPO"
  git push -u origin master
  
  # Create a session and make changes
  create_new_session "remote-test" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "remote test content" > remote_test.txt
  git add remote_test.txt
  git commit -m "add remote test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test integration (should try to pull from remote)
  run handle_finish_command "finish" "remote commit" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "updating master with latest changes" ]]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Cleanup
  rm -rf "$REMOTE_REPO"
}

@test "integration should handle case where base branch has no remote tracking" {
  # Create a session and make changes (no remote configured)
  create_new_session "no-remote-test" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "no remote content" > no_remote.txt
  git add no_remote.txt
  git commit -m "add no remote test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test integration
  run handle_finish_command "finish" "no remote commit" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "no remote configured" ]]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify changes are integrated
  git checkout master
  [ -f "no_remote.txt" ]
}

@test "integration should clean up feature branch after successful merge" {
  # Create a session and make changes
  create_new_session "cleanup-test" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Get the feature branch name
  feature_branch="$TEMP_BRANCH"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "cleanup test" > cleanup.txt
  git add cleanup.txt
  git commit -m "add cleanup test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Verify feature branch exists before integration
  git branch | grep -q "$feature_branch"
  
  # Test integration
  run handle_finish_command "finish" "cleanup commit" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "cleaned up feature branch" ]]
  
  # Verify feature branch was deleted
  ! git branch | grep -q "$feature_branch"
  
  # But changes should be on master
  git checkout master
  [ -f "cleanup.txt" ]
}

@test "finish with both --branch and --integrate should work together" {
  # Create a session and make changes
  create_new_session "combined-test" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "combined test" > combined.txt
  git add combined.txt
  git commit -m "add combined test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test with both flags
  run handle_finish_command "finish" "combined commit" "--branch" "feature/combined" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify changes are on master
  git checkout master
  [ -f "combined.txt" ]
  
  # Verify the custom branch name was used and then cleaned up
  ! git branch | grep -q "feature/combined"
}

@test "integration with non-fast-forward scenario" {
  # Test scenario where master has moved ahead but rebase can still succeed
  # Create a session and make changes
  create_new_session "non-ff-test" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  feature_branch="$TEMP_BRANCH"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "feature content" > feature_file.txt
  git add feature_file.txt
  git commit -m "feature changes"
  
  # Add non-conflicting changes to master (moves master ahead)
  cd "$REPO_ROOT"
  git checkout master
  echo "master content" > master_file.txt
  git add -f master_file.txt  # Force add to bypass .gitignore
  git commit -m "master advances"
  
  # Test integration - should succeed with rebase (not fast-forward)
  run handle_finish_command "finish" "non-ff commit" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify feature file is on master after integration
  git checkout master
  [ -f "feature_file.txt" ]
  [ "$(cat feature_file.txt)" = "feature content" ]
  
  # Verify feature branch was cleaned up
  ! git branch | grep -q "$feature_branch"
}

@test "integration state functions work correctly" {
  # Test save_integration_state function
  mkdir -p "$STATE_DIR"
  save_integration_state "test-feature" "master" "test_message"
  
  # Verify state file was created
  [ -f "$STATE_DIR/integration_conflict.state" ]
  
  # Verify state file content
  grep -q "FEATURE_BRANCH=test-feature" "$STATE_DIR/integration_conflict.state"
  grep -q "BASE_BRANCH=master" "$STATE_DIR/integration_conflict.state"
  grep -q "COMMIT_MSG=test_message" "$STATE_DIR/integration_conflict.state"
  
  # Test load_integration_state function
  run load_integration_state
  [ "$status" -eq 0 ]
  
  # Load the state into current shell
  load_integration_state
  
  # Verify variables were loaded correctly
  [ "$FEATURE_BRANCH" = "test-feature" ]
  [ "$BASE_BRANCH" = "master" ]
  [ "$COMMIT_MSG" = "test_message" ]
  
  # Test clear_integration_state function
  clear_integration_state
  [ ! -f "$STATE_DIR/integration_conflict.state" ]
  
  # Test load after clear fails
  run load_integration_state
  [ "$status" -eq 1 ]
}

@test "successful rebase deletes temporary branches properly" {
  # Create a session and make changes
  create_new_session "temp-cleanup" "" false
  session_id=$(ls "$STATE_DIR"/*.state 2>/dev/null | head -1 | xargs basename | sed 's/\.state$//')
  get_session_info "$session_id"
  feature_branch="$TEMP_BRANCH"
  
  # Verify feature branch exists before integration
  git branch | grep -q "$feature_branch"
  
  # Make changes in the worktree
  cd "$WORKTREE_DIR"
  echo "temp cleanup test" > temp_test.txt
  git add temp_test.txt
  git commit -m "add temp test"
  
  # Go back to main repo
  cd "$REPO_ROOT"
  
  # Test integration
  run handle_finish_command "finish" "temp cleanup" "--integrate"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "successfully integrated into" ]]
  
  # Verify no temporary rebase branches remain
  ! git branch | grep -q "temp-rebase-"
  
  # Verify feature branch was cleaned up
  ! git branch | grep -q "$feature_branch"
  
  # Verify only master branch remains
  [ "$(git branch | wc -l)" -eq 1 ]
  git branch | grep -q "^\* master$"
}

@test "continue command cleans up temporary branches after successful rebase" {
  # Create a conflicting situation
  echo "master for temp test" > temp_conflict.txt
  git add temp_conflict.txt
  git commit -m "master temp"
  
  # Create feature branch with conflict manually
  git checkout -b manual-feature
  echo "feature temp content" > temp_conflict.txt
  git add temp_conflict.txt
  git commit -m "feature temp"
  
  # Switch back and create conflict
  git checkout master
  echo "master temp conflict" > temp_conflict.txt
  git add temp_conflict.txt
  git commit -m "master temp conflict"
  
  # Start rebase manually (will fail)
  git checkout manual-feature
  git rebase master || true
  
  # Save integration state
  mkdir -p "$STATE_DIR"
  save_integration_state "manual-feature" "master" "temp test"
  
  # Resolve conflicts
  echo "resolved temp content" > temp_conflict.txt
  git add temp_conflict.txt
  
  # Test continue command
  run handle_continue_command "continue"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "conflicts resolved and rebase completed" ]]
  [[ "$output" =~ "cleaned up feature branch" ]]
  
  # Verify no temporary branches remain
  ! git branch | grep -q "temp-rebase-"
  
  # Verify original feature branch was cleaned up
  ! git branch | grep -q "manual-feature"
}