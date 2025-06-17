# Add Configuration Support for Multi-Dispatch Mode

## Objective
Make the multi-dispatch feature configurable through Para's configuration system, allowing users to enable/disable worktree-integrated mode for Claude Code instances.

## Requirements

### 1. Update Configuration Schema (`src/config/mod.rs`)

Add a new configuration section for multi-dispatch settings:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDispatchConfig {
    /// Enable worktree-integrated mode for Claude Code
    #[serde(default = "default_worktree_integrated_mode")]
    pub worktree_integrated_mode: bool,
    
    /// Default number of terminals to create if not specified
    #[serde(default = "default_terminal_count")]
    pub default_terminal_count: usize,
    
    /// Automatically launch all Claude instances on workspace open
    #[serde(default = "default_auto_launch")]
    pub auto_launch: bool,
}
```

Add this to the main `Config` struct:
```rust
pub struct Config {
    // ... existing fields ...
    
    /// Multi-dispatch configuration
    #[serde(default)]
    pub multi_dispatch: MultiDispatchConfig,
}
```

### 2. Add Default Functions (`src/config/defaults.rs`)

Create default functions for the new configuration:
```rust
pub fn default_multi_dispatch_config() -> MultiDispatchConfig {
    MultiDispatchConfig {
        worktree_integrated_mode: false,  // Disabled by default
        default_terminal_count: 3,
        auto_launch: true,
    }
}

pub fn default_worktree_integrated_mode() -> bool { false }
pub fn default_terminal_count() -> usize { 3 }
pub fn default_auto_launch() -> bool { true }
```

### 3. Update Configuration Wizard (`src/config/wizard.rs`)

Add prompts for multi-dispatch configuration:
- Ask if user wants to enable worktree-integrated mode
- Ask for default terminal count (with validation for reasonable limits)
- Ask if terminals should auto-launch

### 4. Modify Dispatch Command Behavior

Update both `dispatch.rs` and `multi_dispatch.rs` to check the configuration:

```rust
// In dispatch.rs
if config.multi_dispatch.worktree_integrated_mode {
    // Use multi-dispatch logic even for single dispatch
    // Create workspace with single worktree
} else {
    // Use existing single dispatch logic
}
```

### 5. Update Multi-Dispatch Command

The `multi_dispatch.rs` should:
- Check if worktree-integrated mode is enabled
- If disabled, show an error message suggesting to enable it in config
- Use `default_terminal_count` when user doesn't specify session count
- Respect `auto_launch` setting in tasks.json generation

### 6. Add Configuration Migration

If config file exists without multi_dispatch section:
- Add the section with defaults during config loading
- Save the updated config to preserve the new structure

### 7. Update Help Text and Documentation

- Add configuration options to `para config` help
- Document the new settings in CLI help
- Update any relevant error messages

## Implementation Notes

- Ensure backward compatibility - old configs should work without multi_dispatch section
- The configuration should be validated on load
- Consider adding a `para config set` command for easy toggling:
  ```bash
  para config set multi_dispatch.worktree_integrated_mode true
  ```

## Testing Requirements

- Test config loading with and without multi_dispatch section
- Test configuration wizard includes new prompts
- Test dispatch behavior changes based on configuration
- Test that multi-dispatch respects all configuration options
- Ensure proper error messages when features are disabled

## Example Configuration

```json
{
  "ide": {
    "name": "claude",
    "command": "claude"
  },
  "multi_dispatch": {
    "worktree_integrated_mode": true,
    "default_terminal_count": 4,
    "auto_launch": true
  }
}
```