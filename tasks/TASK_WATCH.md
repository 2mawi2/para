Perfect! Here's the final design combining the visual flow header with the enhanced compact view:

```
┌─ Para Watch ─────────────────────────────────────────────────────────────────┐
│ 💻 IDE Quick Switcher                                      [q]uit [r]efresh   │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│      WORKING ─────▶ AI REVIEW ─────▶ HUMAN REVIEW                            │
│        (4)            (2)              (1)                                    │
│                         ↓                                                     │
│                    (retry once)                                               │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│ WORKING (4) ──────────────────────────────────────────────────────────────   │
│                                                                               │
│   [1] auth-flow      👤 alice     ✓    "OAuth2 authentication flow"          │
│   [2] payment-api    👤 bob       ✓    "Stripe payment integration"     ←    │
│   [3] email-svc      👤 charlie   ✗    "Email notification service"          │
│   [4] search         👤 dave      ✓    "Elasticsearch integration"           │
│                                                                               │
│ AI REVIEW (2) ────────────────────────────────────────────────────────────   │
│                                                                               │
│   [5] ui-components  👤 eve       ⏱️ 15m   "React dashboard components"       │
│   [6] api-tests      👤 frank     ⏱️ 8m    "API integration test suite"       │
│                                                                               │
│ HUMAN REVIEW (1) ─────────────────────────────────────────────────────────   │
│                                                                               │
│   [7] backend-api                 📝      "REST API implementation"          │
│                                                                               │
│ ─────────────────────────────────────────────────────────────────────────    │
│ Today: ✅ 12 Merged (avg: 12m AI review) | ❌ 3 Cancelled | 🔄 7 Active       │
│                                                                               │
│ Press [1-7] to open IDE | ✓ = IDE open | ⏱️ = Time in AI review              │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Focused Session View (Tab to expand)

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

## Key Features of Final Design:

1. **Visual Flow Header**: Shows the state progression clearly
2. **Compact Task List**: Maintained the efficient layout
3. **No Human Reviewer**: Human review tasks don't show agent names
4. **AI Review Time Only**: Only tracks time during AI review phase
5. **Clean Statistics**: Shows average AI review time, not total development time
6. **Numbered Shortcuts**: Quick IDE access remains the primary action

## Implementation Notes:

```rust
// Display logic for tasks
impl SessionInfo {
    pub fn display_line(&self, index: usize) -> String {
        let ide_indicator = if self.ide_open { "✓" } else { "✗" };
        
        match &self.state {
            SessionState::Working => {
                format!("[{}] {:<12} 👤 {:<8} {}    \"{}\"",
                    index + 1,
                    self.task_name,
                    self.agent_name,
                    ide_indicator,
                    truncate(&self.description, 40)
                )
            }
            SessionState::AIReview { .. } => {
                let time = self.time_tracking.current_ai_review_duration()
                    .map(|d| format!("⏱️ {}m", d.num_minutes()))
                    .unwrap_or_else(|| "⏱️ <1m".to_string());
                    
                format!("[{}] {:<12} 👤 {:<8} {:<8} \"{}\"",
                    index + 1,
                    self.task_name,
                    self.agent_name,
                    time,
                    truncate(&self.description, 40)
                )
            }
            SessionState::HumanReview => {
                // No agent name for human review
                format!("[{}] {:<12}              📝      \"{}\"",
                    index + 1,
                    self.task_name,
                    truncate(&self.description, 40)
                )
            }
            _ => String::new()
        }
    }
}
```


# Task: Implement Para Watch UI Prototype

Create a terminal UI for monitoring Para development sessions using Ratatui.

## Requirements

1. **Main View** - Display active Para sessions grouped by state:
   - WORKING - Sessions with agents actively developing
   - AI REVIEW - Sessions being reviewed by AI
   - HUMAN REVIEW - Sessions awaiting human review
   
2. **Visual Flow Header** - Show state progression at the top:
   ```
   WORKING ─────▶ AI REVIEW ─────▶ HUMAN REVIEW
     (4)            (2)              (1)
   ```

3. **Task List** - For each state section, show:
   - Number shortcut [1-9] for quick access
   - Task name
   - Agent name (except for HUMAN REVIEW)
   - IDE status indicator (✓ or ✗)
   - Time in AI review (format: ⏱️ 15m)
   - Task description (truncated to 40 chars)

4. **Statistics Bar** - At the bottom show:
   - Tasks merged today
   - Tasks cancelled today
   - Active task count
   - Average AI review time

5. **Keyboard Navigation**:
   - Number keys (1-9): Print "Opening IDE for task: [task_name]"
   - 'q': Quit application
   - 'r': Refresh (just print "Refreshing..." for now)
   - Arrow keys: Highlight different tasks

## Implementation

Use Ratatui with crossterm backend. Create a simple TUI application with:
- Mock data for 7-10 tasks in different states
- Clean layout using Ratatui's Layout and Block widgets
- Colored state headers
- Smooth keyboard interaction

## Mock Data Structure

```rust
struct SessionInfo {
    task_name: String,
    agent_name: String,
    description: String,
    state: SessionState,
    ide_open: bool,
    ai_review_minutes: Option<u16>, // Only for AI REVIEW state
}

enum SessionState {
    Working,
    AIReview,
    HumanReview,
}
```

## Deliverables

1. Single `watch.rs` file that compiles and runs
2. Clean, readable UI matching the design
3. All keyboard shortcuts working (even if they just print actions)
4. Use colors: Blue for WORKING, Yellow for AI REVIEW, Magenta for HUMAN REVIEW

## Example Output

The UI should look like the provided design mockup. Focus on making it visually clean and easy to navigate. Don't worry about actual Para integration - just make the UI work with mock data.

Keep it simple - no complex state management or external dependencies beyond Ratatui and crossterm.

When complete, run: para integrate "Add para watch UI prototype"



