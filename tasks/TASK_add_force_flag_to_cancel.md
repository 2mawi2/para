# Add --force flag to para cancel command

## Goal
Allow the orchestrator to cancel agent sessions with uncommitted changes by adding a `--force` flag to the `para cancel` command.

## Requirements
1. Add a `--force` flag to the `para cancel` command
2. When `--force` is used:
   - Skip the uncommitted changes check
   - Allow cancellation even with uncommitted work
   - The flag should work in both interactive and non-interactive modes
3. Update the MCP server to pass the force flag when the orchestrator needs to cancel sessions
4. Maintain existing safety behavior when `--force` is not used

## Implementation Notes
- The force flag should be clearly documented as destructive
- Consider adding a warning message when force is used
- Update help text to explain the risks of using --force

## Testing
- Test cancellation with uncommitted changes using --force
- Test that normal cancellation still prevents data loss
- Test MCP integration with the new flag

When done: para finish "Add --force flag to para cancel for orchestrator use"