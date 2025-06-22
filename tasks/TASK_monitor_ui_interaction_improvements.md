# Monitor UI Interaction Improvements

## Overview
The current monitor UI behavior is unintuitive - clicking anywhere on a row opens/resumes the session. We need to improve this by adding explicit action buttons and making row clicks only select the session.

## Requirements

### 1. Add Action Buttons
Add two small buttons to the left of the session name column:
- **Resume/Open button**: Opens or resumes the session (current click behavior)
- **Copy button**: Copies the session name/ID to clipboard with a copy icon

### 2. Change Row Click Behavior
- Normal click on a row should only select the session (highlight it)
- This simulates navigation to that session without opening it
- Remove the current behavior where clicking anywhere opens the session

### 3. Button Design
- Use icon buttons if the TUI framework supports it (preferred)
- Fall back to text buttons if icons aren't available (e.g., "[R]" for resume, "[C]" for copy)
- Buttons should be visually distinct but not overwhelming
- Consider using Unicode characters for icons if supported (e.g., ▶ for resume, 📋 for copy)

### 4. Maintain Keyboard Navigation
- All existing keyboard shortcuts must continue to work
- Arrow keys for navigation
- Enter to open/resume selected session
- Other existing shortcuts unchanged
- Tab navigation should include the new buttons

### 5. Implementation Notes
- The monitor uses the `ratatui` TUI framework
- Check `src/cli/commands/monitor.rs` for the current implementation
- The table rendering happens in the monitor command
- Consider using `ratatui::widgets::TableState` for selection tracking
- For clipboard functionality, use the `copypasta` crate or similar

### 6. Testing
- Test mouse interactions with the new buttons
- Verify keyboard navigation still works
- Test clipboard functionality on different platforms
- Ensure the UI remains responsive and intuitive

## Success Criteria
- Users can clearly see and use the action buttons
- Row selection is visually distinct from the action buttons
- Clicking a row selects it without opening the session
- All keyboard interactions work as before
- The UI feels more intuitive and provides better control

When done: para finish "Add action buttons to monitor UI for better interaction control" --branch feature/monitor-ui-buttons