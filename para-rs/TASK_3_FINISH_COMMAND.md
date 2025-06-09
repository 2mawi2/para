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