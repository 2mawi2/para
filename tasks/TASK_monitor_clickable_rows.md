# Add clickable rows to para monitor UI

## Goal
Investigate if the current terminal UI framework supports clickable rows in `para monitor`, and implement clicking functionality where each row click opens the corresponding session.

## Requirements
1. Research the current TUI framework used by `para monitor` (likely ratatui or similar)
2. Check if the framework supports mouse events and click detection
3. If supported, implement:
   - Mouse event handling for the session table
   - Click detection on individual rows
   - When a row is clicked, trigger the same action as pressing Enter (open the session)
4. If not natively supported, investigate alternatives:
   - Alternative TUI frameworks that support mouse events
   - Workarounds within the current framework
   - Document limitations if implementation isn't feasible

## Implementation Notes
- Maintain keyboard navigation as the primary interaction method
- Mouse support should be an enhancement, not a replacement
- Consider terminal compatibility - not all terminals support mouse events
- The click behavior should mirror the current Enter key behavior exactly

## Testing
- Test mouse clicks on different rows
- Verify keyboard navigation still works
- Test in different terminal emulators (Terminal.app, iTerm2, etc.)
- Ensure graceful degradation if terminal doesn't support mouse events

When done: para finish "Add clickable rows to para monitor UI"