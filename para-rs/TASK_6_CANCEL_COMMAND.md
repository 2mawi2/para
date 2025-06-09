# Task 6: Implement Cancel Command

## Objective
Implement the `para cancel` command that archives individual sessions without integration, providing safe session cleanup for abandoned work.

## Key Requirements

### Command Behavior
- `para cancel` - Cancel current session (auto-detect from working directory)
- `para cancel session-id` - Cancel specific session by ID
- Moves session to archive without integrating changes
- Preserves work for potential future recovery

### Core Functionality
1. **Session Detection**: Auto-detect current session or use provided session ID
2. **Archive Operation**: Move branch to archive namespace (para/archived/timestamp/session-name)
3. **State Cleanup**: Remove session state files
4. **Worktree Cleanup**: Remove worktree directory safely
5. **User Notification**: Inform user about recovery options
6. **Safety Checks**: Confirm operation for sessions with uncommitted changes

### Implementation Files to Modify
- `src/cli/commands/cancel.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `GitService::discover()` for repository operations
- Use `BranchManager::move_to_archive()` for archiving branches
- Use `WorktreeManager::remove_worktree()` for worktree cleanup
- Use session state management from `.para_state/` directory
- Use existing error handling patterns

### Workflow Steps
1. **Detect Session**: Auto-detect from current directory or use provided session ID
2. **Validate Session**: Ensure session exists and is active
3. **Safety Checks**: Warn about uncommitted changes if present
4. **User Confirmation**: Ask for confirmation if there are uncommitted changes
5. **Archive Branch**: Move branch to archive namespace with timestamp
6. **Clean Worktree**: Remove worktree directory
7. **Clean State**: Remove session state files
8. **User Guidance**: Inform user about recovery command

### Success Criteria
- Command properly detects current session when no ID provided
- Handles explicit session ID parameter
- Archives session branch with proper naming (para/archived/timestamp/session-name)
- Safely removes worktree and state files
- Warns user about uncommitted changes before proceeding
- Provides recovery guidance after successful cancellation
- Handles edge cases gracefully (missing session, corrupted state, etc.)

### Error Handling
- Handle case when not in a para session and no session ID provided
- Handle case when provided session ID doesn't exist
- Handle case when session has uncommitted changes
- Handle case when archive operation fails
- Handle case when worktree removal fails
- Provide helpful error messages with next steps
- Handle incomplete cancel operations gracefully

### Safety Considerations
- Always warn about uncommitted changes before canceling
- Require explicit confirmation for sessions with uncommitted work
- Preserve session branch in archive for potential recovery
- Provide clear guidance on how to recover canceled sessions
- Handle case when user is currently in the session directory

### Relationship to Clean Command
- Cancel handles individual session cleanup (what clean was initially supposed to do)
- Clean focuses on bulk operations (all sessions at once)
- Cancel preserves work in archive, clean can optionally remove archives
- This separation provides both targeted and bulk cleanup workflows

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for cancel command logic
- **Integration Tests**: Test end-to-end cancel command functionality with real git operations
- **Legacy Tests**: Ensure relevant legacy bats tests pass with new implementation
- **Safety Tests**: Test warning and confirmation prompts
- **Edge Case Tests**: Test with missing sessions, corrupted state, uncommitted changes
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the cancel command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement cancel command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.