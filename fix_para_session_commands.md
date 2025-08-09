# Fix Para Session Commands for Worktree Sessions

## Problem 1: Uncommitted Changes Detection
The `para list --json` command incorrectly reports `has_uncommitted_changes: false` for sessions that actually have uncommitted changes. This causes issues in para-ui where:
1. The UI doesn't show warnings about uncommitted changes
2. Cancel operations fail with "Cannot cancel session with uncommitted changes in non-interactive mode"

### Current Behavior
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

### Root Cause
The `has_uncommitted_changes` field in the session list is not properly checking the git status of the worktree. It needs to:
1. Check both staged and unstaged changes
2. Check for untracked files
3. Properly handle worktree paths

### Required Fix for Problem 1
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

## Problem 2: Para Finish Fails with Worktree Checkout Error
The `para finish` command fails when trying to finish a session that uses a worktree:

### Current Behavior
```bash
para finish "commit message" test-session-2
# Error: fatal: 'para/test-session-2' is already used by worktree at '/path/to/.para/worktrees/test-session-2'
# para: Git operation failed: Git command failed: checkout para/test-session-2
```

### Root Cause
Para finish is trying to `git checkout` the branch in the main repository, but the branch is already checked out in a worktree. Git doesn't allow a branch to be checked out in multiple places.

### Required Fix for Problem 2
Para finish should:
1. **Option A**: Work within the worktree itself (recommended)
   - Change to the worktree directory
   - Create the commit there
   - Push from the worktree
   - No need to checkout in main repo

2. **Option B**: Remove worktree before checkout
   - Remove the worktree first
   - Then checkout in main repo
   - Complete the finish operation
   - (This is more disruptive)

3. **Option C**: Use detached operations
   - Don't checkout the branch at all
   - Use git commands that don't require checkout
   - Create commit using `git commit-tree` or similar

Recommended implementation (Option A):
```rust
fn finish_session(session_id: &str, message: &str) -> Result<()> {
    let session = get_session(session_id)?;
    
    if session.session_type == SessionType::Worktree {
        // Work in the worktree directly
        let worktree_path = &session.worktree_path;
        
        // Stage all changes in the worktree
        run_git_command(&["add", "."], worktree_path)?;
        
        // Create commit in the worktree
        run_git_command(&["commit", "-m", message], worktree_path)?;
        
        // The branch is already checked out in worktree, no need to checkout
        // Continue with merge/PR creation as needed
    } else {
        // Original flow for non-worktree sessions
        checkout_branch(&session.branch)?;
        // ... rest of finish logic
    }
}
```

## Test Cases

### For Problem 1 (Uncommitted Changes)
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

### For Problem 2 (Finish Command)
1. **Finish with clean worktree**
   ```bash
   para start finish-test
   echo "content" > file.txt
   git add file.txt
   git commit -m "initial"
   para finish "Final commit" finish-test
   # Should succeed without checkout errors
   ```

2. **Finish with uncommitted changes**
   ```bash
   para start finish-uncommitted
   echo "content" > file.txt
   para finish "Commit all changes" finish-uncommitted
   # Should stage, commit, and complete successfully
   ```

## Files Likely to Need Changes
1. **For uncommitted changes detection**:
   - `src/cli/commands/list/mod.rs`
   - `src/core/session.rs`
   - `src/core/git.rs`

2. **For finish command**:
   - `src/cli/commands/finish.rs`
   - Look for where it does `git checkout`
   - Modify to handle worktree sessions differently

## Success Criteria
1. `para list --json` correctly reports `has_uncommitted_changes` status
2. `para finish` works without checkout errors for worktree sessions
3. Both commands handle worktree sessions properly
4. All test cases pass
5. No regression in non-worktree session handling

## Additional Context
These issues are blocking proper integration with para-ui:
- Users can't see if sessions have uncommitted changes
- Users can't finish sessions from the UI
- Cancel button fails unexpectedly
The fixes should make para commands work consistently whether called from CLI or via para-ui.