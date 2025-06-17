# Implement Workspace-Integrated Dispatch Mode

## Objective
Modify the dispatch command to support a "workspace-integrated mode" where subsequent dispatches add new worktrees to an existing VS Code workspace instead of opening new IDE windows.

## Design Overview

When workspace-integrated mode is enabled:
1. First dispatch creates a VS Code workspace file and opens it
2. Subsequent dispatches update the workspace file to add new folders
3. Each dispatch adds a new Claude terminal task to the workspace
4. VS Code reloads to show the new folder and terminal

## Requirements

### 1. Update Configuration (`src/config/mod.rs`)

Add workspace integration settings to `IdeConfig`:
```rust
pub struct IdeConfig {
    // ... existing fields ...
    
    /// Enable workspace-integrated dispatch mode
    #[serde(default = "default_workspace_integrated_mode")]
    pub workspace_integrated_mode: bool,
    
    /// Path to active workspace file (managed by para)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_workspace_file: Option<String>,
}
```

### 2. Create Workspace Manager (`src/core/workspace.rs`)

New module to handle workspace operations:

```rust
pub struct WorkspaceManager {
    config: Config,
    workspace_dir: PathBuf,  // .para/workspaces/
}

impl WorkspaceManager {
    /// Get or create the main workspace file for the repository
    pub fn get_or_create_workspace(&self) -> Result<PathBuf>;
    
    /// Add a worktree folder to existing workspace
    pub fn add_folder_to_workspace(&self, session_name: &str, worktree_path: &Path) -> Result<()>;
    
    /// Update tasks.json to include new Claude terminal
    pub fn add_claude_task(&self, session_name: &str, worktree_path: &Path, prompt: &str) -> Result<()>;
    
    /// Check if VS Code is running with this workspace
    pub fn is_workspace_open(&self) -> bool;
    
    /// Reload workspace in VS Code (using --reuse-window)
    pub fn reload_workspace(&self) -> Result<()>;
}
```

### 3. Modify Dispatch Command (`src/cli/commands/dispatch.rs`)

Update the execute function to check workspace mode:

```rust
pub fn execute(config: Config, args: DispatchArgs) -> Result<()> {
    // ... existing validation and setup ...
    
    if config.ide.workspace_integrated_mode {
        execute_workspace_integrated(config, session_state, prompt, args.dangerously_skip_permissions)
    } else {
        // Existing standalone dispatch logic
        launch_claude_code(&config, &session_state.worktree_path, &prompt, args.dangerously_skip_permissions)
    }
}

fn execute_workspace_integrated(
    config: Config,
    session_state: SessionState,
    prompt: &str,
    skip_permissions: bool,
) -> Result<()> {
    let workspace_manager = WorkspaceManager::new(&config);
    
    // Get or create main workspace
    let workspace_file = workspace_manager.get_or_create_workspace()?;
    
    // Add new folder to workspace
    workspace_manager.add_folder_to_workspace(&session_state.name, &session_state.worktree_path)?;
    
    // Add Claude task for this session
    workspace_manager.add_claude_task(&session_state.name, &session_state.worktree_path, prompt)?;
    
    // Launch or reload VS Code
    if workspace_manager.is_workspace_open() {
        workspace_manager.reload_workspace()?;
    } else {
        launch_vscode_workspace(&config, &workspace_file)?;
    }
    
    Ok(())
}
```

### 4. Workspace File Structure

Create workspace at `.para/workspaces/main.code-workspace`:
```json
{
  "folders": [
    {
      "path": "../../",
      "name": "📁 Main Repository"
    },
    {
      "path": "../worktrees/session-1",
      "name": "🤖 session-1 (Claude)"
    }
  ],
  "settings": {
    "terminal.integrated.tabs.enabled": true,
    "terminal.integrated.tabs.location": "left"
  },
  "tasks": {
    "version": "2.0.0",
    "tasks": [
      {
        "label": "Claude: session-1",
        "type": "shell",
        "command": "claude --dangerously-skip-permissions",
        "options": {
          "cwd": "${workspaceFolder:🤖 session-1 (Claude)}"
        },
        "presentation": {
          "group": "claude-sessions",
          "reveal": "always"
        }
      }
    ]
  }
}
```

### 5. Update Configuration Wizard (`src/config/wizard.rs`)

Add prompt for workspace-integrated mode:
```rust
let workspace_integrated = Confirm::new()
    .with_prompt("Enable workspace-integrated dispatch mode? (Recommended for VS Code)")
    .default(true)
    .interact()?;
```

### 6. Handle Edge Cases

- **First dispatch**: Creates workspace and opens VS Code
- **Subsequent dispatches**: Updates workspace file and reloads
- **Manual IDE close**: Detect and handle gracefully
- **Finish command**: Should work with workspace-integrated sessions
- **Cancel command**: Remove folder from workspace

### 7. Platform Considerations

- Use `code --reuse-window <workspace>` to reload
- Fall back to opening new window if reload fails
- Store workspace state in config for persistence

## Benefits

1. **Single IDE Window**: All work happens in one VS Code instance
2. **Better Overview**: See all active sessions in the file explorer
3. **Terminal Organization**: All Claude terminals in one window
4. **Resource Efficient**: Fewer IDE processes running

## Testing Requirements

- Test first dispatch creates workspace correctly
- Test subsequent dispatches add folders properly
- Test Claude tasks launch in correct directories
- Test finish/cancel commands update workspace
- Test config migration for existing users
- Test fallback when VS Code is closed

## Implementation Notes

- Workspace file uses relative paths for portability
- Tasks are embedded in workspace file (VS Code supports this)
- Consider adding `para workspace` command to manage workspaces
- Ensure backward compatibility with standalone mode

