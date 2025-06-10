# Task 12: Clean Configuration System

## Objective
Remove environment variable overrides and legacy config complexity from the Rust configuration system. Make it read only from the JSON config file for normal user operations.

## Current Problems
1. **Environment Variable Confusion**: The current system has `PARA_*` environment variables that override config, but users will never set these
2. **Legacy Config Priority**: Legacy shell config files take precedence over JSON config, making testing difficult
3. **Complex Config Loading**: The config system tries to migrate and merge multiple sources instead of having a clean JSON-first approach

## Requirements

### 1. Simplify Configuration Loading
**Primary Goal**: Make the Rust implementation read ONLY from `~/.config/para/config.json` for normal operations.

**Current problematic flow:**
```rust
// In ConfigManager::load_or_create()
if config_path.exists() {
    Self::load_from_file(&config_path)  // JSON config
} else {
    let legacy_path = get_legacy_config_path();
    if legacy_path.exists() {
        Self::migrate_legacy_config(&legacy_path, &config_path)  // Shell config takes priority!
    }
}

// Then applies environment overrides
config = Self::apply_env_overrides(config);
```

**New desired flow:**
```rust
// Simple: JSON config only
if config_path.exists() {
    Self::load_from_file(&config_path)
} else {
    // Create default config
    Self::save(&default_config())?;
    default_config()
}
```

### 2. Remove Environment Variable Overrides
**Remove from `ConfigManager::apply_env_overrides()`:**
- `PARA_IDE_NAME`
- `PARA_IDE_COMMAND` 
- `PARA_BRANCH_PREFIX`
- `PARA_SUBTREES_DIR`
- `PARA_STATE_DIR`
- `PARA_AUTO_STAGE`
- `PARA_AUTO_COMMIT`
- `PARA_PRESERVE_ON_FINISH`

**Keep ONLY these for the specialized test mode support:**
- `IDE_CMD` (for test mode: `"true"` or `"echo ..."`)

### 3. Remove Legacy Config Migration
**Remove or disable:**
- `migrate_legacy_config()` function
- `parse_legacy_config()` function  
- `get_legacy_config_path()` function

The Rust implementation should not try to read shell config files.

### 4. Clean Configuration Flow
**For normal users:**
1. Check if `~/.config/para/config.json` exists
2. If yes → load it
3. If no → create default config and save it
4. Use that config (no overrides, no migration)

**For specialized test mode only:**
- Check `IDE_CMD` environment variable for test stubs
- This is only for the test mode feature, not configuration

## Files to Modify

### Primary Files
1. **`src/config/manager.rs`**
   - Remove `apply_env_overrides()` (except `IDE_CMD` check)
   - Remove legacy config migration logic
   - Simplify `load_or_create()` to be JSON-only

2. **`src/config/defaults.rs`**
   - Remove `get_legacy_config_path()` if not needed elsewhere
   - Keep `get_config_file_path()` for JSON config

3. **`src/core/ide.rs`** 
   - Keep the `IDE_CMD` environment variable check for test mode
   - Remove any other environment variable dependencies

### Configuration Structure
Keep the current JSON structure but ensure it's the single source of truth:
```json
{
  "ide": {
    "name": "cursor|code|claude",
    "command": "cursor|code|claude", 
    "wrapper": { ... }
  },
  "directories": { ... },
  "git": { ... },
  "session": { ... }
}
```

## Success Criteria
1. **Clean Config Loading**: Only reads from `~/.config/para/config.json`
2. **No Environment Overrides**: `PARA_*` environment variables are ignored
3. **No Legacy Migration**: Shell config files are ignored
4. **Test Mode Still Works**: `IDE_CMD="true"` still enables test mode
5. **Easy Testing**: Can change config by editing JSON file directly

## Testing Requirements
1. **JSON Config Priority**: Verify that changing `~/.config/para/config.json` changes behavior
2. **Environment Variable Isolation**: Verify that `PARA_IDE_NAME=cursor` doesn't override JSON config
3. **Legacy Config Ignored**: Verify that `~/.config/para/config` (shell format) is ignored
4. **Test Mode Preserved**: Verify that `IDE_CMD="true"` still works for test mode
5. **Default Creation**: Verify that missing config creates sensible defaults

## Implementation Notes
- This is a breaking change for anyone using environment variable overrides
- Legacy shell config files will be ignored (not deleted, just ignored)
- The test mode `IDE_CMD` check should remain in the IDE manager for testing purposes
- Focus on making the normal user experience simple and predictable

## Expected Behavior After Fix
```bash
# Edit config file directly
vim ~/.config/para/config.json  # Change "cursor" to "code"

# Test immediately - should use VS Code now
./para-rs/target/debug/para start test-vscode
# Should launch VS Code with the worktree

# Test mode still works
IDE_CMD="true" ./para-rs/target/debug/para start test-stub
# Should skip IDE launch for testing
```

This task should result in a clean, predictable configuration system that users can understand and modify easily.