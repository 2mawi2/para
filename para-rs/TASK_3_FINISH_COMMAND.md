# Task 3: Implement Finish Command

## Objective
Implement the `para finish` command that commits changes, squashes commits, and optionally integrates back to the base branch.

## Key Requirements

### Command Behavior
- `para finish "commit message"` - Finish current session with commit message
- `para finish "message" --branch new-name` - Rename branch after finishing
- `para finish "message" --integrate` - Auto-integrate into base branch
- `para finish "message" session-id` - Finish specific session

### Core Functionality
1. **Auto-Detection**: Detect current session from working directory
2. **Change Staging**: Auto-stage all changes in the worktree
3. **Commit Squashing**: Squash all commits into single commit with provided message
4. **Branch Management**: Optionally rename feature branch
5. **Integration**: Optionally merge back to base branch
6. **Cleanup**: Update session state and handle worktree cleanup
7. **Context Switch**: Return to base branch/directory

### Implementation Files to Modify
- `src/cli/commands/finish.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `GitService` for all git operations
- Use `IntegrationManager::finish_session()` for squashing logic
- Use `ConfigManager` for auto-stage/commit settings
- Update session state files appropriately
- Use `WorktreeManager` for worktree operations

### Workflow Steps
1. Validate we're in a para session
2. Check for uncommitted changes
3. Auto-stage changes if configured
4. Squash all commits into single commit
5. Optionally rename branch
6. Optionally integrate into base branch
7. Update session state (preserve or archive)
8. Switch back to base branch
9. Optionally remove worktree

### Success Criteria
- Command properly squashes commits with provided message
- Handles both current directory detection and explicit session ID
- Supports optional branch renaming
- Supports optional auto-integration
- Preserves or archives session based on config
- Returns user to clean state after finish
- Maintains git history integrity

### Error Handling
- Handle case when not in para session
- Handle case when session has no changes
- Handle case when integration conflicts occur
- Handle case when branch rename conflicts
- Provide helpful error messages with next steps
- Handle incomplete finish operations gracefully

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for finish command logic
- **Integration Tests**: Test end-to-end finish command functionality with real git operations
- **Legacy Tests**: Ensure relevant legacy bats tests pass with new implementation
- **Conflict Handling**: Test merge conflict scenarios
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the finish command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement finish command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.

## PREVIOUS IMPLEMENTATION REVIEW (Commit a780e848623d86208012242f31e6047da1f5080f)

### Assessment: **PARTIALLY COMPLIANT** - Needs Core Implementation

### ✅ **Correctly Implemented:**
1. **Command Behavior**: Supports all required flags (`--branch`, `--integrate`, session ID parameter)
2. **Auto-Detection**: Detects current session from working directory via `validate_session_environment()`
3. **Change Staging**: Auto-stages changes if `config.should_auto_stage()` is true
4. **Session State Management**: Properly handles session state updates and cleanup
5. **Context Switch**: Returns to base branch after finishing
6. **Error Handling**: Good error handling for invalid environments and missing sessions
7. **Testing**: Comprehensive unit tests covering validation and integration scenarios

### ❌ **Critical Issues Found:**

1. **COMPILATION ERRORS** - These must be fixed first:
   ```rust
   // These methods don't exist in ParaError:
   ParaError::git_error(...)    // Should be: ParaError::git_operation(...)
   ParaError::fs_error(...)     // Should be: ParaError::file_operation(...)
   
   // This method doesn't exist in GitService:
   git_service.finish_session(finish_request)?  // Must be implemented
   ```

2. **Missing Core Types** - These need to be defined:
   ```rust
   // In src/core/git/mod.rs or integration.rs:
   pub struct FinishRequest {
       pub feature_branch: String,
       pub base_branch: String,
       pub commit_message: String,
       pub target_branch_name: Option<String>,
       pub integrate: bool,
   }
   
   pub enum FinishResult {
       Success { final_branch: String },
       ConflictsPending { state_saved: bool },
   }
   ```

3. **Missing Core Implementation** - The most important part is missing:
   - **No `GitService::finish_session()` method** - This is the core functionality
   - **No commit squashing logic** - Need interactive rebase or reset + commit
   - **No branch renaming implementation** - Missing git branch rename logic
   - **No integration/merge logic** - Missing merge into base branch
   - **No worktree cleanup** - Missing worktree removal after finish

### **Required Next Steps:**

1. **Fix Compilation**: Update all error method calls to match existing ParaError API
2. **Define Types**: Create `FinishRequest` and `FinishResult` in appropriate modules
3. **Implement `GitService::finish_session()`**: This is the core method that needs:
   - Commit squashing (interactive rebase -i or reset + new commit)
   - Branch renaming support (`git branch -m`)
   - Integration logic (merge into base branch)
   - Conflict detection and handling
   - Return appropriate `FinishResult`
4. **Add Integration Logic**: Connect with `IntegrationManager` for merge operations
5. **Complete Worktree Cleanup**: Remove worktree after successful finish
6. **Test All Paths**: Ensure tests cover squashing, renaming, integration, and conflicts

### **Architecture Notes:**
- The high-level structure is good and follows the right patterns
- Repository enhancement in `repository.rs` adds useful utilities
- The session detection and validation logic is well implemented
- Just needs the core Git operations to be fully functional

**CRITICAL**: The implementation cannot compile or run until the error method calls are fixed and the `finish_session()` method is implemented.