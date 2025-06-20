# Fix allowed_tools Parameter in claude-code-action-extended

## Problem

The `allowed_tools` parameter is not working correctly in the extended Claude action. When workflows run, Claude gets "Claude requested permissions to use Bash" errors even though Bash is explicitly listed in allowed_tools.

## Current Behavior

1. Workflow sets: `allowed_tools: "Bash,Edit,Read,Write,MultiEdit,Glob,Grep,LS,TodoRead,TodoWrite"`
2. Action logs show it's setting ALLOWED_TOOLS environment variable
3. Claude still gets permission errors when trying to use Bash

## Investigation Results

From the Auditor workflow logs, we can see Claude is being invoked but cannot execute any Bash commands. This happens with multiple formats:
- Single line comma-separated: `"Bash,Edit,Read"`
- Multi-line format: `Bash\nEdit\nRead`
- Specific commands: `Bash(gh issue create)`

## Root Cause

The extended action is setting `ALLOWED_TOOLS` as an environment variable, but Claude Code likely expects the permissions to be passed differently - possibly as command-line arguments or in a configuration file.

## Fix Required

1. Check how the original grll/claude-code-action passes allowed_tools to Claude Code
2. Update the extended action to use the same mechanism
3. Test that Claude can actually use the allowed tools

## Code to Check

In the extended action's implementation:
- How is `allowed_tools` being processed?
- How is Claude Code being invoked?
- Are the tools being passed as CLI arguments like `--allowed-tools`?

## Expected Behavior

When `allowed_tools` includes "Bash", Claude should be able to execute Bash commands without permission errors.

## Test Case

After fixing, the Auditor workflow should be able to:
1. Use `gh issue create` to create GitHub issues
2. Use `grep`, `rg`, `find` to search the codebase
3. Use `just test` to run tests

When done: para finish "Fix allowed_tools parameter passing in extended action"