# Task 7: Interactive Configuration Wizard

## Objective
Enhance the `para config` command with a comprehensive interactive wizard that guides users through first-time setup and configuration management.

## Key Requirements

### Command Behavior
- `para config` - Launch interactive configuration wizard
- `para config show` - Display current configuration
- `para config edit` - Open config file in editor
- `para config auto` - Auto-detect and configure IDE settings
- `para config reset` - Reset configuration to defaults

### Core Functionality
1. **Interactive Wizard**: Step-by-step configuration setup using dialoguer
2. **IDE Detection**: Auto-detect available IDEs (Claude Code, Cursor, VS Code)
3. **Path Validation**: Validate IDE paths and commands
4. **Configuration Validation**: Ensure all settings are valid
5. **First-Run Experience**: Seamless onboarding for new users
6. **Configuration Migration**: Handle upgrading from old config formats

### Implementation Files to Modify
- `src/cli/commands/config.rs` - Main command implementation
- `src/config/wizard.rs` - Interactive wizard implementation
- `src/config/mod.rs` - Configuration management enhancements
- `src/config/validation.rs` - Configuration validation logic

### Expected Integration Points
- Use `dialoguer` crate for interactive prompts
- Use `Config` struct for configuration management
- Use `directories` crate for cross-platform config paths
- Integrate with existing IDE detection logic
- Use file system utilities for config file operations

### Interactive Wizard Flow
1. **Welcome Screen**: Introduction and overview
2. **IDE Selection**: Detect and choose primary IDE
   - Auto-detect installed IDEs
   - Allow manual path specification
   - Validate IDE executables
3. **Directory Configuration**: Setup subtree and state directories
   - Validate write permissions
   - Create directories if needed
4. **Git Configuration**: Branch prefix and merge preferences
   - Validate git repository access
   - Set default merge strategies
5. **Session Configuration**: Default naming and cleanup policies
   - Configure session naming format
   - Set auto-cleanup preferences
6. **Confirmation**: Review and save configuration
   - Display final configuration
   - Confirm and write config file

### IDE Detection Logic
- **Claude Code**: Check for `claude` command in PATH
- **Cursor**: Check for `cursor` command in PATH and standard install locations
- **VS Code**: Check for `code` command in PATH and standard install locations
- **Wrapper Mode**: Detect when running inside another IDE terminal
- **Custom IDEs**: Allow manual specification of command and arguments

### Configuration Validation
- Validate IDE commands are executable
- Ensure directories are writable
- Check git repository accessibility
- Validate session naming patterns
- Verify all required fields are present

### Success Criteria
- Interactive wizard guides users through complete setup
- Auto-detects common IDE installations accurately
- Validates all configuration settings before saving
- Handles edge cases (missing IDEs, permission issues)
- Provides clear error messages with solutions
- Compatible with existing configuration files
- Works on all supported platforms (macOS)

### Error Handling
- Handle case when no IDEs are detected
- Handle case when config directory is not writable
- Handle case when IDE paths are invalid
- Handle case when git is not available
- Handle case when config file is corrupted
- Provide helpful error messages with suggested fixes
- Allow users to retry failed steps

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for wizard logic
- **Integration Tests**: Test end-to-end configuration workflow
- **Platform Tests**: Test on different operating systems
- **IDE Detection Tests**: Test IDE detection on various systems
- **Configuration Tests**: Test config validation and migration
- **Error Handling Tests**: Test all error scenarios
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors
- **User Experience**: Intuitive and helpful interactive flow

## Completion Process
1. Implement the configuration wizard functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Test on multiple platforms if possible
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement interactive configuration wizard"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.