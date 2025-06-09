# Task 8: Advanced Git Operations (Integrate Command)

## Objective
Enhance the `para integrate` command with comprehensive merge conflict handling, branch management, and integration workflows.

## Key Requirements

### Command Behavior
- `para integrate` - Integrate current session into base branch
- `para integrate session-name` - Integrate specific session
- `para integrate --strategy merge|squash|rebase` - Choose integration strategy
- `para integrate --target branch-name` - Integrate into specific target branch
- `para integrate --dry-run` - Preview integration without executing

### Core Functionality
1. **Smart Integration**: Automatically choose best integration strategy
2. **Conflict Resolution**: Detect and guide users through merge conflicts
3. **Branch Management**: Handle complex branching scenarios
4. **Integration Strategies**: Support merge, squash, and rebase workflows
5. **Rollback Support**: Ability to undo failed integrations
6. **Integration History**: Track integration attempts and outcomes

### Implementation Files to Modify
- `src/cli/commands/integrate.rs` - Main command implementation
- `src/core/git/integration.rs` - Enhanced integration logic
- `src/core/git/conflict.rs` - Conflict detection and resolution
- `src/core/git/strategy.rs` - Integration strategy management

### Expected Integration Points
- Use `GitService` for all git operations
- Use `SessionManager` for session state management
- Use existing `BranchManager` and `WorktreeManager`
- Integrate with configuration system for default strategies
- Use file system utilities for conflict marker detection

### Integration Strategies

#### Merge Strategy
- Create merge commit preserving feature branch history
- Handle merge conflicts interactively
- Maintain full commit history from feature branch

#### Squash Strategy
- Combine all feature branch commits into single commit
- Generate meaningful commit message from branch history
- Clean linear history on target branch

#### Rebase Strategy
- Replay feature branch commits on target branch
- Handle rebase conflicts step by step
- Maintain individual commits with updated base

### Conflict Resolution Workflow
1. **Conflict Detection**: Automatically detect merge conflicts
2. **Conflict Presentation**: Show conflicted files with clear formatting
3. **Resolution Guidance**: Provide clear instructions for manual resolution
4. **Resolution Validation**: Verify conflicts are properly resolved
5. **Continue Integration**: Resume integration after conflict resolution
6. **Abort Option**: Allow users to abort integration and restore state

### Advanced Git Operations
- **Fast-Forward Detection**: Optimize for fast-forward merges when possible
- **Branch Cleanup**: Optionally delete feature branch after integration
- **Remote Synchronization**: Handle remote branch updates during integration
- **Stash Management**: Automatically stash/unstash uncommitted changes
- **Commit Message Generation**: Smart commit message creation for squash merges

### State Management
- **Integration State**: Track ongoing integration progress
- **Rollback Information**: Store enough information to undo integration
- **Session Updates**: Update session state during integration
- **Backup Creation**: Create safety backups before risky operations

### Success Criteria
- Successfully integrates sessions using all three strategies
- Handles merge conflicts gracefully with clear user guidance
- Provides rollback capability for failed integrations
- Maintains git repository integrity throughout process
- Generates appropriate commit messages for each strategy
- Works with both local and remote repositories
- Preserves session state and metadata correctly

### Error Handling
- Handle case when target branch doesn't exist
- Handle case when session has uncommitted changes
- Handle case when integration conflicts cannot be resolved
- Handle case when git repository is in inconsistent state
- Handle case when remote repository access fails
- Provide helpful error messages with suggested fixes
- Allow recovery from partial integration states

### Conflict Resolution UI
- Clear presentation of conflicted files
- Show conflict markers and explanations
- Provide commands to open files in configured editor
- Guide users through resolution process step by step
- Validate that all conflicts are resolved before continuing

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for integration logic
- **Integration Tests**: Test end-to-end integration workflows
- **Conflict Tests**: Test various conflict scenarios and resolutions
- **Strategy Tests**: Test all integration strategies thoroughly
- **Error Handling Tests**: Test all error conditions and recovery
- **Repository State Tests**: Verify git repository integrity
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors
- **Git Safety**: Never leave repository in inconsistent state

## Completion Process
1. Implement the enhanced integrate command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Test with various git scenarios and conflict situations
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Enhance integrate command with advanced git operations"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.