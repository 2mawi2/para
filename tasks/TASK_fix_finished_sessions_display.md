# Fix Display of Finished Sessions in Para List Command

## Problem Analysis

The `para list` command is not properly displaying finished sessions. The root cause is a conceptual mismatch between two different types of "archived" sessions:

1. **State-based archives**: Sessions with `SessionStatus::Finished` or `SessionStatus::Cancelled` in their state files
2. **Branch-based archives**: Branches moved to the archive naming pattern (`para/archived/...`)

Currently:
- `para list` (without flags) shows active sessions but marks Finished/Cancelled sessions as "archived" 
- `para list --archived` ONLY shows branches with the archive naming pattern
- Sessions that are finished but haven't had their branches archived are not visible with `--archived`

## Solution

Modify the `list_archived_sessions` function in `src/cli/commands/list.rs` to:

1. First get all sessions from state files (like `list_active_sessions` does)
2. Filter for sessions with `SessionStatus::Finished` or `SessionStatus::Cancelled`
3. Also include branches with the archived naming pattern (existing logic)
4. Merge and deduplicate the results

This ensures that:
- All finished sessions are shown with `para list --archived` regardless of branch status
- The display is consistent with user expectations
- No finished sessions are hidden from view

## Implementation Steps

1. Refactor `list_archived_sessions` to check session state files for Finished/Cancelled status
2. Keep the existing branch-based archive detection as a fallback
3. Ensure proper deduplication if a session appears in both sources
4. Add appropriate test coverage for the new behavior

## Testing

After implementation:
1. Create a session and finish it with `para finish`
2. Verify it appears in `para list --archived`
3. Archive the branch and verify it still appears (no duplicates)
4. Test with sessions that have been cancelled
5. Run all existing tests to ensure no regressions

When done: para integrate "Fix display of finished sessions in list command"