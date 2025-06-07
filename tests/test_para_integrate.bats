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

@test "continue command should complete merge after conflict resolution" {
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
  
  # Simulate conflict state by attempting merge
  git checkout master
  git merge test-feature --no-edit || true  # This will fail with conflicts
  
  # Save integration state manually (normally done by finish --integrate)
  mkdir -p "$STATE_DIR"
  cat > "$STATE_DIR/integration_conflict.state" <<EOF
FEATURE_BRANCH=test-feature
BASE_BRANCH=master
COMMIT_MSG=test merge
EOF
  
  # Resolve conflicts manually
  echo "resolved content" > conflict.txt
  git add conflict.txt
  
  # Test continue command
  run handle_continue_command "continue"
  
  [ "$status" -eq 0 ]
  [[ "$output" =~ "conflicts resolved and merge completed" ]]
  [[ "$output" =~ "successfully integrated into master" ]]
  
  # Verify merge was completed
  [ ! -f "$REPO_ROOT/.git/MERGE_HEAD" ]
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
  
  # Start merge (will fail)
  git merge test-feature2 --no-edit || true
  
  # Save integration state
  mkdir -p "$STATE_DIR"
  cat > "$STATE_DIR/integration_conflict.state" <<EOF
FEATURE_BRANCH=test-feature2
BASE_BRANCH=master
COMMIT_MSG=test merge 2
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