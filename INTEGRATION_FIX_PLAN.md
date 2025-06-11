# Para Integration Fix Plan: Worktree Branch Checkout Issue

## Problem Analysis

### Current Issue
Para's integration logic fails with the error: `fatal: 'main' is already used by worktree at /path/to/main/repo`

**Root Cause**: Para tries to `git checkout main` from within session worktrees, but main is already checked out in the main repository. Git prevents multiple worktrees from having the same branch checked out simultaneously.

**Problematic Code Locations**:
- `src/core/git/strategy.rs:81` - `self.repo.checkout_branch(&request.target_branch)?;`
- `src/core/git/strategy.rs:107, 121, 163` - All strategies try to checkout main during integration  
- `src/cli/commands/finish.rs:109` - `git_service.repository().checkout_branch(&base_branch)?;`

### Current Broken Workflow
```bash
# From session worktree: /subtrees/para/session1 (on branch para/session1-timestamp)
para integrate "message"
  ↓
git checkout main        # ❌ FAILS - main locked by main repo
git merge session-branch # Never reached
```

### Desired Workflow (What User Actually Wants)
The user wants **change extraction**, not **branch merging**:

1. **Extract changes** from the worktree (all modifications since branching from main)
2. **Apply changes** to main (or new branch off main) as if work was done directly on main
3. **Archive/remove** the worktree after successful integration
4. **Result**: Clean integration without worktree branch merge complexity

## Deep Analysis: Will This Work?

### Option 1: Two-Phase Integration (Initial Plan)
**Problems identified**:
1. **Git patch limitations**: `git diff` + `git apply` fails with conflicts or path changes
2. **Coordination complexity**: Split operations between worktree and main repo contexts
3. **Execution context**: Para runs in worktree, can't easily switch to main repo due to security restrictions

### Option 2: Remote Git Operations (Revised Plan)
**Better approach**: Use git's remote operation capabilities

Instead of trying to checkout main in worktrees, operate on main repo remotely:
```bash
# From worktree, create proper git patches  
git format-patch main..HEAD --stdout > /tmp/session-changes.patch

# Apply to main repo using remote git operations
git --git-dir=/main/repo/.git --work-tree=/main/repo am /tmp/session-changes.patch
```

**Advantages**:
- No branch checkout conflicts
- Preserves commit messages and metadata
- Better conflict resolution via `git am`
- Single atomic operation
- Works from any directory

## Implementation Plan

### Phase 1: Fix Immediate Checkout Issue (Quick Win)

**File**: `src/core/git/strategy.rs`
**Goal**: Add safety check to prevent checkout attempts in worktrees

```rust
impl<'a> StrategyManager<'a> {
    fn execute_merge(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        // ✅ ADD: Prevent checkout in worktrees
        if self.is_in_worktree()? {
            return self.execute_worktree_integration(request);
        }
        
        // Original logic (only for main repo)
        self.integration.update_base_branch(&request.target_branch)?;
        self.repo.checkout_branch(&request.target_branch)?; // Now safe
        // ... rest of logic
    }
}
```

### Phase 2: Implement Change Extraction

**File**: `src/core/git/integration.rs`
**Goal**: Add methods for remote git operations

```rust
impl<'a> IntegrationManager<'a> {
    /// Check if current repository is a worktree (not main repo)
    pub fn is_in_worktree(&self) -> Result<bool> {
        let output = execute_git_command(self.repo, &["rev-parse", "--is-inside-work-tree"])?;
        if output.trim() != "true" {
            return Ok(false);
        }
        
        // Check if we're in a worktree by looking for .git file (not directory)
        let git_path = self.repo.root.join(".git");
        Ok(git_path.is_file()) // Worktrees have .git file, main repo has .git directory
    }
    
    /// Get path to main repository from worktree
    pub fn get_main_repo_path(&self) -> Result<PathBuf> {
        if !self.is_in_worktree()? {
            return Ok(self.repo.root.clone());
        }
        
        // Read .git file to find main repo path
        let git_file = self.repo.root.join(".git");
        let git_content = std::fs::read_to_string(git_file)?;
        
        // Format: "gitdir: /path/to/main/repo/.git/worktrees/session-name"
        let git_dir = git_content.strip_prefix("gitdir: ")
            .ok_or_else(|| ParaError::git_operation("Invalid .git file format".to_string()))?
            .trim();
            
        // Extract main repo path: /path/to/main/repo/.git/worktrees/session -> /path/to/main/repo
        let main_git_dir = PathBuf::from(git_dir)
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| ParaError::git_operation("Cannot determine main repo path".to_string()))?;
            
        Ok(main_git_dir.to_path_buf())
    }
    
    /// Extract changes from worktree and apply to main repo
    pub fn integrate_from_worktree(
        &self, 
        feature_branch: &str, 
        target_branch: &str,
        commit_message: &str
    ) -> Result<()> {
        let main_repo_path = self.get_main_repo_path()?;
        let main_git_dir = main_repo_path.join(".git");
        
        // Create patches for all commits since branching from target_branch
        let patch_output = execute_git_command(
            self.repo, 
            &["format-patch", &format!("{}..HEAD", target_branch), "--stdout"]
        )?;
        
        if patch_output.trim().is_empty() {
            // No commits to apply
            return Ok(());
        }
        
        // Write patch to temporary file
        let temp_patch = format!("/tmp/para-integration-{}.patch", generate_timestamp());
        std::fs::write(&temp_patch, patch_output)?;
        
        // Apply to main repo using remote git operations
        let git_dir_arg = format!("--git-dir={}", main_git_dir.display());
        let work_tree_arg = format!("--work-tree={}", main_repo_path.display());
        
        // First, checkout target branch in main repo
        execute_git_command_with_status(
            self.repo,
            &["--git-dir", &main_git_dir.to_string_lossy(), "checkout", target_branch]
        )?;
        
        // Apply patches
        execute_git_command_with_status(
            self.repo,
            &[&git_dir_arg, &work_tree_arg, "am", &temp_patch]
        )?;
        
        // Cleanup
        let _ = std::fs::remove_file(temp_patch);
        
        Ok(())
    }
}
```

### Phase 3: Update Strategy Logic

**File**: `src/core/git/strategy.rs`
**Goal**: Route worktree integrations to change extraction

```rust
impl<'a> StrategyManager<'a> {
    pub fn execute_strategy(&self, request: StrategyRequest) -> Result<StrategyResult> {
        self.integration
            .validate_integration_preconditions(&request.feature_branch, &request.target_branch)?;

        if request.dry_run {
            return self.preview_strategy(&request);
        }

        // ✅ NEW: Handle worktree integration differently
        if self.integration.is_in_worktree()? {
            return self.execute_worktree_strategy(&request);
        }

        // Original main repo integration logic
        let backup_name = self.integration.create_backup_branch(
            &request.target_branch,
            &format!("pre-integration-{}", chrono::Utc::now().timestamp()),
        )?;

        match self.execute_integration_strategy(&request) {
            Ok(result) => Ok(result),
            Err(e) => {
                self.integration
                    .restore_from_backup(&backup_name, &request.target_branch)?;
                Err(e)
            }
        }
    }
    
    /// Handle integration from worktree using change extraction
    fn execute_worktree_strategy(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        match self.integration.integrate_from_worktree(
            &request.feature_branch,
            &request.target_branch,
            "Integration from worktree" // TODO: Get proper commit message
        ) {
            Ok(()) => Ok(StrategyResult::Success {
                final_branch: request.target_branch.clone(),
            }),
            Err(e) => {
                // Check if it's a conflict error from git am
                if e.to_string().contains("patch does not apply") {
                    Ok(StrategyResult::ConflictsPending {
                        conflicted_files: vec![], // TODO: Get actual conflicted files
                    })
                } else {
                    Ok(StrategyResult::Failed {
                        error: e.to_string(),
                    })
                }
            }
        }
    }
}
```

### Phase 4: Remove Problematic Checkout

**File**: `src/cli/commands/finish.rs`
**Goal**: Remove the checkout line that causes the error

```rust
// ❌ REMOVE THIS LINE (around line 109):
// git_service.repository().checkout_branch(&base_branch)?;

// The integration logic will handle branch management properly
```

## Testing Strategy

### Test Cases
1. **Integration from worktree** - should extract changes and apply to main
2. **Integration from main repo** - should work as before (backward compatibility)
3. **Conflict handling** - should detect and report conflicts properly
4. **No-changes case** - should handle gracefully when worktree has no commits
5. **Multiple commits** - should preserve all commits during integration

### Manual Testing Flow
```bash
# Setup test scenario
para start test-session
echo "test change" > test.txt
git add test.txt
git commit -m "test change"

# Test integration (should work without checkout errors)
para integrate "test integration"

# Verify result in main repo
cd /main/repo
git log --oneline  # Should show integrated commit
```

## Potential Issues & Mitigations

### Issue 1: Conflict Resolution
**Problem**: Conflicts during `git am` are harder to resolve than merge conflicts
**Mitigation**: 
- Provide clear error messages directing user to main repo
- Add `para continue` support for `git am --continue` workflow

### Issue 2: Commit Message Handling  
**Problem**: `git format-patch` preserves original commit messages
**Mitigation**: 
- For single commits: use original message
- For multiple commits: provide squash option
- Allow message override via command args

### Issue 3: Branch Creation
**Problem**: User wants result on new branch, not directly on main
**Mitigation**:
- Create integration branch before applying patches
- Use `--target` flag to specify destination branch

## Success Criteria

✅ **No more "already used by worktree" errors**
✅ **Clean integration from worktrees to main**  
✅ **Preserved commit history and messages**
✅ **Proper conflict detection and handling**
✅ **Backward compatibility with main repo operations**

## Implementation Priority

1. **Phase 1** (Immediate): Add worktree detection and safety checks
2. **Phase 2** (Core): Implement `integrate_from_worktree()` method
3. **Phase 3** (Integration): Update strategy routing logic  
4. **Phase 4** (Cleanup): Remove problematic checkout calls
5. **Testing**: Comprehensive test suite for all scenarios

This approach solves the immediate problem while implementing the user's desired "change extraction" workflow in a robust, git-native way.