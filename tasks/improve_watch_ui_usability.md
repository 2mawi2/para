# Improve Para Watch UI for Better Usability

## Current Issues with Watch UI
Based on the screenshot, the current UI has these problems:
1. Too many sessions displayed (21+) making it hard to find relevant ones
2. All showing "No description provided" - not helpful for identifying tasks
3. States are confusing (WORKING, AI REVIEW, HUMAN REVIEW) - should be simpler
4. No clear indication of which sessions are outdated/stale
5. Too verbose - needs to be more concise

## Design Requirements

### 1. Simplified States
Replace current states with more intuitive ones:
- **Active** (green) - Recent file changes in last 5 minutes
- **Idle** (yellow) - No activity for 5-30 minutes  
- **Ready** (blue) - Branch ready for review/integration
- **Stale** (gray) - No activity for >30 minutes

### 2. Concise Display Format
Each session should show:
```
[1] auth-api          ✓ Ready    15m ago   +248/-12
    "Implement JWT authentication endpoints"
    
[2] frontend-login    ⚡ Active   2m ago    +156/-0
    "Create login/signup React components"
```

### 3. Essential Information Only
- Session name (truncated if needed)
- Status icon and text
- Time since last activity
- Git diff stats (+lines/-lines)
- Task description (from dispatch)

### 4. Smart Filtering
- By default, hide stale sessions (>30 min inactive)
- Show only sessions from last 24 hours
- Press 's' to show all sessions
- Press 'h' to toggle help

### 5. Improved Navigation
- Number keys (1-9) to jump to session
- Enter to resume in IDE
- 'd' to mark as done/ready
- 'x' to hide session from view
- '/' to search/filter sessions

### 6. Integration with Dispatch
Modify dispatch command to:
1. Store task description in SessionState
2. Save to a `.para/sessions/{name}.task` file
3. Track in state file for watch to read

### 7. Activity Detection
Implement basic activity tracking:
- Check git status for uncommitted changes
- Monitor file modification times in worktree
- Track last commit time
- Detect if IDE process is still running

## Implementation Steps

1. **Update SessionState Structure**
   - Add `task_description: Option<String>`
   - Add `last_activity: DateTime<Utc>`
   - Add `files_changed: u32`
   - Add `lines_added: u32`
   - Add `lines_removed: u32`

2. **Modify Dispatch Command**
   - Save task description when creating session
   - Store in both state file and separate task file

3. **Enhance Watch UI**
   - Simplify the TUI layout
   - Implement filtering logic
   - Add activity detection
   - Improve visual hierarchy

4. **Add Activity Monitoring**
   - Periodic git status checks
   - File system monitoring
   - Process detection

## Key Code Changes

### In `src/core/session/state.rs`:
```rust
pub struct SessionState {
    // existing fields...
    pub task_description: Option<String>,
    pub last_activity: DateTime<Utc>,
    pub git_stats: GitStats,
}

pub struct GitStats {
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
}
```

### In `src/cli/commands/dispatch.rs`:
```rust
// After creating session, save task description
let task_file = state_dir.join(format!("{}.task", session_name));
fs::write(&task_file, &prompt)?;

// Update session state with task
session_state.task_description = Some(prompt.clone());
```

### In `src/cli/commands/watch.rs`:
- Read real sessions from SessionManager
- Load task descriptions from files
- Calculate activity based on git status
- Implement filtering and better display

## Testing
1. Dispatch multiple test sessions with descriptions
2. Verify watch shows real data
3. Test activity detection
4. Ensure IDE activation works
5. Test filtering and navigation

## Completion
When complete, run: para finish "Improve watch UI for better usability and real-time monitoring"