#!/usr/bin/env bats

# Unit tests for finish_session function failure modes
# Tests error handling and edge cases that could leave the repository in bad state

# Source common test functions  
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    # Set up test environment
    export TEST_DIR="$(pwd)"
    export LIB_DIR="$TEST_DIR/lib"
}

teardown() {
    teardown_temp_git_repo
}

# Tests for finish_session failure modes and error handling
@test "finish_session handles git add failure gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a file that will cause git add to fail (simulate read-only filesystem)
    cd "$worktree_dir"
    echo "new content" > new-file.txt
    
    # Mock git add to fail
    git() {
        if [ "$1" = "add" ] && [ "$2" = "-A" ]; then
            echo "git: error: cannot add files" >&2
            return 1
        else
            command git "$@"
        fi
    }
    export -f git
    
    cd "$TEST_REPO"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to stage changes" ]]
}

@test "finish_session handles git commit failure gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a file that will be staged but fail to commit
    cd "$worktree_dir"
    echo "new content" > new-file.txt
    
    # Mock git to fail only on commit
    git() {
        if [ "$1" = "commit" ] && [ "$2" = "-m" ]; then
            echo "git: error: cannot commit" >&2
            return 1
        else
            command git "$@"
        fi
    }
    export -f git
    
    cd "$TEST_REPO"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to commit changes" ]]
}

@test "finish_session handles git reset failure gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Create multiple commits to trigger squashing
    cd "$worktree_dir"
    echo "commit 1" > file1.txt
    git add file1.txt
    git commit -m "First commit"
    
    echo "commit 2" > file2.txt
    git add file2.txt
    git commit -m "Second commit"
    
    # Mock git to fail only on reset --soft
    git() {
        if [ "$1" = "reset" ] && [ "$2" = "--soft" ]; then
            echo "git: error: cannot reset" >&2
            return 1
        else
            command git "$@"
        fi
    }
    export -f git
    
    cd "$TEST_REPO"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Squashed commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to reset to base branch for squashing" ]]
}

@test "finish_session handles squash commit failure gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Create multiple commits to trigger squashing
    cd "$worktree_dir"
    echo "commit 1" > file1.txt
    git add file1.txt
    git commit -m "First commit"
    
    echo "commit 2" > file2.txt
    git add file2.txt
    git commit -m "Second commit"
    
    # Mock git to fail only on the squash commit (but not the regular commit)
    git() {
        if [ "$1" = "commit" ] && [ "$2" = "-m" ] && [ "$3" = "Squashed commit message" ]; then
            echo "git: error: cannot create squash commit" >&2
            return 1
        else
            command git "$@"
        fi
    }
    export -f git
    
    cd "$TEST_REPO"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Squashed commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to create squash commit" ]]
}

@test "finish_session handles branch rename failure gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    custom_branch_name="feature/custom-auth"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a commit in the worktree
    cd "$worktree_dir"
    echo "custom feature" > custom.txt
    git add custom.txt
    git commit -m "Custom feature commit"
    
    # Mock git branch to fail on rename
    git() {
        if [ "$1" = "branch" ] && [ "$2" = "-m" ]; then
            echo "git: error: cannot rename branch" >&2
            return 1
        else
            command git "$@"
        fi
    }
    export -f git
    
    cd "$TEST_REPO"
    
    # Run finish_session with custom branch name and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Custom commit message" "$custom_branch_name"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to rename branch to 'feature/custom-auth'" ]]
}

@test "finish_session handles invalid worktree directory gracefully" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Use a non-existent worktree directory
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/nonexistent-session"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to change to worktree directory" ]]
}

@test "finish_session with corrupted git repository" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a file and commit it first, then corrupt
    cd "$worktree_dir"
    echo "test content" > test.txt
    git add test.txt
    git commit -m "Initial test commit"
    
    # Corrupt the git repository by making .git directory unreadable
    chmod 000 .git
    
    cd "$TEST_REPO"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit message"
    [ "$status" -ne 0 ]
    
    # Restore permissions for cleanup
    chmod -R 755 "$worktree_dir/.git" 2>/dev/null || true
}

@test "finish_session handles permission denied on directory change" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Remove read permissions from the worktree directory
    chmod 000 "$worktree_dir"
    
    # Run finish_session and expect it to fail gracefully
    run finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Test commit message"
    [ "$status" -ne 0 ]
    [[ "$output" =~ "failed to change to worktree directory" ]]
    
    # Restore permissions for cleanup
    chmod 755 "$worktree_dir"
}

@test "finish_session with session containing only staged changes" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add only staged changes (no unstaged or untracked)
    cd "$worktree_dir"
    echo "staged content" > staged-file.txt
    git add staged-file.txt
    
    cd "$TEST_REPO"
    
    # Run finish_session
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Staged changes commit")
    
    [[ "$result" =~ "staging all changes" ]]
    [[ "$result" =~ "committing changes" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify the commit was created
    cd "$worktree_dir"
    [ "$(git log --oneline -1 --format=%s)" = "Staged changes commit" ]
}

@test "finish_session with session containing submodules" {
    setup_temp_git_repo
    cd "$TEST_REPO"
    
    # Source git functions to test
    . "$LIB_DIR/para-git.sh"
    . "$LIB_DIR/para-utils.sh"
    . "$LIB_DIR/para-config.sh"
    
    # Initialize paths
    need_git_repo
    load_config
    init_paths
    
    # Get base branch name
    base_branch=$(git rev-parse --abbrev-ref HEAD)
    
    # Create a worktree directory to test in
    temp_branch="pc/test-20240531-120000"
    worktree_dir="$TEST_REPO/subtrees/para/test-session"
    mkdir -p "$worktree_dir"
    git worktree add -b "$temp_branch" "$worktree_dir" HEAD
    
    # Add a fake submodule structure (just a .gitmodules file)
    cd "$worktree_dir"
    echo '[submodule "test-sub"]' > .gitmodules
    echo '    path = test-sub' >> .gitmodules
    echo '    url = https://example.com/test.git' >> .gitmodules
    mkdir test-sub
    echo "submodule content" > test-sub/README.md
    
    cd "$TEST_REPO"
    
    # Run finish_session - it should handle submodules properly
    result=$(finish_session "$temp_branch" "$worktree_dir" "$base_branch" "Submodule changes commit")
    
    [[ "$result" =~ "staging all changes" ]]
    [[ "$result" =~ "committing changes" ]]
    [[ "$result" =~ "Session finished successfully!" ]]
    
    # Verify the files were committed (submodule content will be committed as regular files)
    cd "$worktree_dir"
    git ls-files | grep -q ".gitmodules"
    git ls-files | grep -q "test-sub/README.md"
}