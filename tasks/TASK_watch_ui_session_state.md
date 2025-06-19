# Investigate Active Session State in Para Watch UI

## Problem
In the `para watch` UI feature, the first session "cleanup-other" is showing as "Active" state while other sessions show "Review" or "Idle" states. This appears inconsistent and needs investigation specifically within the watch UI implementation.

## Task
1. Examine the `para watch` UI implementation to understand how session states are displayed
2. Check how the watch UI retrieves and renders session state information
3. Look at the UI state management and rendering logic in the watch feature
4. Identify why "cleanup-other" session is marked as Active in the watch UI when others show Review/Idle
5. Determine if this is a UI rendering issue or an underlying state tracking problem
6. If it's a bug, fix the state display logic in the watch UI
7. Test the fix to ensure proper state reporting in the watch interface

## Investigation Areas
- `para watch` command implementation in CLI
- UI rendering and state display logic
- Session state retrieval for the watch interface
- Real-time update mechanisms in the watch UI
- Terminal UI state formatting and display

## Expected Outcome
- Clear understanding of why the inconsistent state occurs in the watch UI
- Fix for proper state display in the watch interface
- All sessions should show correct states in the watch UI based on their actual status
- Consistent and accurate real-time state updates in the watch feature

When done: para finish "Fix session state display in para watch UI"