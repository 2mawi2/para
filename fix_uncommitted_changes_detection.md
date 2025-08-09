# Fix Uncommitted Changes Detection in Para List Command

## Problem
The `para list --json` command incorrectly reports `has_uncommitted_changes: false` for sessions that actually have uncommitted changes. This causes issues in para-ui where:
1. The UI doesn't show warnings about uncommitted changes
2. Cancel operations fail with "Cannot cancel session with uncommitted changes in non-interactive mode"

## Current Behavior
```bash
# Create a session with uncommitted changes
para start test-session -p "test"
echo "test" > /path/to/worktree/test.txt

# List shows incorrect status
para list --json | jq '.[] | select(.session_id == "test-session") | .has_uncommitted_changes'
# Output: false (INCORRECT!)

# But cancel correctly detects uncommitted changes
para cancel test-session
# Error: Cannot cancel session with uncommitted changes in non-interactive mode
```

## Root Cause
The `has_uncommitted_changes` field in the session list is not properly checking the git status of the worktree. It needs to:
1. Check both staged and unstaged changes
2. Check for untracked files
3. Properly handle worktree paths

## Required Fix

### Find the code that populates `has_uncommitted_changes`
Look for where `SessionInfo` or similar struct is populated in the list command. It's likely in one of these files:
- `src/cli/commands/list/mod.rs`
- `src/core/session.rs`
- `src/core/git.rs`

### Update the git status check
The code should properly check git status in the worktree:

```rust
// Pseudo-code for the fix needed
fn check_uncommitted_changes(worktree_path: &Path) -> bool {
    // Run git status in the worktree
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()
        .unwrap_or_else(|_| return false);
    
    // If output is not empty, there are uncommitted changes
    !output.stdout.is_empty()
}
```

Or use the existing git library methods if available:
```rust
// If using git2 crate
fn has_uncommitted_changes(repo: &Repository) -> bool {
    let statuses = repo.statuses(None).unwrap();
    !statuses.is_empty()
}
```

## Test Cases
After fixing, verify these scenarios:

1. **Clean worktree**
   ```bash
   para start clean-session
   para list --json | jq '.[] | select(.session_id == "clean-session") | .has_uncommitted_changes'
   # Should output: false
   ```

2. **Worktree with unstaged changes**
   ```bash
   para start unstaged-session
   echo "test" > test.txt
   para list --json | jq '.[] | select(.session_id == "unstaged-session") | .has_uncommitted_changes'
   # Should output: true
   ```

3. **Worktree with staged changes**
   ```bash
   para start staged-session
   echo "test" > test.txt
   git add test.txt
   para list --json | jq '.[] | select(.session_id == "staged-session") | .has_uncommitted_changes'
   # Should output: true
   ```

4. **Worktree with committed changes only**
   ```bash
   para start committed-session
   echo "test" > test.txt
   git add test.txt
   git commit -m "test"
   para list --json | jq '.[] | select(.session_id == "committed-session") | .has_uncommitted_changes'
   # Should output: false
   ```

## Files Likely to Need Changes
1. Look for where `SessionInfo` struct is defined and populated
2. Find the list command implementation
3. Check if there's a shared git status checking function that might be broken
4. Update the logic to properly detect uncommitted changes in worktrees

## Success Criteria
- `para list --json` correctly reports `has_uncommitted_changes: true` when a session's worktree has any uncommitted changes (staged, unstaged, or untracked files)
- `para list --json` correctly reports `has_uncommitted_changes: false` only when the worktree is clean
- The detection works consistently with what `para cancel` checks
- All test cases pass

## Additional Context
This is blocking proper UX in para-ui where users can't see if their sessions have uncommitted changes, and the cancel button fails unexpectedly. The fix should be straightforward - just ensure the git status check is actually running in the worktree directory and checking for any changes.