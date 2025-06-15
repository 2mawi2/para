# Task: Debug MCP Recovery and Session Management Issues

## Overview
Investigate and fix issues with MCP para_recover calls failing/timing out and session cancellation problems. The `para recover` command through MCP is experiencing timeouts and failures, and some sessions like 'custom-branch-finish' cannot be cancelled properly.

## Specific Issues to Investigate

### 1. MCP para_recover Timeout/Failure
- `para recover custom-branch-finish` fails with "Session not found" despite session showing in `para list`
- MCP calls to para_recover are timing out or hanging
- Need to understand why recovery fails for sessions that appear to exist

### 2. Session Cancellation Issues  
- Sessions like 'custom-branch-finish' fail to cancel properly
- Session shows as existing in `para list` but recovery claims it doesn't exist
- Inconsistent session state between list and recovery operations

### 3. MCP Integration Problems
- MCP wrapper around para commands may have timeout issues
- Need to verify MCP tool implementations handle edge cases properly
- Check if session state detection is working correctly in MCP context

## Investigation Steps

### Phase 1: Reproduce and Document
**IMPORTANT**: Be extremely careful with para_recover calls - they WILL timeout and may block the session. Only call para_recover if absolutely necessary for debugging, and expect timeouts.

1. Use `para list` to examine current session states
2. Identify problematic sessions (dirty, missing, etc.)
3. Try basic para commands directly (not through MCP) to isolate issues
4. Document exact error messages and failure conditions

### Phase 2: Code Investigation
1. Review MCP tool implementations in `src/` 
2. Check session state management logic
3. Examine recovery logic in core para code
4. Look for race conditions or state inconsistencies

### Phase 3: Session State Analysis
1. Check `.para/state/` directory contents for problematic sessions
2. Verify worktree directories exist where expected
3. Look for orphaned state files or missing worktrees
4. Examine git branch states vs session records

### Phase 4: Fix Implementation
1. Fix session state detection issues
2. Improve error handling in recovery logic
3. Add better validation before attempting recovery
4. Enhance MCP tool error handling and timeouts

## Safety Warnings

⚠️ **CRITICAL**: Do NOT call `para_recover` through MCP unless absolutely necessary - it will likely timeout and block your session

⚠️ **Use direct CLI calls** for testing: `para recover session-name` instead of MCP tools when possible

⚠️ **Test incrementally** - don't make multiple recovery attempts that could compound issues

## Expected Deliverables

1. Root cause analysis of recovery failures
2. Fix for session cancellation issues  
3. Improved MCP error handling for recovery operations
4. Better session state validation
5. Documentation of proper recovery workflows
6. Tests to prevent regression of these issues

## Testing Requirements

1. Test recovery of sessions in different states (dirty, active, missing)
2. Verify cancellation works for problematic session types
3. Test MCP timeout handling improvements
4. Ensure session list consistency with actual state
5. Test edge cases like missing worktrees or orphaned state files

## Implementation Notes

- Focus on session state consistency as root cause
- Check if worktree paths vs session records are mismatched
- Look for platform-specific path issues (macOS vs Linux)
- Verify git operations work correctly for all session states
- Consider adding session validation/repair commands

When complete, run: para finish "Fix MCP recovery and session management issues"