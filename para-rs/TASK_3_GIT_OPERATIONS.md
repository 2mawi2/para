# Task 3: Git Operations Module

## Overview
Implement all Git operations using `std::process::Command` to maintain 100% compatibility with user's existing Git setup, as specified in the PRD.

## Scope
Build the `src/core/git/` module with comprehensive Git operations:

```
src/core/git/
├── mod.rs           // Main git module interface
├── repository.rs    // Repository discovery and validation
├── worktree.rs      // Worktree creation/removal operations
├── branch.rs        // Branch operations and validation
└── integration.rs   // Rebase and merge logic for integration
```

## Deliverables

### 1. Repository Operations (`repository.rs`)
```rust
pub struct GitRepository {
    pub root: PathBuf,
    pub git_dir: PathBuf,
}

impl GitRepository {
    pub fn discover() -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
    pub fn get_current_branch(&self) -> Result<String>;
    pub fn has_uncommitted_changes(&self) -> Result<bool>;
    pub fn get_commit_count_since(&self, base_branch: &str, feature_branch: &str) -> Result<usize>;
    pub fn is_clean_working_tree(&self) -> Result<bool>;
    pub fn get_remote_url(&self) -> Result<Option<String>>;
}

// Git command execution utilities
fn execute_git_command(repo: &GitRepository, args: &[&str]) -> Result<String>;
fn execute_git_command_with_status(repo: &GitRepository, args: &[&str]) -> Result<()>;
```

### 2. Worktree Management (`worktree.rs`)
```rust
pub struct WorktreeManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> WorktreeManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self;
    
    // Core worktree operations
    pub fn create_worktree(&self, branch_name: &str, path: &Path) -> Result<()>;
    pub fn remove_worktree(&self, path: &Path) -> Result<()>;
    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>>;
    pub fn is_worktree_clean(&self, path: &Path) -> Result<bool>;
    
    // Worktree validation and cleanup
    pub fn validate_worktree(&self, path: &Path) -> Result<()>;
    pub fn force_remove_worktree(&self, path: &Path) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
    pub is_bare: bool,
}
```

### 3. Branch Operations (`branch.rs`)
```rust
pub struct BranchManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> BranchManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self;
    
    // Branch lifecycle
    pub fn create_branch(&self, name: &str, base: &str) -> Result<()>;
    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()>;
    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()>;
    pub fn branch_exists(&self, name: &str) -> Result<bool>;
    
    // Branch information
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>>;
    pub fn get_merge_base(&self, branch1: &str, branch2: &str) -> Result<String>;
    pub fn is_branch_merged(&self, branch: &str, into: &str) -> Result<bool>;
    
    // Archive operations
    pub fn move_to_archive(&self, branch: &str, prefix: &str) -> Result<String>;
    pub fn restore_from_archive(&self, archived_branch: &str, prefix: &str) -> Result<String>;
    pub fn list_archived_branches(&self, prefix: &str) -> Result<Vec<String>>;
    pub fn clean_archived_branches(&self, prefix: &str) -> Result<usize>;
    
    // Branch validation
    pub fn validate_branch_name(&self, name: &str) -> Result<()>;
    pub fn generate_unique_branch_name(&self, base_name: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub commit: String,
    pub is_current: bool,
    pub upstream: Option<String>,
}
```

### 4. Integration Operations (`integration.rs`)
```rust
pub struct IntegrationManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> IntegrationManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self;
    
    // Session finishing operations
    pub fn finish_session(&self, request: FinishRequest) -> Result<FinishResult>;
    pub fn squash_commits(&self, feature_branch: &str, base_branch: &str, message: &str) -> Result<()>;
    
    // Integration operations
    pub fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult>;
    pub fn prepare_rebase(&self, feature_branch: &str, base_branch: &str) -> Result<()>;
    pub fn continue_rebase(&self) -> Result<()>;
    pub fn abort_rebase(&self) -> Result<()>;
    
    // Conflict handling
    pub fn has_rebase_conflicts(&self) -> Result<bool>;
    pub fn get_conflicted_files(&self) -> Result<Vec<PathBuf>>;
    pub fn is_rebase_in_progress(&self) -> Result<bool>;
    
    // Remote operations
    pub fn update_base_branch(&self, branch: &str) -> Result<()>;
    pub fn pull_latest_changes(&self, branch: &str) -> Result<()>;
}

#[derive(Debug)]
pub struct FinishRequest {
    pub feature_branch: String,
    pub base_branch: String,
    pub commit_message: String,
    pub target_branch_name: Option<String>,
    pub integrate: bool,
}

#[derive(Debug)]
pub enum FinishResult {
    Success { final_branch: String },
    ConflictsPending { state_saved: bool },
}

#[derive(Debug)]
pub struct IntegrationRequest {
    pub feature_branch: String,
    pub base_branch: String,
    pub commit_message: String,
}

#[derive(Debug)]
pub enum IntegrationResult {
    Success,
    ConflictsPending { conflicted_files: Vec<PathBuf> },
    Failed { error: String },
}
```

### 5. Main Module Interface (`mod.rs`)
```rust
// Re-exports for easy access
pub use repository::{GitRepository};
pub use worktree::{WorktreeManager, WorktreeInfo};
pub use branch::{BranchManager, BranchInfo};
pub use integration::{IntegrationManager, FinishRequest, FinishResult, IntegrationRequest, IntegrationResult};

// Trait for Git operations (enables testing with mocks)
pub trait GitOperations {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
    fn remove_worktree(&self, path: &Path) -> Result<()>;
    fn finish_session(&self, request: FinishRequest) -> Result<FinishResult>;
    fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult>;
}

// Implementation for real Git operations
impl GitOperations for GitRepository {
    // ... delegate to manager structs
}
```

## Dependencies
```toml
# Add to Cargo.toml (minimal - using std::process::Command)
regex = "1.0"
tempfile = "3.0" # for testing
```

## Key Implementation Principles

### 1. Use std::process::Command Only
- No `git2-rs` or `libgit2` dependencies
- All Git operations via command-line interface
- Ensures 100% compatibility with user's Git setup

### 2. Robust Error Handling
```rust
fn execute_git_command(repo: &GitRepository, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(&repo.root)
        .args(args)
        .output()
        .map_err(|e| ParaError::git_operation(format!("Failed to execute git: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParaError::git_operation(format!("Git command failed: {}", stderr)));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}
```

### 3. Command Safety
- Validate all branch names before using in commands
- Escape special characters in commit messages
- Sanitize paths before passing to Git
- Use absolute paths to avoid confusion

## Testing Approach
- Unit tests with temporary Git repositories
- Integration tests with real Git operations
- Mock implementations of GitOperations trait for other modules
- Test error conditions (invalid repos, network failures, conflicts)
- Cross-platform testing (Windows, macOS, Linux)

## Acceptance Criteria
✅ Can discover Git repositories correctly (main repo and worktrees)  
✅ Worktree operations work reliably (create, remove, validate)  
✅ Branch operations handle all edge cases (naming, conflicts, archives)  
✅ Integration operations handle rebase conflicts gracefully  
✅ All Git commands are executed safely with proper error handling  
✅ Archive/recovery system works for cancelled sessions  
✅ Supports both simple finish and full integration workflows  
✅ Cross-platform compatibility verified  
✅ Performance is acceptable for typical repository sizes  

## Integration Points
- **Error types**: Uses error types from utils module
- **Config**: Gets branch prefix and other Git settings from config
- **Session module**: Called by session operations for Git functionality
- **CLI**: Error messages are user-friendly and actionable

## Testing Strategy
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn setup_test_repo() -> (TempDir, GitRepository) {
        // Create temporary Git repository for testing
    }
    
    #[test]
    fn test_worktree_creation() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = WorktreeManager::new(&repo);
        // Test worktree operations
    }
}
```

## Notes
- This module is critical for behavioral parity with shell version
- Focus on robustness and error handling
- All Git interactions must be well-tested
- Keep the public API simple but comprehensive
- Design for easy testing with mock implementations