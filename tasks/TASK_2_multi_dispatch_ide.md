# Implement Multi-Worktree IDE Integration

## Objective
Extend the `IdeManager` in `src/core/ide.rs` to support launching multiple Claude Code instances in split terminals within a single VS Code window.

## Requirements

### 1. Add New Methods to `IdeManager` (`src/core/ide.rs`)

#### Main Public Method
```rust
pub fn setup_multi_worktree_workspace(
    &self,
    workspace_root: &Path,
    worktrees: &[(String, PathBuf)],
    skip_permissions: bool,
) -> Result<()>
```
- Create `.vscode` directory at workspace root
- Generate multi-worktree `tasks.json`
- Create workspace file `claude-worktrees.code-workspace`

#### Helper Methods
1. `fn create_multi_worktree_tasks(&self, worktrees: &[(String, PathBuf)], skip_permissions: bool) -> String`
   - Generate tasks.json with individual Claude tasks for each worktree
   - Build Claude command with optional `--dangerously-skip-permissions` and `--no-confirm`
   - Create compound task "Launch All Claude Instances" that runs all tasks in parallel
   - Use `presentation.group: "claude-worktrees"` for side-by-side terminals
   - Set `runOn: "folderOpen"` for automatic execution

2. `fn create_workspace_file(&self, worktrees: &[(String, PathBuf)]) -> String`
   - Generate VS Code workspace file with all worktrees as folders
   - Add emoji prefix "🤖" to folder names for clarity
   - Configure terminal settings for better multi-terminal experience
   - Include Git extension recommendations

3. `pub fn launch_multi_worktree_workspace(&self, workspace_file: &Path) -> Result<()>`
   - Launch VS Code (or wrapper IDE) with the workspace file
   - Use wrapper command if enabled, otherwise default to "code"

### 2. Implementation Details

#### tasks.json Structure
- Individual tasks for each worktree with dedicated terminal panels
- Compound task that launches all instances in parallel
- Proper working directory (`cwd`) for each task
- Terminal presentation settings for split view

#### workspace.json Structure
- Multiple folder entries with worktree paths
- Custom folder names with Claude indicator
- Terminal tab settings for better organization
- Window title customization

### 3. Error Handling
- Wrap all file system operations with proper error messages
- Use `ParaError::ide_error` for IDE-related failures
- Use `ParaError::fs_error` for file system failures

### 4. Integration Points
- The `multi_dispatch` command will call these methods
- Maintain compatibility with existing wrapper mode functionality
- Ensure proper path handling across platforms

## Testing Considerations
- Test workspace file generation with multiple worktrees
- Verify tasks.json structure is valid
- Test with and without permission skip flag
- Ensure wrapper mode compatibility

## Notes
- Follow existing patterns in `ide.rs` for consistency
- The generated files should be valid JSON
- Terminal groups ensure side-by-side display
- The workspace approach provides better VS Code integration than manual terminal creation

When done: para finish "Add multi-worktree IDE integration to IdeManager"