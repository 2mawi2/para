# Task: Remove para integrate command

## Objective
Remove the `para integrate` command completely from the codebase. This feature is no longer supported and should be removed to simplify the application.

## Requirements

1. **Remove CLI Command**
   - Remove the `integrate` subcommand from the CLI
   - Remove all associated command handlers and logic
   - Update help text and command listings

2. **Update MCP Server**
   - Remove `para_integrate` tool from mcp-server-ts/src/para-mcp-server.ts
   - Remove all references to integrate in tool descriptions
   - Update dispatch tool description to only mention `para finish`

3. **Update Documentation**
   - README.md - remove all mentions of `para integrate`
   - docs/SAMPLE_PARA_INSTRUCTIONS.md - remove integrate workflow
   - CLAUDE.md - update any workflow instructions
   - Any other documentation files

4. **Remove Implementation**
   - Remove integrate command implementation
   - Remove integration strategies (squash, merge, rebase)
   - Remove auto-integration logic
   - Clean up any helper functions only used by integrate

5. **Remove Tests**
   - Remove all tests for the integrate command
   - Update any tests that reference integrate
   - Ensure remaining tests still pass

6. **Update Workflow Instructions**
   - Change all "para integrate" references to "para finish"
   - Update task examples to use finish instead of integrate
   - Ensure consistency across all documentation

## Search Strategy
1. Search for "integrate" in all files (case-insensitive)
2. Search for "integration" to find related functionality
3. Review all matches to determine if they're related to the para integrate command

## Important Notes
- Be careful not to remove unrelated uses of the word "integrate" or "integration"
- The `--integrate` flag on the finish command should also be removed
- Ensure all tests pass after removal

When complete, run: para finish "Remove para integrate command and simplify codebase"