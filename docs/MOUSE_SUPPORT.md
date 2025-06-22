# Mouse Support in Para Monitor

## Overview
The para monitor UI now supports mouse interactions for improved navigation and usability.

## Features
- **Click to Select**: Click on any session row to select it
- **Click to Open**: Clicking on a row will immediately open that session (same as pressing Enter)
- **Mouse events are ignored in dialog modes**: When in finish prompt, cancel confirm, or error dialogs, mouse clicks are ignored to prevent accidental actions

## Implementation Details
- Uses ratatui's built-in mouse support via crossterm
- Mouse capture is enabled automatically when monitor starts
- The table starts at y=4 (accounting for header and borders)
- Click detection properly maps mouse coordinates to table rows

## Terminal Compatibility
Mouse support requires a terminal that supports mouse events:
- ✅ Terminal.app (macOS)
- ✅ iTerm2
- ✅ Alacritty
- ✅ VS Code integrated terminal
- ✅ Most modern terminal emulators

## Graceful Degradation
- Keyboard navigation remains the primary interaction method
- If the terminal doesn't support mouse events, the UI continues to work normally with keyboard only
- No error messages or warnings are shown for terminals without mouse support

## Technical Notes
- Mouse events are handled in `MonitorCoordinator::handle_mouse()`
- Only left mouse button clicks are processed
- Row calculation accounts for the UI layout (header, borders, etc.)
- Selection is updated before triggering the resume action