# Task: Complete Redesign of Para Watch UI

Implement a clean, user-focused terminal UI for `para watch` that provides instant overview of active development sessions.

## Core Design Principles
1. **Clarity over complexity** - Show only what matters
2. **Action-oriented** - Make it easy to resume work
3. **Smart defaults** - Hide noise, surface active work
4. **Real-time accuracy** - Show actual session data

## Implementation Requirements

### 1. New Status Model
Replace complex states with simple, intuitive indicators:
```rust
pub enum SessionStatus {
    Active,   // 🟢 Recent activity (< 5 min)
    Idle,     // 🟡 No activity (5-30 min)  
    Ready,    // ✅ Finished, ready for review
    Stale,    // ⏸️  No activity (> 30 min)
}
```

### 2. Compact Display Format with Auto-Refresh
```
Para Watch - 3 active • 1 ready • 5 total               Auto-refresh: 2s  [?] Help
────────────────────────────────────────────────────────────────────────────────

 #  Session         Status    Last Activity   Task
───────────────────────────────────────────────────────────────────────────────
[1] auth-api        🟢 Active    2m ago       "Implement JWT authentication"
[2] payment-flow    🟡 Idle      12m ago      "Add Stripe payment integration"  
[3] ui-components   ✅ Ready     -            "React dashboard components"

────────────────────────────────────────────────────────────────────────────────
auth-api • para/auth-api • [Enter] Resume • [f] Finish • [c] Cancel • [q] Quit
```

### Help Overlay
When user presses '?' or 'h':
```
┌─ Para Watch Help ─────────────────────────────────────────────┐
│                                                               │
│ Navigation:                                                   │
│   ↑/↓ or j/k  - Move selection                              │
│   1-9         - Jump to session by number                    │
│                                                               │
│ Actions:                                                      │
│   Enter       - Resume session in IDE                        │
│   f           - Finish session (mark as ready)               │
│   c           - Cancel session (archive)                     │
│   i           - Integrate ready session                      │
│                                                               │
│ View:                                                         │
│   s           - Show/hide stale sessions                     │
│   /           - Filter sessions                              │
│                                                               │
│ Press any key to close help                                  │
└───────────────────────────────────────────────────────────────┘
```

### 3. Activity Detection System
```rust
// In src/cli/commands/watch.rs
impl ActivityDetector {
    fn get_last_activity(&self, worktree_path: &Path) -> Result<DateTime<Utc>> {
        // Walk directory tree (excluding .git)
        // Find most recent file modification
        // Return the timestamp
    }
    
    fn get_git_stats(&self, worktree_path: &Path) -> Result<GitStats> {
        // Run git diff --stat
        // Parse output for files changed, +lines, -lines
        // Return stats
    }
    
    fn detect_status(&self, session: &SessionState) -> SessionStatus {
        let last_activity = self.get_last_activity(&session.worktree_path)?;
        let elapsed = Utc::now() - last_activity;
        
        match elapsed.num_minutes() {
            0..=5 => SessionStatus::Active,
            6..=30 => SessionStatus::Idle,
            _ => SessionStatus::Stale,
        }
    }
}
```

### 4. Task Description Storage
Modify dispatch to save descriptions:
```rust
// In src/cli/commands/dispatch.rs
pub fn execute(config: Config, args: DispatchArgs) -> Result<()> {
    // ... existing code ...
    
    // Save task description
    let task_file = state_dir.join(format!("{}.task", session_name));
    fs::write(&task_file, &prompt)?;
    
    // Update session state
    let mut session_state = session_manager.load_state(&session_name)?;
    session_state.task_description = Some(prompt.clone());
    session_manager.save_state(&session_state)?;
    
    // ... rest of dispatch ...
}
```

### 5. Enhanced SessionState
```rust
// In src/core/session/state.rs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    pub name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
    
    // New fields
    pub task_description: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
    pub git_stats: Option<GitStats>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitStats {
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
}
```

### 6. Smart Filtering
```rust
impl WatchApp {
    fn filter_sessions(&self, sessions: Vec<SessionInfo>) -> Vec<SessionInfo> {
        sessions.into_iter()
            .filter(|s| {
                if self.show_all {
                    true
                } else {
                    // Hide stale sessions by default
                    !matches!(s.status, SessionStatus::Stale)
                }
            })
            .take(10) // Max 10 sessions visible
            .collect()
    }
}
```

### 7. Complete Session Management with Auto-Refresh
```rust
// Auto-refresh timer
impl WatchApp {
    fn new() -> Self {
        Self {
            sessions: vec![],
            selected_index: 0,
            mode: Mode::Normal,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2),
            // ... other fields
        }
    }
    
    fn tick(&mut self) -> Result<()> {
        // Auto-refresh every 2 seconds
        if self.last_refresh.elapsed() >= self.refresh_interval {
            self.refresh_sessions()?;
            self.last_refresh = Instant::now();
        }
        Ok(())
    }
}

// Keyboard handling (no manual refresh needed)
match key.code {
    KeyCode::Enter => self.resume_selected_session()?,
    KeyCode::Char('f') => self.finish_selected_session()?,
    KeyCode::Char('c') => self.cancel_selected_session()?,
    KeyCode::Char('i') => self.integrate_if_ready()?,
    KeyCode::Char('1'..='9') => self.jump_to_session(n)?,
    KeyCode::Char('s') => self.toggle_show_all(),
    KeyCode::Char('?') | KeyCode::Char('h') => self.show_help(),
    KeyCode::Char('q') => self.should_quit = true,
    KeyCode::Up => self.move_selection_up(),
    KeyCode::Down => self.move_selection_down(),
    _ => {}
}
```

### Session Management Actions

```rust
impl WatchApp {
    fn finish_selected_session(&mut self) -> Result<()> {
        let session = &self.sessions[self.selected_index];
        
        // Show inline prompt for commit message
        self.mode = Mode::FinishPrompt;
        self.input_buffer.clear();
        
        // In render, show: "Finish 'auth-api'? Enter commit message: _"
        Ok(())
    }
    
    fn cancel_selected_session(&mut self) -> Result<()> {
        let session = &self.sessions[self.selected_index];
        
        // Show confirmation
        self.mode = Mode::CancelConfirm;
        
        // In render, show: "Cancel 'auth-api'? This will archive the session. [y/N]"
        Ok(())
    }
    
    fn execute_finish(&mut self, message: &str) -> Result<()> {
        let session = &self.sessions[self.selected_index];
        
        // Run para finish in background
        Command::new("para")
            .args(["finish", message])
            .current_dir(&session.worktree_path)
            .spawn()?;
            
        // Update session status locally
        self.sessions[self.selected_index].status = SessionStatus::Ready;
        self.mode = Mode::Normal;
        Ok(())
    }
    
    fn execute_cancel(&mut self) -> Result<()> {
        let session = &self.sessions[self.selected_index];
        
        // Run para cancel
        Command::new("para")
            .args(["cancel", &session.name])
            .output()?;
            
        // Remove from list
        self.sessions.remove(self.selected_index);
        self.selected_index = self.selected_index.saturating_sub(1);
        self.mode = Mode::Normal;
        Ok(())
    }
}
```

### 8. Resume Integration
```rust
fn resume_selected_session(&self) -> Result<()> {
    let session = &self.sessions[self.selected_index];
    
    // Execute para resume command
    Command::new("para")
        .args(["resume", &session.name])
        .spawn()?;
        
    // Exit watch UI after launching
    self.should_quit = true;
    Ok(())
}
```

## Testing Plan

1. **Mock Data Testing**
   - Create 10+ test sessions with varying states
   - Verify filtering works correctly
   - Test all keyboard shortcuts

2. **Real Data Integration**
   - Connect to actual SessionManager
   - Load real task descriptions
   - Verify activity detection works

3. **Usability Testing**
   - Ensure UI fits in 80x24 terminal
   - Test on different terminal emulators
   - Verify color/emoji rendering

## Migration Steps

1. First implement with mock data
2. Add activity detection
3. Connect to real SessionManager
4. Add task description storage in dispatch
5. Full integration testing

### 9. Interactive Prompts

**Finish Flow:**
```
────────────────────────────────────────────────────────────────────────────────
Finish 'auth-api'? Enter commit message (ESC to cancel):
> Implement JWT authentication with refresh tokens_
────────────────────────────────────────────────────────────────────────────────
```

**Cancel Flow:**
```
────────────────────────────────────────────────────────────────────────────────
Cancel 'old-feature'? This will archive the session. [y/N]: _
────────────────────────────────────────────────────────────────────────────────
```

**Success Feedback:**
```
✓ Session 'auth-api' marked as ready for review
✓ Session 'old-feature' archived successfully
```

## UI State Management

```rust
enum Mode {
    Normal,
    FinishPrompt,
    CancelConfirm,
    FilterInput,
    Help,
}

struct WatchApp {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    mode: Mode,
    input_buffer: String,
    show_all: bool,
    filter: Option<String>,
}
```

## Complete Workflow Example

1. **User launches para watch**
   - Sees clean list of active sessions
   - Stale sessions hidden by default

2. **Managing active work:**
   - Press `2` to select payment-flow
   - Press `Enter` to resume in IDE
   - Or press `f` to finish with commit message
   - Watch auto-refreshes every 2 seconds

3. **Cleaning up:**
   - Navigate to old session
   - Press `c` to cancel
   - Confirm with `y`
   - Session removed from view

4. **Quick actions:**
   - See a ready session? Press `i` to integrate
   - Need to see all? Press `s` to toggle
   - Lost? Press `?` for help

## Success Metrics

- User can identify active work in < 2 seconds
- Complete session management without leaving watch
- Resume, finish, or cancel with 2-3 keypresses
- No visual clutter from old sessions
- Task descriptions provide instant context
- Works smoothly with 20+ sessions

When complete, run: para finish "Complete redesign of watch UI with full session management"