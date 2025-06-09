# Task 9: Session Recovery System

## Objective
Complete the `para recover` and `para resume` commands to provide robust session recovery and resumption capabilities for interrupted workflows.

## Key Requirements

### Command Behavior
- `para recover` - List all recoverable sessions (cancelled/archived)
- `para recover session-name` - Recover specific cancelled session
- `para recover --all` - Recover all recent cancelled sessions
- `para resume` - Resume current session in IDE
- `para resume session-name` - Resume specific session in IDE
- `para resume --list` - List resumable sessions

### Core Functionality
1. **Session Recovery**: Restore cancelled sessions from archives
2. **Session Resumption**: Reopen existing sessions in IDE
3. **Archive Management**: Browse and manage archived sessions
4. **State Restoration**: Fully restore session state and context
5. **Conflict Detection**: Handle naming conflicts during recovery
6. **Recovery History**: Track recovery operations and success rates

### Implementation Files to Modify
- `src/cli/commands/recover.rs` - Main recover command implementation
- `src/cli/commands/resume.rs` - Main resume command implementation
- `src/core/session/recovery.rs` - Session recovery logic
- `src/core/session/archive.rs` - Archive management

### Expected Integration Points
- Use `GitService` for git operations and branch management
- Use `SessionManager` for session state management
- Use `BranchManager` for archived branch operations
- Use `WorktreeManager` for worktree recreation
- Use `IdeManager` for IDE launching
- Integrate with existing configuration system

### Recovery Workflow
1. **Archive Discovery**: Find and list archived sessions
2. **Session Selection**: Allow user to choose session to recover
3. **Conflict Resolution**: Handle naming conflicts with existing sessions
4. **Branch Restoration**: Recreate branch from archive
5. **Worktree Recreation**: Recreate worktree in appropriate location
6. **State Restoration**: Restore session state and metadata
7. **IDE Launch**: Optionally launch IDE in recovered session

### Resume Workflow
1. **Session Detection**: Detect current or specified session
2. **State Validation**: Verify session is valid and accessible
3. **IDE Launch**: Launch configured IDE in session directory
4. **Context Restoration**: Restore any saved IDE context
5. **Session Activation**: Mark session as current/active

### Archive Management
- **Archive Listing**: Display archived sessions with metadata
- **Archive Browsing**: Show archive details and commit history
- **Archive Cleanup**: Remove old or unwanted archives
- **Archive Export**: Export session data for backup/sharing
- **Archive Import**: Import sessions from backup files

### Session State Management
- **State Persistence**: Save comprehensive session metadata
- **State Validation**: Verify session state integrity
- **State Migration**: Handle state format upgrades
- **State Backup**: Create backups before risky operations
- **State Recovery**: Restore from corrupted state files

### Recovery Conflict Resolution
- **Name Conflicts**: Handle when recovered session name already exists
- **Branch Conflicts**: Handle when branch name conflicts with existing branch
- **Directory Conflicts**: Handle when worktree directory conflicts
- **State Conflicts**: Handle when session state is inconsistent
- **Auto-Resolution**: Automatically resolve simple conflicts
- **User Interaction**: Prompt user for complex conflict resolution

### IDE Integration
- **Session Context**: Restore IDE-specific session context
- **Project Settings**: Restore project configuration if applicable
- **Window Layout**: Restore window/tab layout where possible
- **Recent Files**: Restore recently opened files list
- **Workspace State**: Restore workspace-specific settings

### Success Criteria
- Successfully recovers cancelled sessions from archives
- Recreates proper git branches and worktrees
- Restores session state and metadata accurately
- Handles naming and directory conflicts gracefully
- Launches IDE with proper session context
- Maintains session list and status correctly
- Works with all supported session types
- Preserves git history and commit information

### Error Handling
- Handle case when archived session is corrupted
- Handle case when branch archive is missing
- Handle case when worktree location is not available
- Handle case when IDE launch fails
- Handle case when session state is corrupted
- Handle case when git repository is in inconsistent state
- Provide helpful error messages with suggested fixes
- Allow partial recovery when some components fail

### Recovery Options
- **Full Recovery**: Restore complete session with all components
- **Branch Only**: Recover just the git branch without worktree
- **Partial Recovery**: Recover what's possible, report what failed
- **Recovery Preview**: Show what would be recovered without executing
- **Recovery Validation**: Verify recovery is possible before attempting

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for recovery logic
- **Integration Tests**: Test end-to-end recovery and resume workflows
- **Archive Tests**: Test archive management and browsing
- **Conflict Tests**: Test various conflict scenarios and resolutions
- **State Tests**: Test session state persistence and restoration
- **IDE Tests**: Test IDE integration and context restoration
- **Error Handling Tests**: Test all error conditions and recovery
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors
- **Data Safety**: Never lose session data during recovery operations

## Completion Process
1. Implement the session recovery and resume functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Test with various recovery scenarios and edge cases
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement session recovery and resume system"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.