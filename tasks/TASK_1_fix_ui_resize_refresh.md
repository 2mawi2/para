# Fix UI Resize Refresh Issue

## Problem
The monitor UI doesn't update when the terminal is resized. The UI only refreshes when a key is pressed or during periodic refresh intervals, causing visual artifacts when resizing.

## Solution
Handle the terminal resize event in the monitor command's event loop to trigger an immediate redraw.

## Implementation
1. In `src/cli/commands/monitor.rs`, modify the event handling to check for `Event::Resize`
2. When a resize event is detected, immediately redraw the terminal

## Code Changes
In the `run_app` method around line 60-70, update the event handling:

```rust
if event::poll(std::time::Duration::from_millis(100))? {
    match event::read()? {
        Event::Key(key) => {
            self.coordinator.handle_key(key).unwrap_or(());
            if self.coordinator.should_quit() {
                break;
            }
            terminal.draw(|f| self.coordinator.render(f))?;
        }
        Event::Resize(_, _) => {
            // Redraw immediately on resize
            terminal.draw(|f| self.coordinator.render(f))?;
        }
        _ => {}
    }
}
```

## Testing
1. Run `para monitor`
2. Resize the terminal window
3. Verify the UI updates immediately without visual artifacts
4. Ensure all existing functionality still works (keyboard navigation, refresh, etc.)

When done: para integrate "Fix monitor UI resize refresh issue"