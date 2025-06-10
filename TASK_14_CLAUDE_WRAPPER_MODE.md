# Task 14: Implement Claude Wrapper Mode Infrastructure

## Objective
Implement the missing Claude Code wrapper mode functionality in the Rust implementation to match the shell version's behavior exactly.

## Background
The Rust implementation currently fails when trying to launch Claude Code in wrapper mode because it's missing the entire wrapper infrastructure. The shell implementation has sophisticated wrapper logic that:

1. **Never launches Claude standalone** - always requires wrapper mode
2. **Launches wrapper IDE instead** - when wrapper is enabled, launches cursor/code instead of claude
3. **Generates auto-run tasks** - creates `.vscode/tasks.json` that automatically starts Claude
4. **Avoids permission conflicts** - by not launching Claude directly from within IDEs

## Current Problem
```bash
./para-rs/target/debug/para start claude-test
# Error: Permission denied: Claude Code should not be launched from within another IDE
```

This happens because the Rust code tries to launch `claude` command directly, triggering permission checks.

## Requirements

### 1. Wrapper Detection Logic
Implement the same logic as shell version (para-ide.sh lines 14-19):

```rust
pub fn launch(&self, path: &Path, skip_permissions: bool) -> Result<()> {
    // Check if IDE wrapper is enabled and we're launching Claude Code
    if self.config.name == "claude" && self.config.wrapper.enabled {
        println!("▶ launching Claude Code inside {} wrapper...", self.config.wrapper.name);
        return self.launch_wrapper(path, skip_permissions);
    }
    
    // Existing direct launch logic...
}
```

### 2. Claude Standalone Prevention
Like shell version (para-ide.sh lines 198-203), Claude should not run standalone:

```rust
if self.config.name == "claude" && !self.config.wrapper.enabled {
    return Err(ParaError::ide_error(
        "Claude Code requires IDE wrapper mode. Please run 'para config' to enable wrapper mode.\n   Available options: VS Code wrapper or Cursor wrapper".to_string()
    ));
}
```

### 3. Wrapper Launch Implementation
Add `launch_wrapper()` method that:

**For Cursor wrapper:**
```rust
fn launch_wrapper(&self, path: &Path, skip_permissions: bool) -> Result<()> {
    match self.config.wrapper.name.as_str() {
        "cursor" => self.launch_cursor_wrapper(path, skip_permissions),
        "code" => self.launch_vscode_wrapper(path, skip_permissions),
        _ => Err(ParaError::ide_error(format!(
            "Unsupported wrapper IDE: {}", self.config.wrapper.name
        )))
    }
}
```

### 4. Auto-Run Task Generation
Implement task JSON generation matching shell version (para-ide.sh lines 282-314):

**Task structure:**
```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Start Claude Code",
            "type": "shell", 
            "command": "claude",
            "group": "build",
            "presentation": {
                "echo": true,
                "reveal": "always", 
                "focus": true,
                "panel": "new",
                "showReuseMessage": false,
                "clear": false
            },
            "runOptions": {
                "runOn": "folderOpen"
            }
        }
    ]
}
```

### 5. Wrapper-Specific Launch Logic

**Cursor wrapper implementation:**
```rust
fn launch_cursor_wrapper(&self, path: &Path, skip_permissions: bool) -> Result<()> {
    // Write auto-run task
    self.write_cursor_autorun_task(path, skip_permissions)?;
    
    // Test mode handling
    if self.is_test_mode() {
        println!("▶ skipping Cursor wrapper launch (test stub)");
        println!("✅ Cursor wrapper (test stub) opened with Claude Code auto-start");
        return Ok();
    }
    
    // Launch Cursor
    let mut cmd = Command::new(&self.config.wrapper.command);
    cmd.arg(path.to_string_lossy().as_ref());
    
    println!("▶ launching Cursor wrapper with Claude Code auto-start...");
    cmd.spawn().map_err(|e| ParaError::ide_error(format!("Failed to launch Cursor wrapper: {}", e)))?;
    println!("✅ Cursor opened - Claude Code will start automatically");
    
    Ok(())
}
```

### 6. Test Mode Compatibility
Ensure wrapper logic respects test mode like shell version:
- Check `IDE_CMD` environment variable for test stubs
- Support both wrapper and primary IDE test modes
- Use same test detection logic as existing implementation

## Files to Modify

### Primary Changes
1. **`src/core/ide.rs`**
   - Add wrapper detection in `launch()` method
   - Add `launch_wrapper()` method
   - Add `launch_cursor_wrapper()` and `launch_vscode_wrapper()` methods
   - Add `write_autorun_task()` methods
   - Add Claude standalone prevention

### Supporting Changes  
2. **`src/core/ide/wrapper.rs`** (new file)
   - Task JSON generation utilities
   - Wrapper-specific helper functions
   - Keep wrapper logic modular and testable

3. **`src/core/ide/mod.rs`** (if needed)
   - Export wrapper module
   - Organize IDE-related functionality

## Implementation Strategy

### Phase 1: Core Wrapper Detection (High Priority)
- Add wrapper detection logic to prevent direct Claude launches
- Add Claude standalone prevention with helpful error messages
- This will fix the immediate permission issue

### Phase 2: Wrapper Launch Infrastructure  
- Implement `launch_wrapper()` and wrapper-specific launch methods
- Add basic wrapper IDE launching (without auto-run tasks initially)
- Focus on Cursor wrapper first since that's most commonly used

### Phase 3: Auto-Run Task Generation
- Implement `.vscode/tasks.json` generation
- Add proper command escaping and argument handling
- Test auto-run functionality with real IDEs

### Phase 4: Complete Feature Parity
- Add VS Code wrapper support  
- Add session resumption and prompt handling in tasks
- Ensure complete compatibility with shell version

## Success Criteria

### Functional Requirements
1. **Wrapper Mode Works**: `claude + wrapper.enabled=true` launches wrapper IDE with auto-run tasks
2. **No Permission Errors**: No more "Claude Code should not be launched from within another IDE" errors
3. **Auto-Run Tasks Work**: Generated tasks successfully start Claude in wrapper IDEs
4. **Standalone Prevention**: `claude + wrapper.enabled=false` shows helpful error message

### Compatibility Requirements  
5. **Shell Parity**: Behaves identically to shell implementation wrapper mode
6. **Test Mode Preserved**: Test stubs work correctly in wrapper scenarios
7. **Configuration Respect**: Uses `config.ide.wrapper.*` settings correctly

### Testing Requirements
8. **Cursor Wrapper**: Successfully launches Cursor and starts Claude automatically
9. **VS Code Wrapper**: Successfully launches VS Code and starts Claude automatically  
10. **Test Mode**: Wrapper test stubs work without launching actual IDEs

## Example Usage After Implementation

```bash
# Configure wrapper mode
./para-rs/target/debug/para config show
# Shows: "wrapper": {"enabled": true, "name": "cursor", "command": "cursor"}

# Launch with wrapper - should work
./para-rs/target/debug/para start claude-test
# ▶ launching Claude Code inside cursor wrapper...
# ▶ launching Cursor wrapper with Claude Code auto-start...  
# ✅ Cursor opened - Claude Code will start automatically

# Try standalone Claude - should show helpful error
# (after changing config to wrapper.enabled=false)
./para-rs/target/debug/para start claude-test  
# Error: Claude Code requires IDE wrapper mode. Please run 'para config' to enable wrapper mode.
```

## Dependencies
- **Depends on**: Clean configuration system (Task 12) for proper wrapper config access
- **Blocks**: Any Claude Code usage scenarios until wrapper mode works
- **Priority**: High - this is a core missing feature preventing Claude wrapper usage

This task implements a fundamental missing feature that's required for Claude Code integration to work properly with the Rust implementation.