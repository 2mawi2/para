#!/usr/bin/env sh
# Git operations for pursor

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

# Setup gitignore files to ensure pursor directories are ignored
setup_gitignore() {
  GIT_EXCLUDE_FILE="$REPO_ROOT/.git/info/exclude"
  SUBTREES_GITIGNORE="$SUBTREES_DIR/.gitignore"
  
  # Entries to ensure are ignored
  SUBTREES_ENTRY="$SUBTREES_DIR_NAME/"
  STATE_ENTRY="$STATE_DIR_NAME/"
  
  # Setup git exclude file (local to repository, not tracked)
  if [ ! -f "$GIT_EXCLUDE_FILE" ]; then
    echo "▶ creating git exclude file for pursor directories" >&2
    mkdir -p "$(dirname "$GIT_EXCLUDE_FILE")"
    cat > "$GIT_EXCLUDE_FILE" <<EOF
# pursor - parallel cursor sessions (local excludes)
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
      echo "▶ updating git exclude file with pursor entries" >&2
      {
        echo ""
        echo "# pursor - parallel cursor sessions (local excludes)"
        grep -q "^$SUBTREES_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$SUBTREES_ENTRY"
        grep -q "^$STATE_ENTRY\$" "$GIT_EXCLUDE_FILE" 2>/dev/null || echo "$STATE_ENTRY"
      } >> "$GIT_EXCLUDE_FILE"
    fi
  fi
  
  # Setup subtrees directory .gitignore to ignore all contents
  if [ ! -f "$SUBTREES_GITIGNORE" ]; then
    echo "▶ creating .gitignore in subtrees directory" >&2
    cat > "$SUBTREES_GITIGNORE" <<EOF
# Ignore all pursor worktree contents
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
  
  # Auto-stage and commit all changes if any exist
  cd "$worktree_dir"
  if ! git diff --quiet --exit-code --ignore-submodules -- || ! git diff --quiet --exit-code --cached --ignore-submodules -- || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "▶ staging all changes"
    git add -A || die "failed to stage changes"
    echo "▶ committing changes"
    git commit -m "$commit_msg" || die "failed to commit changes"
  else
    echo "ℹ️  no changes to commit"
  fi
  cd "$REPO_ROOT"

  echo "▶ rebasing $temp_branch onto $base_branch"
  if ! git -C "$worktree_dir" rebase "$base_branch"; then
    echo "❌ rebase conflicts" >&2
    echo "   → resolve conflicts in $worktree_dir" >&2
    echo "   → then run: pursor continue" >&2
    return 1
  fi

  echo "▶ merging into $base_branch"
  git -C "$REPO_ROOT" checkout "$base_branch"
  if ! git -C "$REPO_ROOT" merge --ff-only "$temp_branch" 2>/dev/null; then
    echo "▶ using non-fast-forward merge"
    if ! git -C "$REPO_ROOT" merge --no-ff -m "$commit_msg" "$temp_branch"; then
      echo "❌ merge conflicts – resolve them and complete the merge manually" >&2
      return 1
    fi
  fi
  
  return 0
}

# Continue merge after conflict resolution
continue_merge() {
  worktree_dir="$1"
  temp_branch="$2"
  base_branch="$3"
  
  cd "$worktree_dir"
  
  # Check if we're in the middle of a rebase
  GIT_DIR=$(git rev-parse --git-dir)
  if [ -d "$GIT_DIR/rebase-merge" ] || [ -d "$GIT_DIR/rebase-apply" ]; then
    # Check for remaining conflict markers in any files
    if git diff --check 2>/dev/null; then
      # No conflict markers found, auto-stage resolved files
      echo "▶ auto-staging resolved conflicts"
      git add -u || die "failed to stage resolved conflicts"
    else
      echo "❌ There are still unresolved conflicts:" >&2
      echo "Files with conflict markers:" >&2
      git diff --check 2>&1 || true
      return 1
    fi
    
    echo "▶ continuing rebase"
    if ! GIT_EDITOR=true git rebase --continue; then
      echo "❌ rebase continue failed – check for remaining conflicts" >&2
      return 1
    fi
    echo "✅ rebase completed"
  else
    echo "ℹ️  No rebase in progress"
  fi
  
  cd "$REPO_ROOT"
  
  # Continue with the merge process
  echo "▶ merging into $base_branch"
  git -C "$REPO_ROOT" checkout "$base_branch"
  if ! git -C "$REPO_ROOT" merge --ff-only "$temp_branch" 2>/dev/null; then
    echo "▶ using non-fast-forward merge"
    COMMIT_MSG="Merge session after resolving conflicts"
    if ! git -C "$REPO_ROOT" merge --no-ff -m "$COMMIT_MSG" "$temp_branch"; then
      echo "❌ merge conflicts – resolve them and complete the merge manually" >&2
      return 1
    fi
  fi
  
  return 0
} 