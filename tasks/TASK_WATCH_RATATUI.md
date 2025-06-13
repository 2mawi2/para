# Task: Para Watch - Ratatui TUI Implementation

Create a terminal-based monitoring interface for Para development sessions using Ratatui.

## Core Functionality

Build a TUI application that displays active Para sessions in a clean, organized interface:

```
┌─ Para Watch ─────────────────────────────────────────────────────────────────┐
│ 💻 IDE Quick Switcher                                      [q]uit [r]efresh   │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│      WORKING ─────▶ AI REVIEW ─────▶ HUMAN REVIEW                            │
│        (4)            (2)              (1)                                    │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│ WORKING (4) ──────────────────────────────────────────────────────────────   │
│   [1] auth-flow      👤 alice     ✓    "OAuth2 authentication flow"          │
│   [2] payment-api    👤 bob       ✓    "Stripe payment integration"          │
│   [3] email-svc      👤 charlie   ✗    "Email notification service"          │
│   [4] search         👤 dave      ✓    "Elasticsearch integration"           │
│                                                                               │
│ AI REVIEW (2) ────────────────────────────────────────────────────────────   │
│   [5] ui-components  👤 eve       ⏱️ 15m   "React dashboard components"       │
│   [6] api-tests      👤 frank     ⏱️ 8m    "API integration test suite"       │
│                                                                               │
│ HUMAN REVIEW (1) ─────────────────────────────────────────────────────────   │
│   [7] backend-api                 📝      "REST API implementation"          │
│                                                                               │
│ Today: ✅ 12 Merged | ❌ 3 Cancelled | 🔄 7 Active                            │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Technical Requirements

### Dependencies
- `ratatui` for TUI framework
- `crossterm` for terminal backend
- Standard Rust libraries only

### Data Structure
```rust
struct SessionInfo {
    task_name: String,
    agent_name: String,
    description: String,
    state: SessionState,
    ide_open: bool,
    ai_review_minutes: Option<u16>,
}

enum SessionState {
    Working,
    AIReview,
    HumanReview,
}
```

### UI Layout
1. **Header**: Title with navigation hints
2. **Flow Diagram**: Visual state progression with counts
3. **Task Lists**: Grouped by state with numbered shortcuts
4. **Statistics**: Daily summary at bottom

### Keyboard Controls
- `1-9`: Print "Opening IDE for task: [task_name]"
- `q`: Quit application
- `r`: Print "Refreshing..." (mock refresh)
- Arrow keys: Navigate between tasks

### Visual Design
- **Colors**: Blue (Working), Yellow (AI Review), Magenta (Human Review)
- **Icons**: ✓/✗ for IDE status, ⏱️ for time, 📝 for human review
- **Layout**: Clean borders, proper spacing, responsive design

## Mock Data Requirements

Create 7-10 realistic test sessions with:
- Mix of states (4 working, 2 AI review, 1 human review)
- Various agent names and task descriptions
- Random IDE open/closed status
- AI review times (5-30 minutes)

## Deliverables

1. Single `src/watch.rs` file with complete implementation
2. Add necessary dependencies to `Cargo.toml`
3. Working TUI that matches the design exactly
4. All keyboard shortcuts functional (even if just printing)
5. Clean, readable code with proper error handling

## Success Criteria

- Application starts without errors
- UI matches the provided mockup design
- All keyboard controls work as specified
- Clean exit with 'q' key
- Proper color coding and layout

Focus on creating a polished, usable interface. Don't worry about actual Para integration - use mock data that represents realistic development scenarios.

When complete, run: para integrate "Add para watch TUI with Ratatui"