# Task: Para Watch UI Improvements

Enhance the Para Watch TUI with better visual elements, navigation, and detail views.

## UI Improvements Required

### 1. Replace Emojis with ASCII Art
- **Remove all emojis**: No emojis anywhere in the interface
- **ASCII progress animations**: For working tasks, show rotating ASCII progress indicators
- **ASCII state indicators**: Use text-based symbols instead of emojis
- **ASCII workflow icons**: Replace emoji pipeline with clean ASCII symbols

### 2. Number Selection
- **Numeric shortcuts**: Allow pressing 1-9 to select tasks directly
- **Visual indicators**: Show numbers clearly in the table
- **Keyboard feedback**: Highlight when number keys are pressed

### 3. Loading Animation
- **Spinner for refreshing**: Show ASCII spinner when refreshing data
- **Progress indicators**: Rotating ASCII chars for working tasks (|/-\)
- **Smooth animations**: Update progress indicators every 500ms

### 4. Remove Status Column
- **Simplify table**: Remove the Status column as requested
- **Keep columns**: #, Task, Agent, State, Description
- **Adjust widths**: Redistribute column space after Status removal

### 5. Detail View Implementation
- **Enter key**: Press Enter on any task to show detail view
- **Full screen overlay**: Show detailed session information
- **Navigation**: ESC to go back to main table
- **Timeline view**: Show session progress timeline
- **Action buttons**: [Enter] Open IDE, [l] View logs, [g] Git status

### 6. Enhanced Workflow Pipeline
- **Better visual flow**: Improve the pipeline section appearance
- **ASCII arrows**: Use clean ASCII arrows instead of unicode
- **Section highlighting**: Better highlight of current section
- **Progress flow**: Show task movement through pipeline

## Detail View Layout

```
┌─ Para Watch - Session Details ───────────────────────────────────────────────┐
│ [Esc] Back                                         Session: ui-components     │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│ Task: Create responsive React components for user dashboard                   │
│ Agent: eve                                                                    │
│ Branch: para/ui-components-20240115                                           │
│ Current State: AI REVIEW (Attempt 1/2)                                        │
│                                                                               │
│ Timeline:                                                                     │
│ ─────────────────────────────────────────────────────────────────────────    │
│ 10:15 AM  Started working                                                    │
│ 11:45 AM  Submitted for AI review                                           │
│ 11:48 AM  AI Review started                                                 │
│           Currently reviewing... (15 minutes elapsed)                         │
│                                                                               │
│ AI Review Checklist:                                                         │
│ • Code quality and best practices                                            │
│ • Test coverage analysis                                                     │
│ • Component documentation                                                    │
│ • Accessibility compliance                                                   │
│                                                                               │
│ [Enter] Open IDE  [l] View logs  [g] Git status                              │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Requirements

### ASCII Progress Indicators
```rust
// Rotating progress for working tasks
const PROGRESS_CHARS: [char; 4] = ['|', '/', '-', '\\'];

// State indicators without emojis
fn get_state_indicator(state: &SessionState) -> &'static str {
    match state {
        SessionState::Working => "[W]",
        SessionState::AIReview => "[A]", 
        SessionState::HumanReview => "[H]",
    }
}
```

### Detail View State Management
```rust
pub struct App {
    // ... existing fields
    view_mode: ViewMode,
    detail_session_index: Option<usize>,
}

enum ViewMode {
    MainTable,
    SessionDetail,
}
```

### Animation Timer
- Use `std::time::Instant` for animation timing
- Update progress indicators every 500ms
- Rotate through progress characters for working tasks

## Key Features to Implement

1. **Emoji Removal**: Clean ASCII-only interface
2. **Number Selection**: Direct task selection with 1-9 keys
3. **Loading Animation**: Visual feedback during operations
4. **Column Simplification**: Remove Status column
5. **Detail View**: Full session information overlay
6. **ASCII Animations**: Progress indicators for working tasks
7. **Better Navigation**: Enhanced keyboard shortcuts

## Testing Requirements

- Test all keyboard shortcuts (1-9, Enter, ESC, arrows, Tab)
- Verify ASCII animations work smoothly
- Test detail view navigation and display
- Ensure no emojis remain in the interface
- Test column layout after Status removal

Focus purely on UI improvements. Do not modify data loading or integration logic.

When complete, run: para finish "Enhance Para Watch UI with ASCII art, detail view, and improved navigation"