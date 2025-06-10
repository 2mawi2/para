# Task 13: Legacy Test Compatibility Layer

## Objective
**DEPENDS ON: Task 12 (Clean Configuration System)**

Create a compatibility layer that allows legacy shell tests to work with the Rust implementation by translating environment variables to temporary JSON config changes.

## Background
After Task 12 removes environment variable overrides from normal operation, we still need the legacy bats tests (which use environment variables) to work with the Rust implementation during the transition period.

## Requirements

### 1. Legacy Test Environment Detection
Detect when running in a legacy test environment and enable compatibility mode.

**Detection methods:**
- Environment variable: `PARA_LEGACY_TEST_MODE=true`
- Or detect common test environment variables: `BATS_TEST_FILENAME`, `CI`, etc.

### 2. Environment Variable Translation
When in legacy test mode, translate shell-style environment variables to temporary JSON config changes.

**Environment variables to support:**
- `IDE_NAME` → `config.ide.name`
- `IDE_CMD` → `config.ide.command` 
- `CURSOR_CMD` → `config.ide.command` (legacy compatibility)
- `BRANCH_PREFIX` → `config.git.branch_prefix`
- `SUBTREES_DIR_NAME` → `config.directories.subtrees_dir`
- `STATE_DIR_NAME` → `config.directories.state_dir`

### 3. Temporary Config Override
Create a mechanism to temporarily override config for the duration of a single command run, without modifying the actual config file.

**Implementation approach:**
```rust
impl ConfigManager {
    pub fn load_with_legacy_overrides() -> Result<Config> {
        let mut config = Self::load_or_create()?;
        
        if Self::is_legacy_test_mode() {
            config = Self::apply_legacy_env_overrides(config);
        }
        
        Ok(config)
    }
    
    fn is_legacy_test_mode() -> bool {
        std::env::var("PARA_LEGACY_TEST_MODE").is_ok() ||
        std::env::var("BATS_TEST_FILENAME").is_ok()
    }
    
    fn apply_legacy_env_overrides(mut config: Config) -> Config {
        // Translate legacy environment variables
        if let Ok(ide_name) = std::env::var("IDE_NAME") {
            config.ide.name = ide_name;
        }
        // ... etc
    }
}
```

### 4. Wrapper Mode Translation
Handle the shell script's wrapper mode environment variables:

**Shell variables:**
- `IDE_WRAPPER_ENABLED` → `config.ide.wrapper.enabled`
- `IDE_WRAPPER_NAME` → `config.ide.wrapper.name`
- `IDE_WRAPPER_CMD` → `config.ide.wrapper.command`

### 5. Test Mode Preservation
Ensure that the test mode features from Task 11 still work:
- `IDE_CMD="true"` → skip IDE launch
- `IDE_CMD="echo ..."` → use echo stub

## Files to Modify

### Primary Files
1. **`src/config/manager.rs`**
   - Add `load_with_legacy_overrides()` method
   - Add `is_legacy_test_mode()` detection
   - Add `apply_legacy_env_overrides()` translation

2. **`src/main.rs` or entry points**
   - Use `load_with_legacy_overrides()` instead of `load_or_create()` at startup

3. **`src/config/legacy.rs`** (new file)
   - Contains all legacy test compatibility logic
   - Keeps this isolated from the main config system

### Example Usage
```rust
// Normal operation (Task 12)
let config = ConfigManager::load_or_create()?;  // JSON only

// Legacy test mode (Task 13)
let config = ConfigManager::load_with_legacy_overrides()?;  // JSON + env overrides
```

## Success Criteria
1. **Normal Users Unaffected**: Regular users see no change (JSON config only)
2. **Legacy Tests Work**: Existing bats tests pass with environment variable overrides
3. **Test Mode Preserved**: `IDE_CMD="true"` test mode still works
4. **No Config File Pollution**: Environment variables don't modify the actual JSON config file
5. **Clean Separation**: Legacy compatibility code is isolated and can be removed later

## Testing Requirements
1. **Legacy Test Simulation**: Verify environment variables work in legacy mode
   ```bash
   PARA_LEGACY_TEST_MODE=true IDE_NAME=code ./para-rs/target/debug/para start test
   # Should use VS Code even if JSON config says cursor
   ```

2. **Normal Mode Isolation**: Verify environment variables are ignored in normal mode
   ```bash
   IDE_NAME=code ./para-rs/target/debug/para start test
   # Should use whatever is in JSON config, ignoring IDE_NAME
   ```

3. **Test Mode Compatibility**: Verify test mode works in both modes
   ```bash
   IDE_CMD="true" ./para-rs/target/debug/para start test
   # Should skip IDE launch in both normal and legacy modes
   ```

## Implementation Strategy
1. **Phase 1**: Add legacy compatibility alongside existing system
2. **Phase 2**: Test with existing bats tests
3. **Phase 3**: Document the transition plan for removing legacy compatibility later

## Future Removal Plan
This compatibility layer should be temporary:
- Once all tests are converted to JSON config approach
- Or once the Rust implementation fully replaces the shell version
- The legacy compatibility code can be removed

## Dependencies
- **MUST complete Task 12 first** (Clean Configuration System)
- Task 12 provides the clean JSON-only foundation
- Task 13 adds the compatibility layer on top

This task ensures a smooth transition period where both approaches work, but the default behavior is clean and user-friendly.