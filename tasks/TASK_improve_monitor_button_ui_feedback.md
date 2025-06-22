# Improve Monitor Button UI and User Feedback

## Problem
The current action buttons in the monitor UI have two issues:
1. The buttons (▶ and 📋) don't look like clickable buttons - they appear as plain text/icons
2. There's no visual feedback when clicking them, leaving users uncertain if their action was registered

## Requirements

### 1. Make Buttons Look Clickable
- Add visual borders or backgrounds to make buttons appear interactive
- Consider using box-drawing characters to create button borders: `[▶]` or `│▶│`
- Alternatively, use background colors to create button appearance
- Ensure selected and unselected states are visually distinct
- Buttons should look "pressable" and invite interaction

### 2. Add Click Feedback
- Implement visual feedback when a button is clicked
- Options to explore with ratatui:
  - Brief highlight/flash effect on click
  - Temporary status message at bottom of screen
  - Small popup/toast notification (if supported by ratatui)
  - Change button appearance momentarily (e.g., inverted colors)

### 3. Specific Feedback Messages
- Resume button: "Opening session: [session-name]"
- Copy button: "Copied: [session-name]" or "Session name copied to clipboard"
- Error cases: Show appropriate error messages if action fails

### 4. Implementation Considerations
- Check ratatui's capabilities for:
  - Popup/overlay widgets
  - Temporary UI elements
  - Animation or timed state changes
- If popups aren't available, use the status line or a dedicated message area
- Feedback should be brief (1-2 seconds) and non-intrusive
- Maintain responsiveness - don't block UI during feedback display

### 5. Visual Design Guidelines
- Use consistent styling with the rest of the monitor UI
- Ensure accessibility - buttons should be distinguishable without color
- Test appearance in different terminal themes
- Keep the compact table layout - don't make buttons too large

## Technical Notes
- The current implementation is in `src/ui/monitor/renderer.rs` (create_action_buttons_cell)
- Mouse handling is in `src/ui/monitor/coordinator.rs`
- Consider adding a message/notification system to MonitorAppState
- May need to implement a timer system for temporary feedback

## Success Criteria
- Buttons visually appear as interactive elements, not plain text
- Users receive immediate, clear feedback when clicking buttons
- Feedback messages are helpful and informative
- The UI remains responsive and clean
- Implementation works across different terminal emulators

When done: para finish "Improve monitor button UI with visual design and click feedback" --branch feature/monitor-button-feedback