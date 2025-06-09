# Task 5: Implement Clean Command

## Objective
Implement the `para clean` command that removes all active sessions and optionally cleans up archived sessions.

## Key Requirements

### Command Behavior
- `para clean` - Remove all active sessions (worktrees and state)
- `para clean --backups` - Also remove archived sessions
- Interactive confirmation before destructive operations

### Core Functionality
1. **Session Discovery**: Find all active sessions from `.para_state/` directory
2. **Worktree Cleanup**: Remove all para worktrees safely
3. **State Cleanup**: Remove session state files
4. **Archive Cleanup**: Optionally remove archived sessions when --backups flag used
5. **Branch Cleanup**: Remove para branches that no longer have worktrees
6. **Safety Checks**: Confirm destructive operations with user
7. **Graceful Handling**: Handle missing or corrupted sessions

### Implementation Files to Modify
- `src/cli/commands/clean.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `GitService::discover()` for repository operations
- Use `WorktreeManager::cleanup_stale_worktrees()` for worktree removal
- Use `BranchManager` for branch cleanup operations
- Read and clean `.para_state/` directory
- Use existing error handling patterns

### Cleanup Operations
1. **Validate Repository**: Ensure we're in a git repository
2. **Discover Sessions**: Find all sessions from state directory
3. **User Confirmation**: Ask for confirmation before destructive operations
4. **Remove Worktrees**: Safely remove all para worktrees
5. **Clean Branches**: Remove para branches without worktrees
6. **Clean State**: Remove session state files
7. **Archive Cleanup**: If --backups, also clean archived sessions
8. **Report Results**: Show what was cleaned up

### Success Criteria
- Command safely removes all active sessions
- Properly handles missing or moved worktrees
- Cleans up orphaned branches
- Provides clear feedback about what was cleaned
- Supports optional archive cleanup with --backups
- Asks for confirmation before destructive operations
- Handles edge cases gracefully (no sessions, corrupted state, etc.)

### Error Handling
- Handle case when not in git repository
- Handle case when no sessions exist
- Handle case when worktree removal fails
- Handle case when branch removal fails
- Handle case when state files are corrupted
- Provide helpful error messages
- Continue cleanup even if some operations fail

### Safety Considerations
- Always ask for user confirmation before cleaning
- Provide summary of what will be cleaned before confirmation
- Handle case when user is currently in a session directory
- Ensure no data loss from accidental cleanup
- Provide option to clean individual sessions vs all

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for clean command logic
- **Integration Tests**: Test end-to-end clean command functionality with real git operations
- **Legacy Tests**: Ensure relevant legacy bats tests pass with new implementation
- **Safety Tests**: Test confirmation prompts and user interaction
- **Edge Case Tests**: Test with missing worktrees, corrupted state, etc.
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the clean command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement clean command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.