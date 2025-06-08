#!/usr/bin/env sh
# Git operations for para

# Find repository root and set up Git environment
need_git_repo() {
  # First check if we're in a git repository at all
  git rev-parse --git-dir >/dev/null 2>&1 || die_repo_state "not in a Git repository"

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
    REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || die_repo_state "could not find repository root"
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

  # Assert paths are initialized before using them
  assert_paths_initialized

  mkdir -p "$SUBTREES_DIR"
  setup_gitignore

  git -C "$REPO_ROOT" worktree add -b "$temp_branch" "$worktree_dir" HEAD >&2 || die_git_operation "git worktree add failed"
}

# Remove worktree and branch
remove_worktree() {
  temp_branch="$1"
  worktree_dir="$2"

  # Optimize: Use --force to skip confirmation and reduce I/O
  # Use 2>/dev/null to suppress expected error output and speed up execution
  git -C "$REPO_ROOT" worktree remove --force "$worktree_dir" 2>/dev/null || true

  # Optimize: Use -D instead of -d to force delete without merge checks (faster)
  git -C "$REPO_ROOT" branch -D "$temp_branch" 2>/dev/null || true
}

# Remove worktree but preserve branch for backup
remove_worktree_preserve_branch() {
  temp_branch="$1"
  worktree_dir="$2"

  # Remove only the worktree, keep the branch for backup recovery
  git -C "$REPO_ROOT" worktree remove --force "$worktree_dir" 2>/dev/null || true
}

# Get current branch name
get_current_branch() {
  git -C "$REPO_ROOT" symbolic-ref --quiet --short HEAD || die_repo_state "detached HEAD; set BASE_BRANCH env"
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
  target_branch_name="$5" # Optional custom target branch name
  integrate="${6:-false}" # Optional integration flag

  # Save current directory for restoration
  ORIGINAL_DIR="$PWD"

  cd "$worktree_dir" || die_repo_state "failed to change to worktree directory: $worktree_dir"

  # Always commit any uncommitted changes first
  if ! git diff --quiet --exit-code --ignore-submodules -- || ! git diff --quiet --exit-code --cached --ignore-submodules -- || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "▶ staging all changes"
    git add -A || die_git_operation "failed to stage changes"
    echo "▶ committing changes"
    git commit -m "$commit_msg" || die_git_operation "failed to commit changes"
  else
    echo "ℹ️  no changes to commit"
  fi

  # Squash all commits into a single commit
  echo "▶ squashing commits into single commit"
  # Count commits to squash (everything after base branch)
  COMMIT_COUNT=$(git rev-list --count HEAD ^"$base_branch")
  if [ "$COMMIT_COUNT" -gt 1 ]; then
    # Reset to squash all commits, but keep the changes staged
    git reset --soft "$base_branch" || die_git_operation "failed to reset to base branch for squashing"
    git commit -m "$commit_msg" || die_git_operation "failed to create squash commit"
    echo "✅ squashed $COMMIT_COUNT commits into single commit with message: $commit_msg"
  else
    echo "ℹ️  only one commit, no squashing needed"
  fi

  # Handle custom branch name if provided
  final_branch_name="$temp_branch"
  if [ -n "$target_branch_name" ]; then
    # Generate unique name in case of conflicts
    unique_branch_name=$(generate_unique_branch_name "$target_branch_name")

    if [ "$unique_branch_name" != "$target_branch_name" ]; then
      echo "⚠️  branch '$target_branch_name' already exists, using '$unique_branch_name' instead"
    fi

    echo "▶ renaming branch from '$temp_branch' to '$unique_branch_name'"
    git branch -m "$unique_branch_name" || die_git_operation "failed to rename branch to '$unique_branch_name'"
    final_branch_name="$unique_branch_name"
  fi

  # Handle integration if requested
  if [ "$integrate" = "true" ]; then
    # Change to repository root for integration
    cd "$REPO_ROOT" || die_repo_state "failed to change to repository root: $REPO_ROOT"

    # Perform integration
    if integrate_branch "$final_branch_name" "$base_branch" "$commit_msg"; then
      echo "✅ integration successful"
      # Note: Feature branch cleanup will happen after worktree removal
    else
      # Integration failed (conflicts) - save state for continue command
      save_integration_state "$final_branch_name" "$base_branch" "$commit_msg"
      cd "$ORIGINAL_DIR" || true
      return 1
    fi
  else
    # Change back to repository root
    cd "$REPO_ROOT" || die_repo_state "failed to change to repository root: $REPO_ROOT"

    # Success - return information about the branch
    echo ""
    echo "🎉 Session finished successfully!"
    echo ""
    echo "📋 Next steps:"
    echo "   Your changes are ready on branch: $final_branch_name"
    echo "   Branch location: local"
    echo ""
    echo "   To merge your changes:"
    echo "   git checkout $base_branch"
    echo "   git merge $final_branch_name"
    echo ""
    echo "   Or create a pull/merge request if using a remote repository."
    echo ""
    echo "   After merging, clean up the branch:"
    echo "   git branch -d $final_branch_name"
    echo ""
  fi

  # Restore original directory
  cd "$ORIGINAL_DIR" || true

  return 0
}

# Get configurable branch prefix, defaulting to "para"
get_branch_prefix() {
  if [ -n "${BRANCH_PREFIX:-}" ]; then
    echo "${BRANCH_PREFIX}"
  elif [ -n "${PARA_BRANCH_PREFIX:-}" ]; then
    echo "${PARA_BRANCH_PREFIX}"
  else
    echo "para"
  fi
}

# Validate branch prefix for Git compatibility
validate_branch_prefix() {
  prefix="$1"

  # Check for empty prefix
  if [ -z "$prefix" ]; then
    return 1
  fi

  # Check for invalid characters that Git doesn't allow in branch names
  # Git branch names cannot contain: space, ~, ^, :, ?, *, [, \, @, .., @{, //, /end
  # Order patterns from most specific to least specific to avoid override warnings
  case "$prefix" in
  *..* | *@\{* | *//* | */ | .*)
    return 1
    ;;
  *\ * | *~* | *^* | *:* | *\?* | *\** | *\[* | *\\* | *@*)
    return 1
    ;;
  *)
    return 0
    ;;
  esac
}

# Generate clean branch name from session name
generate_clean_branch_name() {
  session_name="$1"

  # Convert to lowercase and replace spaces/underscores with hyphens
  clean_name=$(echo "$session_name" | tr '[:upper:]' '[:lower:]' | tr ' _' '--' | tr -s '-')

  # Remove any invalid characters for Git branch names
  clean_name=$(echo "$clean_name" | sed 's/[^a-z0-9-]//g')

  # Ensure it doesn't start or end with a hyphen
  clean_name=$(echo "$clean_name" | sed 's/^-*//;s/-*$//')

  # If empty after cleaning, use "unnamed"
  if [ -z "$clean_name" ]; then
    clean_name="unnamed"
  fi

  echo "$clean_name"
}

# Generate target branch name with prefix
generate_target_branch_name() {
  session_name="$1"
  prefix=$(get_branch_prefix)

  if [ -z "$session_name" ]; then
    session_name="unnamed"
  fi

  clean_name=$(generate_clean_branch_name "$session_name")
  echo "${prefix}/${clean_name}"
}

# Validate target branch name for Git compatibility
validate_target_branch_name() {
  branch_name="$1"

  # Check for empty branch name
  if [ -z "$branch_name" ]; then
    die_invalid_args "branch name cannot be empty"
  fi

  # Check for spaces
  case "$branch_name" in
  *\ *)
    die_invalid_args "invalid branch name '$branch_name': contains spaces"
    ;;
  esac

  # Check for other invalid characters
  case "$branch_name" in
  *~* | *^* | *:* | *\?* | *\** | *\[* | *\\*)
    die_invalid_args "invalid branch name '$branch_name': contains invalid characters"
    ;;
  esac

  # Check for @ character (but not @{ which is checked separately)
  case "$branch_name" in
  *@*)
    # Check if it's the @{ pattern
    case "$branch_name" in
    *@\{*) ;; # This will be caught later
    *) die "invalid branch name '$branch_name': contains invalid characters" ;;
    esac
    ;;
  esac

  # Check for invalid sequences
  case "$branch_name" in
  *..* | *@\{* | *//*)
    die_invalid_args "invalid branch name '$branch_name': contains invalid sequences"
    ;;
  esac

  # Check for invalid start characters
  case "$branch_name" in
  -* | .*)
    die_invalid_args "invalid branch name '$branch_name': cannot start with '-' or '.'"
    ;;
  esac

  # Check for invalid end characters
  case "$branch_name" in
  */)
    die_invalid_args "invalid branch name '$branch_name': cannot end with '/'"
    ;;
  esac

  # Check for specific invalid patterns
  case "$branch_name" in
  . | /)
    die_invalid_args "invalid branch name '$branch_name': invalid name"
    ;;
  */.*)
    die_invalid_args "invalid branch name '$branch_name': cannot contain '/.' sequence"
    ;;
  esac

  return 0
}

# Generate unique branch name by adding suffix if conflicts exist
generate_unique_branch_name() {
  target_branch="$1"

  # Check if branch already exists
  if ! git -C "$REPO_ROOT" rev-parse --verify "$target_branch" >/dev/null 2>&1; then
    # Branch doesn't exist, use as-is
    echo "$target_branch"
    return 0
  fi

  # Branch exists, need to find unique name with suffix
  counter=1
  while [ "$counter" -le 999 ]; do
    candidate="${target_branch}-${counter}"
    if ! git -C "$REPO_ROOT" rev-parse --verify "$candidate" >/dev/null 2>&1; then
      echo "$candidate"
      return 0
    fi
    counter=$((counter + 1))
  done

  # If we get here, we couldn't find a unique name (highly unlikely)
  die_repo_state "unable to generate unique branch name after 999 attempts"
}

# Check if a branch is currently checked out in any worktree
is_branch_in_worktree() {
  branch_name="$1"
  git -C "$REPO_ROOT" worktree list --porcelain | grep -q "branch refs/heads/$branch_name"
}

# Validate repository state for safe integration
validate_repo_state() {
  # Run fsck and filter out harmless dangling objects
  fsck_output=$(git -C "$REPO_ROOT" fsck 2>&1)
  fsck_status=$?

  # Filter out dangling objects (normal in repositories with complex histories)
  critical_errors=$(echo "$fsck_output" | grep -v "^dangling " | grep -v "^Checking " | grep -v "^$")

  if [ $fsck_status -ne 0 ] && [ -n "$critical_errors" ]; then
    echo "Critical repository integrity issues detected:" >&2
    echo "$critical_errors" >&2
    die_repo_state "repository integrity check failed - unsafe to proceed with integration"
  fi
}

# Integrate feature branch into base branch using rebase
integrate_branch() {
  feature_branch="$1"
  base_branch="$2"
  commit_msg="$3"

  echo "▶ updating $base_branch with latest changes..."

  # Validate repository state before proceeding
  validate_repo_state

  # Check if base branch is checked out in a worktree
  if is_branch_in_worktree "$base_branch"; then
    echo "  ⚠️  base branch $base_branch is checked out in a worktree"
    echo "  → using worktree-safe integration (bypasses Git's checkout protection)"
    echo "  → this is safe but may cause working tree desynchronization"

    # Check if we have a remote configured and try to update base branch reference
    if git -C "$REPO_ROOT" remote >/dev/null 2>&1; then
      # Check if base branch tracks a remote
      upstream_ref="${base_branch}@{upstream}"
      if git -C "$REPO_ROOT" rev-parse --verify "$upstream_ref" >/dev/null 2>&1; then
        echo "  → fetching latest changes from remote"
        if git -C "$REPO_ROOT" fetch origin "$base_branch:$base_branch" 2>/dev/null; then
          echo "  → updated base branch from remote"
        else
          echo "  → could not update base branch from remote, continuing with local"
        fi
      else
        echo "  → no remote tracking branch, using local $base_branch"
      fi
    else
      echo "  → no remote configured, using local $base_branch"
    fi
  else
    # Standard approach when base branch is not in a worktree
    # Check if we have a remote configured
    if git -C "$REPO_ROOT" remote >/dev/null 2>&1; then
      # Try to pull latest changes from remote if available
      git -C "$REPO_ROOT" checkout "$base_branch" >/dev/null 2>&1 || die_git_operation "failed to checkout $base_branch"

      # Check if base branch tracks a remote
      if git -C "$REPO_ROOT" rev-parse --verify "@{upstream}" >/dev/null 2>&1; then
        echo "  → pulling latest changes from remote"
        if ! git -C "$REPO_ROOT" pull; then
          echo "⚠️  failed to pull latest changes from remote"
          echo "   continuing with local integration"
        fi
      else
        echo "  → no remote tracking branch, using local $base_branch"
      fi
    else
      echo "  → no remote configured, using local $base_branch"
      git -C "$REPO_ROOT" checkout "$base_branch" >/dev/null 2>&1 || die_git_operation "failed to checkout $base_branch"
    fi
  fi

  echo "▶ integrating changes into $base_branch using rebase..."

  # Get the commit hash of the feature branch
  feature_commit=$(git -C "$REPO_ROOT" rev-parse "$feature_branch") || die_git_operation "failed to get feature branch commit"

  # Create a temporary branch from the feature commit for rebasing
  temp_rebase_branch="temp-rebase-$(date +%s)"
  git -C "$REPO_ROOT" branch "$temp_rebase_branch" "$feature_commit" || die_git_operation "failed to create temporary rebase branch"

  # Checkout the temporary branch and rebase it onto base branch
  git -C "$REPO_ROOT" checkout "$temp_rebase_branch" >/dev/null 2>&1 || die_git_operation "failed to checkout temporary rebase branch"

  if git -C "$REPO_ROOT" rebase "$base_branch"; then
    # Rebase successful, now update the base branch
    if is_branch_in_worktree "$base_branch"; then
      # Base branch is in a worktree, use git update-ref to update it safely
      rebased_commit=$(git -C "$REPO_ROOT" rev-parse "$temp_rebase_branch") || die_git_operation "failed to get rebased commit"

      # Backup current branch state before update
      current_base_commit=$(git -C "$REPO_ROOT" rev-parse "refs/heads/$base_branch")
      echo "  ⚠️  updating $base_branch using git update-ref (bypassing checkout protection)"
      echo "  → backup of current state: $current_base_commit"

      git -C "$REPO_ROOT" update-ref "refs/heads/$base_branch" "$rebased_commit" ||
        die_git_operation "failed to update base branch reference (backup: $current_base_commit)"

      # Validate the update was successful
      updated_commit=$(git -C "$REPO_ROOT" rev-parse "refs/heads/$base_branch")
      if [ "$updated_commit" != "$rebased_commit" ]; then
        die_git_operation "branch update validation failed - expected $rebased_commit, got $updated_commit"
      fi

      echo "✅ successfully integrated into $base_branch using update-ref"
      echo "  → note: worktrees with $base_branch checked out may need refresh"
    else
      # Standard fast-forward merge
      git -C "$REPO_ROOT" checkout "$base_branch" >/dev/null 2>&1 || die_git_operation "failed to checkout $base_branch after rebase"
      git -C "$REPO_ROOT" merge --ff-only "$temp_rebase_branch" || die_git_operation "failed to fast-forward merge after rebase"
      echo "✅ successfully integrated into $base_branch using rebase"
    fi

    # Clean up temporary branch
    git -C "$REPO_ROOT" branch -D "$temp_rebase_branch" 2>/dev/null || true

    return 0
  else
    # Rebase failed due to conflicts - clean up and save state
    echo ""
    echo "⚠️  rebase conflicts detected in the following files:"
    git -C "$REPO_ROOT" diff --name-only --diff-filter=U | sed 's/^/  - /'
    echo ""
    echo "▶ opening IDE to resolve conflicts..."

    # Open IDE for conflict resolution
    open_ide_for_conflicts

    echo ""
    echo "Fix conflicts and run: para continue"
    echo "To abort: git rebase --abort"
    echo ""
    return 1
  fi
}

# Save integration state for continue command
save_integration_state() {
  feature_branch="$1"
  base_branch="$2"
  commit_msg="$3"

  # Ensure state directory exists
  mkdir -p "$STATE_DIR"

  # Save integration state
  cat >"$STATE_DIR/integration_conflict.state" <<EOF
FEATURE_BRANCH=$feature_branch
BASE_BRANCH=$base_branch
COMMIT_MSG='$commit_msg'
EOF
}

# Load integration state
load_integration_state() {
  state_file="$STATE_DIR/integration_conflict.state"
  if [ ! -f "$state_file" ]; then
    return 1
  fi

  # Source the state file to load variables
  # shellcheck source=/dev/null
  . "$state_file"
  return 0
}

# Clear integration state
clear_integration_state() {
  rm -f "$STATE_DIR/integration_conflict.state"
}

# Open IDE for conflict resolution
open_ide_for_conflicts() {
  # Get list of conflicted files
  conflicted_files=$(git -C "$REPO_ROOT" diff --name-only --diff-filter=U)

  if [ -n "$conflicted_files" ]; then
    echo "  → opening IDE with conflicted files..."
    # Change to repo root and open IDE
    cd "$REPO_ROOT" || return

    # Use the configured IDE to open conflicted files
    case "$IDE_NAME" in
    "cursor")
      if command -v cursor >/dev/null 2>&1; then
        cursor . $conflicted_files &
      fi
      ;;
    "code")
      if command -v code >/dev/null 2>&1; then
        code . $conflicted_files &
      fi
      ;;
    "claude")
      if command -v claude >/dev/null 2>&1; then
        claude . &
      fi
      ;;
    *)
      echo "  → IDE not configured for conflict resolution"
      ;;
    esac
  fi
}
