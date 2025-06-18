# Task: Remove Standalone IDE Launching

## Objective
Remove the ability to launch VS Code and Cursor as standalone IDEs. All IDE launching should go through the cloud-based wrapper mode, similar to how Claude Code currently works.

## Context
Currently, para supports two types of IDE launching:
1. **Standalone**: Direct launching of VS Code and Cursor via their CLI commands
2. **Cloud-based**: Launching through a wrapper IDE (required for Claude Code)

We want to remove option 1 and force all IDEs to use the cloud-based wrapper approach.

## Implementation Requirements

### 1. Core IDE Module Changes (`src/core/ide.rs`)
- Remove the direct launch code path (lines 45-64 that handle non-Claude IDEs)
- Force all IDEs to use wrapper mode, not just Claude
- Update the launch logic to always use `launch_wrapper_with_options()`
- Remove the condition that only Claude requires wrapper mode
- Update error messages to be generic for all IDEs

### 2. Configuration Changes
- Update `src/config/defaults.rs` to remove standalone IDE options
- Ensure all default configurations use wrapper mode
- Update the configuration wizard in `src/config/wizard.rs` to:
  - Only offer cloud-based options
  - Remove the choice between wrapper and non-wrapper modes
  - Simplify the IDE selection process

### 3. Validation Updates
- Modify configuration validation to reject configs without wrapper mode enabled
- Add migration logic for existing configurations:
  - If wrapper is not enabled, automatically enable it
  - If wrapper command is not set, use the IDE command as wrapper command
  - Log migration actions for user awareness

### 4. Error Handling
- Update error messages throughout the codebase
- Change "Claude Code requires IDE wrapper mode" to "All IDEs require wrapper mode for cloud-based launching"
- Provide helpful guidance on how to update configurations

### 5. Testing Updates
- Update all tests in `src/core/ide.rs` and related modules
- Remove tests that verify standalone launching
- Add tests for:
  - Wrapper mode enforcement
  - Configuration migration
  - Error cases when wrapper is not properly configured
- Ensure test utilities use wrapper mode configurations

### 6. Documentation Updates
- Update all references to standalone IDE launching in comments
- Update CLAUDE.md to reflect the new cloud-only approach
- Update any user-facing documentation or help text

## Testing Requirements
- All existing tests must pass with the new wrapper-only approach
- Add specific tests for migration scenarios
- Verify that old configurations are properly migrated
- Test error cases thoroughly

## Important Notes
- Preserve the existing wrapper implementation logic
- Keep the `.vscode/tasks.json` generation as-is
- Maintain backward compatibility by migrating old configs
- Ensure the change is transparent to users with proper configs

When done: para finish "Remove standalone IDE launching - force cloud-based wrapper mode for all IDEs"