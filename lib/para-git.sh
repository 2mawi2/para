#!/usr/bin/env sh
# Git operations for para

# Find repository root and set up Git environment
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

# Setup gitignore files to ensure para directories are ignored
setup_gitignore() {
  GIT_EXCLUDE_FILE="$REPO_ROOT/.git/info/exclude"
  SUBTREES_GITIGNORE="$SUBTREES_DIR/.gitignore"

  # Entries to ensure are ignored
  SUBTREES_ENTRY="$SUBTREES_DIR_NAME/"
  STATE_ENTRY="$STATE_DIR_NAME/"

  # Setup git exclude file (local to repository, not tracked)
  if [ ! -f "$GIT_EXCLUDE_FILE" ]; then
    echo "▶ creating git exclude file for para directories" >&2
    mkdir -p "$(dirname "$GIT_EXCLUDE_FILE")"
    cat >"$GIT_EXCLUDE_FILE" <<EOF
# para - parallel cursor sessions (local excludes)
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
      echo "▶ updating git exclude file with para entries" >&2
      {
        echo ""
        echo "# para - parallel cursor sessions (local excludes)"
        grep -q "^$SUBTREES_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$SUBTREES_ENTRY"
        grep -q "^$STATE_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$STATE_ENTRY"
      } >>"$GIT_EXCLUDE_FILE"
    fi
  fi

  # Setup subtrees directory .gitignore to ignore all contents
  if [ ! -f "$SUBTREES_GITIGNORE" ]; then
    echo "▶ creating .gitignore in subtrees directory" >&2
    cat >"$SUBTREES_GITIGNORE" <<EOF
# Ignore all para worktree contents
*
!.gitignore
EOF
  fi
}

# Create worktree for session
create_worktree() {
  temp_branch="$1"
  worktree_dir="$2"

  mkdir -p "$SUBTREES_DIR"
  setup_gitignore

  git -C "$REPO_ROOT" worktree add -b "$temp_branch" "$worktree_dir" HEAD >&2 || die "git worktree add failed"
}

# Remove worktree and branch
remove_worktree() {
  temp_branch="$1"
  worktree_dir="$2"

  git -C "$REPO_ROOT" worktree remove --force "$worktree_dir" 2>/dev/null || true
  git -C "$REPO_ROOT" branch -D "$temp_branch" 2>/dev/null || true
}

# Get current branch name
get_current_branch() {
  git -C "$REPO_ROOT" symbolic-ref --quiet --short HEAD || die "detached HEAD; set BASE_BRANCH env"
}

# Check for uncommitted changes
check_uncommitted_changes() {
  HAS_UNCOMMITTED=0

  if ! git -C "$REPO_ROOT" diff --quiet --exit-code; then
    echo "⚠️  Warning: You have uncommitted changes in the working tree:" >&2
    git -C "$REPO_ROOT" status --porcelain >&2
    HAS_UNCOMMITTED=1
  fi

  if ! git -C "$REPO_ROOT" diff --quiet --exit-code --cached; then
    echo "⚠️  Warning: You have staged changes in the working tree:" >&2
    git -C "$REPO_ROOT" status --porcelain >&2
    HAS_UNCOMMITTED=1
  fi

  if [ "$HAS_UNCOMMITTED" -eq 1 ]; then
    echo "💡 Tip: Your new session will start from the last committed state." >&2
    echo "   Any uncommitted changes will not be included in the session." >&2
    echo "" >&2
  fi
}

# Perform merge operations
merge_session() {
  temp_branch="$1"
  worktree_dir="$2"
  base_branch="$3"
  commit_msg="$4"
  merge_mode="${5:-squash}" # Default to squash mode if not provided

  # Save current directory for restoration
  ORIGINAL_DIR="$PWD"

  cd "$worktree_dir" || die "failed to change to worktree directory: $worktree_dir"

  # Always commit any uncommitted changes first (both modes need this)
  if ! git diff --quiet --exit-code --ignore-submodules -- || ! git diff --quiet --exit-code --cached --ignore-submodules -- || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "▶ staging all changes"
    git add -A || die "failed to stage changes"
    echo "▶ committing changes"
    git commit -m "$commit_msg" || die "failed to commit changes"
  else
    echo "ℹ️  no changes to commit"
  fi

  # Always try rebase first to detect conflicts
  echo "▶ rebasing $temp_branch onto $base_branch"
  if ! git rebase "$base_branch"; then
    echo "❌ rebase conflicts" >&2
    echo "   → resolve conflicts in $worktree_dir" >&2
    echo "   → then run: para continue" >&2
    cd "$ORIGINAL_DIR" || true
    return 1
  fi

  # If squash mode, squash all commits after successful rebase
  if [ "$merge_mode" = "squash" ]; then
    echo "▶ squashing all commits into single commit"
    # Count commits to squash (everything after base branch)
    COMMIT_COUNT=$(git rev-list --count HEAD ^"$base_branch")
    if [ "$COMMIT_COUNT" -gt 1 ]; then
      # Reset to squash all commits, but keep the changes staged
      git reset --soft "$base_branch" || die "failed to reset to base branch for squashing"
      git commit -m "$commit_msg" || die "failed to create squash commit"
      echo "✅ squashed $COMMIT_COUNT commits into single commit with message: $commit_msg"
    else
      echo "ℹ️  only one commit, no squashing needed"
    fi
  fi

  # Change to main repository root for merge operations
  cd "$REPO_ROOT" || die "failed to change to repository root: $REPO_ROOT"

  # Verify we're in the correct repository state
  if ! git rev-parse --git-dir >/dev/null 2>&1; then
    die "not in a valid git repository at $REPO_ROOT"
  fi

  # Ensure repository is not bare (safety check)
  if git config --get core.bare | grep -q "true"; then
    echo "⚠️  Warning: Repository is configured as bare, fixing..." >&2
    git config core.bare false || die "failed to fix bare repository setting"
  fi

  echo "▶ merging into $base_branch"
  git checkout "$base_branch" || die "failed to checkout $base_branch"
  if ! git merge --ff-only "$temp_branch" 2>/dev/null; then
    echo "▶ using non-fast-forward merge"
    # For squash mode, we want a clean merge commit message
    # For rebase mode, we want to preserve the individual commits
    if [ "$merge_mode" = "squash" ]; then
      merge_commit_msg="$commit_msg"
    else
      merge_commit_msg="Merge session $temp_branch"
    fi

    if ! git merge --no-ff -m "$merge_commit_msg" "$temp_branch"; then
      echo "❌ merge conflicts – resolve them and complete the merge manually" >&2
      cd "$ORIGINAL_DIR" || true
      return 1
    fi
  fi

  # Restore original directory
  cd "$ORIGINAL_DIR" || true
  return 0
}

# Continue merge after conflict resolution
continue_merge() {
  worktree_dir="$1"
  temp_branch="$2"
  base_branch="$3"
  merge_mode="${4:-squash}" # Default to squash if not provided

  # Save current directory for restoration
  ORIGINAL_DIR="$PWD"

  cd "$worktree_dir" || die "failed to change to worktree directory: $worktree_dir"

  # Check if we're in the middle of a rebase
  GIT_DIR=$(git rev-parse --git-dir)
  if [ -d "$GIT_DIR/rebase-merge" ] || [ -d "$GIT_DIR/rebase-apply" ]; then
    # Check for conflict markers in ALL files, not just git diff --check
    echo "▶ checking for unresolved conflict markers"

    # Search for conflict markers in all files
    CONFLICT_FILES=""
    if command -v grep >/dev/null 2>&1; then
      # Use find + grep to search for conflict markers in all files
      CONFLICT_FILES=$(find . -type f -not -path './.git/*' -exec grep -l "^<<<<<<< \|^=======$\|^>>>>>>> " {} \; 2>/dev/null || true)
    fi

    if [ -n "$CONFLICT_FILES" ]; then
      echo "❌ There are still unresolved conflicts:" >&2
      echo "Files with conflict markers:" >&2
      echo "$CONFLICT_FILES" | sed 's/^/  /' >&2
      echo "" >&2
      echo "Please resolve all conflicts manually and run 'para continue' again." >&2
      cd "$ORIGINAL_DIR" || true
      return 1
    fi

    # Also check git diff --check for whitespace conflict markers
    if ! git diff --check 2>/dev/null; then
      echo "❌ git diff --check found conflict-related issues:" >&2
      git diff --check 2>&1 | sed 's/^/  /' >&2
      cd "$ORIGINAL_DIR" || true
      return 1
    fi

    # Auto-stage all resolved files
    echo "▶ auto-staging resolved conflicts"
    git add -A || die "failed to stage resolved conflicts"

    echo "▶ continuing rebase"
    if ! GIT_EDITOR=true git rebase --continue; then
      echo "❌ rebase continue failed – check for remaining conflicts" >&2
      cd "$ORIGINAL_DIR" || true
      return 1
    fi
    echo "✅ rebase completed"
  else
    echo "ℹ️  No rebase in progress"
  fi

  # Change to main repository root for merge operations
  cd "$REPO_ROOT" || die "failed to change to repository root: $REPO_ROOT"

  # Verify we're in the correct repository state
  if ! git rev-parse --git-dir >/dev/null 2>&1; then
    die "not in a valid git repository at $REPO_ROOT"
  fi

  # Ensure repository is not bare (safety check)
  if git config --get core.bare | grep -q "true"; then
    echo "⚠️  Warning: Repository is configured as bare, fixing..." >&2
    git config core.bare false || die "failed to fix bare repository setting"
  fi

  # Continue with the merge process
  echo "▶ merging into $base_branch"
  git checkout "$base_branch" || die "failed to checkout $base_branch"
  if ! git merge --ff-only "$temp_branch" 2>/dev/null; then
    echo "▶ using non-fast-forward merge"
    # Use appropriate merge commit message based on mode
    if [ "$merge_mode" = "squash" ]; then
      COMMIT_MSG="Merge session after resolving conflicts"
    else
      COMMIT_MSG="Merge session $temp_branch after resolving conflicts"
    fi

    if ! git merge --no-ff -m "$COMMIT_MSG" "$temp_branch"; then
      echo "❌ merge conflicts – resolve them and complete the merge manually" >&2
      cd "$ORIGINAL_DIR" || true
      return 1
    fi
  fi

  # Restore original directory
  cd "$ORIGINAL_DIR" || true
  return 0
}
