# Task 11: Simplify Configuration Wizard

## Objective
Simplify the Rust configuration wizard to match the shell version's clean approach and remove unnecessary complexity while adding test mode support.

## Current Problems
1. **Overcomplicated Configuration**: The current wizard asks too many questions that should have sensible defaults
2. **Missing Test Mode**: Unlike the shell version, the Rust version doesn't support test mode stubs, causing permission errors during testing
3. **Unclear IDE Mode Selection**: The current flow doesn't clearly distinguish between standalone IDE modes vs wrapped Claude Code modes
4. **Auto-stage/Auto-commit Questions**: These should be defaults, not user choices

## Requirements

### 1. Simplify IDE Mode Selection
The configuration should offer 3 clear modes:
- **`cursor`** → Direct Cursor IDE (standalone mode)
- **`code`** → Direct VS Code IDE (standalone mode)  
- **`claude`** → Claude Code wrapped inside another IDE (requires wrapper config)

### 2. Remove Unnecessary Configuration Options
Remove these from the wizard (make them defaults):
- ❌ "Automatically stage all changes when finishing sessions?" → Default: `true`
- ❌ "Automatically commit staged changes when finishing sessions?" → Default: `true`
- ❌ "Preserve session data for recovery after finishing sessions?" → Default: `true`
- ❌ "Days to keep old sessions" complexity → Default: `30`
- ❌ Custom user data directory prompts → Use sensible defaults

### 3. Simplified Configuration Flow
```
🔧 Para Configuration Wizard

🖥️  IDE Selection
1. Which IDE would you like to use?
   • cursor (Direct Cursor IDE)
   • code (Direct VS Code IDE)
   • claude (Claude Code inside another IDE)

[If claude selected:]
2. Which IDE should wrap Claude Code?
   • cursor
   • code

📁 Directories (optional customization)
3. Subtrees directory: [subtrees/pc]
4. State directory: [.para_state]

✅ Done!
```

### 4. Add Test Mode Support
The Rust version needs the same test mode logic as the shell version (`lib/para-ide.sh` lines 181-186, 189-196):

**In IDE Manager (`src/core/ide.rs`):**
- Check for `IDE_CMD="true"` → skip actual IDE launch, return success
- Check for `IDE_CMD="echo ..."` → use stub command instead of real launch
- Only perform permission checks when actually launching IDEs

**Test mode detection logic:**
```rust
fn is_test_mode(&self) -> bool {
    self.config.command == "true" || self.config.command.starts_with("echo ")
}

fn handle_test_mode(&self, path: &Path) -> Result<()> {
    if self.config.command == "true" {
        println!("▶ skipping {} launch (test stub)", self.config.name);
        println!("✅ {} (test stub) opened", self.config.name);
        return Ok(());
    }
    
    if self.config.command.starts_with("echo ") {
        // Handle echo stub commands
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(format!("{} \"{}\"", self.config.command, path.display()));
        cmd.output()?;
        return Ok(());
    }
    
    unreachable!()
}
```

### 5. Update Default Configuration
**In `src/config/defaults.rs`:**
- Set default IDE to `cursor` (not `claude`)
- Set `auto_stage: true` and `auto_commit: true` as non-configurable defaults
- Set `preserve_on_finish: true` as default
- Set `auto_cleanup_days: Some(30)` as default

### 6. Reference Implementation
**Shell version location:** `/Users/marius.wichtner/Documents/git/para/lib/para-config.sh`
- Study lines 63-77 for IDE validation
- Study lines 280-333 for simple config command handling
- Study lines 354-357 for get_default_ide()

**Shell IDE launch logic:** `/Users/marius.wichtner/Documents/git/para/lib/para-ide.sh`
- Study lines 181-186 for test mode with `IDE_CMD="true"`
- Study lines 189-196 for echo stub commands
- Study lines 198-207 for wrapper mode requirements

## Files to Modify

### Primary Files
1. **`src/config/wizard.rs`** - Simplify the configuration wizard
2. **`src/core/ide.rs`** - Add test mode support
3. **`src/config/defaults.rs`** - Update default values
4. **`src/config/mod.rs`** - Remove unused config fields if needed

### Configuration Structure
Update the configuration to match this simplified structure:
```rust
pub struct Config {
    pub ide: IdeConfig,        // Simplified IDE selection
    pub directories: DirectoryConfig,  // Keep as-is
    // Remove git auto-stage/auto-commit options (make them defaults)
    // Remove session preserve/cleanup complexity
}
```

## Success Criteria
1. **Simple Configuration Flow**: The wizard asks only essential questions
2. **Test Mode Support**: `cargo run -- start test` works without permission errors when IDE_CMD is set to test values
3. **Clear IDE Modes**: Users understand the difference between direct IDE usage vs Claude Code wrapped modes
4. **Backward Compatibility**: Existing configurations continue to work
5. **Sensible Defaults**: Common use cases work out-of-the-box without extensive configuration

## Testing Requirements
1. **Test Mode Verification**: Verify that setting `IDE_CMD="true"` skips actual IDE launch
2. **Permission Check Bypass**: Ensure test mode bypasses the Claude Code permission check
3. **Configuration Simplification**: Verify the wizard flow is streamlined
4. **Default Values**: Verify auto-stage and auto-commit work as defaults
5. **IDE Mode Selection**: Test all three IDE modes (cursor/code/claude-wrapped)

## Implementation Notes
- Keep the existing configuration file format for backward compatibility
- Maintain the same CLI interface (`para config`, `para config show`, etc.)
- Ensure test mode detection happens before permission checks
- Follow the shell version's approach for test mode stubs exactly

This task should result in a configuration system that's as simple and user-friendly as the shell version while maintaining the Rust implementation's reliability and features.