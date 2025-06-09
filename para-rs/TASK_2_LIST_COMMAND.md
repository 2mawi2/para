# Task 2: Implement List Command

## Objective
Implement the `para list` command that displays active and archived sessions with detailed information.

## Key Requirements

### Command Behavior
- `para list` - Show active sessions
- `para list --verbose` - Show detailed session information
- `para list --archived` - Show archived sessions
- `para ls` - Alias for list command

### Core Functionality
1. **Session Discovery**: Find all active sessions from `.para_state/` directory
2. **Worktree Status**: Check if worktrees still exist and are clean
3. **Branch Information**: Show current branch, commit count, last modified
4. **Archive Support**: List archived sessions when requested
5. **Output Formatting**: Clean, readable table format with status indicators

### Display Information
- Session name/ID
- Branch name
- Worktree path
- Status (active, dirty, missing)
- Last modified timestamp
- Commit count ahead of base branch
- Current working directory (verbose mode)

### Implementation Files to Modify
- `src/cli/commands/list.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `GitService::discover()` for repository operations
- Use `WorktreeManager::list_worktrees()` for worktree information
- Use `BranchManager` for branch status
- Read session state files from `.para_state/` directory
- Use existing error handling patterns

### Success Criteria
- Command displays accurate session information
- Shows correct status for each session (clean, dirty, missing)
- Handles missing worktrees gracefully
- Provides clear output formatting
- Supports both verbose and archived modes
- Works from any directory within git repository

### Error Handling
- Handle case when not in git repository
- Handle case when state directory doesn't exist
- Handle corrupted state files gracefully
- Handle missing or moved worktrees
- Provide helpful messages when no sessions exist

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for list command logic
- **Integration Tests**: Test end-to-end list command functionality
- **Legacy Tests**: Ensure relevant legacy bats tests pass with new implementation
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the list command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement list command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.