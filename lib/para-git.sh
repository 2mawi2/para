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
      } >>"$GIT_EXCLUDE_FILE"

      # Check and add entries separately to avoid pipeline issues
      if ! grep -q "^$SUBTREES_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null; then
        echo "$SUBTREES_ENTRY" >>"$GIT_EXCLUDE_FILE"
      fi
      if ! grep -q "^$STATE_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null; then
        echo "$STATE_ENTRY" >>"$GIT_EXCLUDE_FILE"
      fi
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

# Finish session - commit changes and prepare branch for manual merge
finish_session() {
  temp_branch="$1"
  worktree_dir="$2"
  base_branch="$3"
  commit_msg="$4"
  merge_mode="${5:-squash}" # Default to squash mode if not provided

  # Save current directory for restoration
  ORIGINAL_DIR="$PWD"

  cd "$worktree_dir" || die "failed to change to worktree directory: $worktree_dir"

  # Always commit any uncommitted changes first
  if ! git diff --quiet --exit-code --ignore-submodules -- || ! git diff --quiet --exit-code --cached --ignore-submodules -- || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "▶ staging all changes"
    git add -A || die "failed to stage changes"
    echo "▶ committing changes"
    git commit -m "$commit_msg" || die "failed to commit changes"
  else
    echo "ℹ️  no changes to commit"
  fi

  # If squash mode, squash all commits into a single commit
  if [ "$merge_mode" = "squash" ]; then
    echo "▶ squashing commits into single commit"
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

  # Push branch to remote if configured
  echo "▶ preparing branch for manual merge"
  
  # Check if we have a remote configured
  if git remote | grep -q "origin"; then
    echo "▶ pushing branch to remote"
    if git push origin "$temp_branch"; then
      echo "✅ branch '$temp_branch' pushed to remote"
      BRANCH_LOCATION="remote and local"
    else
      echo "⚠️  failed to push to remote, branch available locally only"
      BRANCH_LOCATION="local"
    fi
  else
    echo "ℹ️  no remote configured, branch available locally"
    BRANCH_LOCATION="local"
  fi

  # Change back to repository root
  cd "$REPO_ROOT" || die "failed to change to repository root: $REPO_ROOT"

  # Restore original directory
  cd "$ORIGINAL_DIR" || true

  # Success - return information about the branch
  echo ""
  echo "🎉 Session finished successfully!"
  echo ""
  echo "📋 Next steps:"
  echo "   Your changes are ready on branch: $temp_branch"
  echo "   Branch location: $BRANCH_LOCATION"
  echo ""
  echo "   To merge your changes:"
  echo "   git checkout $base_branch"
  echo "   git merge $temp_branch"
  echo ""
  echo "   Or create a pull/merge request if using a remote repository."
  echo ""
  echo "   After merging, clean up the branch:"
  echo "   git branch -d $temp_branch"
  if [ "$BRANCH_LOCATION" = "remote and local" ]; then
    echo "   git push origin --delete $temp_branch"
  fi
  echo ""

  return 0
}
