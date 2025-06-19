# Investigate Active Session State Issue

## Problem
The first session "cleanup-other" is showing as "Active" state while other sessions show "Review" or "Idle" states. This appears inconsistent and needs investigation.

## Task
1. Examine the session state management code to understand how states are determined
2. Check the session state files in `.para/state/` directory 
3. Look at the monitor/status display logic to see if there's a bug in state reporting
4. Identify why "cleanup-other" session is marked as Active when others are Review/Idle
5. Determine if this is a legitimate state or a bug in state tracking
6. If it's a bug, fix the state determination logic
7. Test the fix to ensure proper state reporting

## Investigation Areas
- Session state persistence and loading
- State transition logic 
- Monitor display formatting
- Worktree status detection
- Session lifecycle management

## Expected Outcome
- Clear understanding of why the inconsistent state occurs
- Fix for proper state reporting if it's a bug
- All sessions should show correct states based on their actual status

When done: para finish "Fix session state reporting inconsistency"