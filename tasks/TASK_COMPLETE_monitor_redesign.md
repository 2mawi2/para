# Task: Complete Monitor UI Redesign (Rename from Watch)

Implement a clean, user-focused terminal UI for `para monitor` (renamed from `para watch`) that provides instant overview and full control of active development sessions.

## Overview
1. Rename `watch` to `monitor` for clarity
2. Implement the redesigned UI with session management
3. Connect to real session data
4. Clear separation from `para list` command

## Part 1: Rename Watch to Monitor

### File Renames
- `src/cli/commands/watch.rs` → `src/cli/commands/monitor.rs`

### Code Updates

**In `src/cli/parser.rs`:**
```rust
/// Monitor and manage active sessions in real-time (interactive TUI)
Monitor(MonitorArgs),  // was: Watch(WatchArgs)

#[derive(Args, Debug)]
pub struct MonitorArgs {}  // was: WatchArgs
```

**In `src/cli/commands/mod.rs`:**
```rust
pub mod monitor;  // was: pub mod watch
```

**Update command execution:**
```rust
Commands::Monitor(args) => commands::monitor::execute(config, args),
```

## Part 2: Implement Redesigned UI

### Core Design
```
Para Monitor - Interactive Session Control                  Auto-refresh: 2s
────────────────────────────────────────────────────────────────────────────────

 #  Session         Status    Last Activity   Task
───────────────────────────────────────────────────────────────────────────────
[1] auth-api        🟢 Active    2m ago       "Implement JWT authentication"
[2] payment-flow    🟡 Idle      12m ago      "Add Stripe payment integration"  
[3] ui-components   ✅ Ready     -            "React dashboard components"

────────────────────────────────────────────────────────────────────────────────
auth-api • para/auth-api • [Enter] Resume • [f] Finish • [c] Cancel • [q] Quit
```

### Status Model
```rust
pub enum SessionStatus {
    Active,   // 🟢 Recent activity (< 5 min)
    Idle,     // 🟡 No activity (5-30 min)  
    Ready,    // ✅ Finished, ready for review
    Stale,    // ⏸️  No activity (> 30 min)
}
```

### Key Features

1. **Auto-refresh every 2 seconds** (no manual refresh)
2. **Smart filtering** - Hide stale sessions by default
3. **Full session management**:
   - Enter: Resume in IDE
   - f: Finish with commit message
   - c: Cancel (archive)
   - i: Integrate if ready
4. **Activity detection** based on file modifications
5. **Task descriptions** from dispatch

### Implementation Requirements

#### 1. Enhanced SessionState
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

#### 2. Update Dispatch to Save Task
```rust
// In src/cli/commands/dispatch.rs
// After creating session, save task description
let task_file = state_dir.join(format!("{}.task", session_name));
fs::write(&task_file, &prompt)?;

// Update session state
let mut session_state = session_manager.load_state(&session_name)?;
session_state.task_description = Some(prompt.clone());
session_manager.save_state(&session_state)?;
```

#### 3. Activity Detection
```rust
impl ActivityDetector {
    fn get_last_activity(&self, worktree_path: &Path) -> Result<DateTime<Utc>> {
        // Walk directory tree (excluding .git)
        // Find most recent file modification
        // Return the timestamp
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

#### 4. Interactive Actions
```rust
impl MonitorApp {
    fn finish_selected_session(&mut self) -> Result<()> {
        // Show inline prompt for commit message
        self.mode = Mode::FinishPrompt;
        self.input_buffer.clear();
        Ok(())
    }
    
    fn cancel_selected_session(&mut self) -> Result<()> {
        // Show confirmation
        self.mode = Mode::CancelConfirm;
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
}
```

## Part 3: Update List Command

Add hint to para list output:
```rust
// At the end of list command output
println!("\nTip: Use 'para monitor' for interactive session management");
```

## Part 4: Connect to Real Data

1. Load sessions from SessionManager
2. Read task descriptions from `.para/sessions/{name}.task` files
3. Calculate real git statistics
4. Implement actual file activity detection

## Testing Plan

1. Verify `para monitor` launches (not `para watch`)
2. Test all keyboard shortcuts work
3. Verify auto-refresh updates sessions
4. Test finish/cancel workflows
5. Ensure task descriptions display correctly
6. Verify filtering hides stale sessions

## Migration Notes

- The existing watch implementation in `para/watch-data-integration` branch can be used as base
- Update all "watch" references to "monitor"
- Preserve existing TUI structure, just enhance it

## Success Criteria

- Clear distinction between `para list` and `para monitor`
- Users can manage all sessions without leaving monitor
- Real-time updates show actual session activity
- Task descriptions provide instant context
- Clean, uncluttered interface

When complete, run: para finish "Implement para monitor with full UI redesign and session management"