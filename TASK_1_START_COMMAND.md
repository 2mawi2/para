# Task 1: Implement Start Command

## Objective
Implement the `para start` command that creates new development sessions with Git worktrees and launches the configured IDE.

## Key Requirements

### Command Behavior
- `para start` - Create session with auto-generated friendly name
- `para start my-feature` - Create session with specific name
- `para start --dangerously-skip-permissions` - Skip IDE permission checks

### Core Functionality
1. **Session Creation**: Generate unique session name using friendly name format
2. **Git Worktree**: Create new worktree in `subtrees/pc/SESSION_NAME` directory
3. **Branch Management**: Create new branch with prefix (e.g., `pc/20250609-123456`)
4. **State Tracking**: Save session state in `.para_state/SESSION_NAME.state`
5. **IDE Launch**: Open configured IDE in the new worktree directory

### Implementation Files to Modify
- `src/cli/commands/start.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `ConfigManager::load_or_create()` for IDE settings
- Use `GitService::discover()` for git operations
- Use `generate_friendly_name()` from utils for session names
- Use `WorktreeManager` for worktree creation
- Use `BranchManager` for branch creation

### Success Criteria
- Command creates working git worktree
- Session state is properly tracked
- IDE launches successfully (or provides clear error)
- Compatible with existing config system
- Passes argument parsing validation
- Works from any directory within git repository

### Error Handling
- Handle case when not in git repository
- Handle case when session name already exists
- Handle case when IDE not found or permission denied
- Handle case when git worktree creation fails
- Provide helpful error messages with suggested fixes

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for start command logic
- **Integration Tests**: Test end-to-end start command functionality
- **Legacy Tests**: Ensure relevant legacy bats tests pass with new implementation
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the start command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement start command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.