# Task: Para Watch - Simple CLI Implementation

Create a command-line interface for monitoring Para sessions using basic terminal output.

## Goal

Implement `para watch` and `para status` commands that display active development sessions in a clean, readable format.

## Commands to Implement

### `para status`
Show current session information once:
```
Para Status - 7 active sessions

WORKING (4):
  auth-flow      [alice]     IDE:✓  OAuth2 authentication flow
  payment-api    [bob]       IDE:✓  Stripe payment integration  
  email-svc      [charlie]   IDE:✗  Email notification service
  search         [dave]      IDE:✓  Elasticsearch integration

AI REVIEW (2):
  ui-components  [eve]       15m    React dashboard components
  api-tests      [frank]     8m     API integration test suite

HUMAN REVIEW (1):
  backend-api                       REST API implementation

Today: 12 merged, 3 cancelled, 7 active (avg AI review: 12m)
```

### `para watch`
Continuously update the display every 2 seconds:
- Clear screen and redisplay status
- Show "Last updated: HH:MM:SS" timestamp
- Exit with Ctrl+C

## Technical Implementation

### Core Functions
```rust
// In src/cli/commands/status.rs
pub fn execute_status() -> Result<()> {
    let sessions = get_active_sessions()?;
    display_status(&sessions);
    Ok(())
}

// In src/cli/commands/watch.rs  
pub fn execute_watch() -> Result<()> {
    loop {
        clear_screen();
        execute_status()?;
        println!("Last updated: {}", current_time());
        thread::sleep(Duration::from_secs(2));
    }
}
```

### Data Structure
```rust
struct SessionStatus {
    name: String,
    agent: Option<String>,
    state: SessionState,
    ide_open: bool,
    ai_review_duration: Option<Duration>,
    description: String,
}

enum SessionState {
    Working,
    AIReview,
    HumanReview,
}
```

### File Structure
- Add `status.rs` to `src/cli/commands/`
- Add `watch.rs` to `src/cli/commands/`
- Update `src/cli/commands/mod.rs` to include new commands
- Update `src/cli/mod.rs` to register commands in clap

## Mock Implementation

For now, create realistic mock data that simulates:
- Reading from `.para/sessions/` directory
- Checking IDE process status (mock with random true/false)
- Calculating time differences for AI review duration
- Reading session metadata from JSON files

## Requirements

1. **Clean Output**: Well-formatted, easy to scan
2. **Color Coding**: Different colors for each state (if terminal supports it)
3. **Error Handling**: Graceful handling of missing sessions or data
4. **Performance**: Fast execution, minimal resource usage
5. **Keyboard Friendly**: Ctrl+C to exit watch mode

## Success Criteria

- `para status` shows all active sessions in organized format
- `para watch` updates display every 2 seconds
- Both commands handle empty session lists gracefully
- Clean integration with existing Para CLI structure
- Proper error messages for common failure cases

Keep it simple and fast. Focus on clear information display rather than fancy visuals.

When complete, run: para integrate "Add para status and watch commands"