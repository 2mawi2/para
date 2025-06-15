# Task: Rename Watch to Monitor with Clear Purpose Separation

Rename `para watch` to `para monitor` and establish clear separation between the list and monitor commands.

## Rationale
- `para list` and `para watch` currently show similar information, causing user confusion
- Users need clarity on which command to use when orchestrating multiple agents
- "Monitor" better conveys the interactive, real-time nature of the command

## Implementation Plan

### 1. Rename Command
- Change `watch` to `monitor` throughout the codebase
- Update command registration in `src/cli/parser.rs`
- Rename `WatchArgs` to `MonitorArgs`
- Rename `src/cli/commands/watch.rs` to `monitor.rs`
- Update all imports and references

### 2. Update Command Descriptions

**para list** - Quick session snapshot (static)
```
/// List active sessions (quick snapshot for scripts and automation)
```

**para monitor** - Interactive session control center
```
/// Monitor and manage active sessions in real-time (interactive TUI)
```

### 3. Enhance List Output with Hint
When users run `para list`, add a helpful hint at the bottom:
```
$ para list
auth-api        ✓ active     2m ago    "Implement JWT auth"
payment-flow    ● dirty      12m ago   "Add Stripe integration"
ui-components   ✓ ready      -         "React dashboard"

Tip: Use 'para monitor' for interactive session management
```

### 4. Update Help Text
Make the distinction clear in help messages:

```
Commands:
  list      List active sessions (quick snapshot)
  monitor   Interactive session control center (manage, resume, finish)
```

### 5. Update Monitor Header
Change the TUI header to reflect the new name:
```
Para Monitor - Interactive Session Control                  Auto-refresh: 2s
────────────────────────────────────────────────────────────────────────────
```

### 6. File Changes Required

1. **Rename files:**
   - `src/cli/commands/watch.rs` → `src/cli/commands/monitor.rs`

2. **Update imports in `src/cli/commands/mod.rs`:**
   ```rust
   pub mod monitor;  // was: pub mod watch
   ```

3. **Update `src/cli/parser.rs`:**
   ```rust
   /// Monitor and manage active sessions in real-time (interactive TUI)
   Monitor(MonitorArgs),  // was: Watch(WatchArgs)
   
   #[derive(Args, Debug)]
   pub struct MonitorArgs {}  // was: WatchArgs
   ```

4. **Update command execution in main.rs or equivalent:**
   ```rust
   Commands::Monitor(args) => commands::monitor::execute(config, args),
   ```

5. **Update tests:**
   - Rename test functions from `test_watch_*` to `test_monitor_*`
   - Update command strings in tests

### 7. Documentation Updates
- Update README if it mentions the watch command
- Update any inline documentation
- Update CLAUDE.md if needed

## Testing
1. Verify `para monitor` launches the interactive TUI
2. Verify `para list` shows the hint about monitor
3. Ensure help text clearly distinguishes the commands
4. Run all existing tests
5. Test that old `para watch` shows helpful error suggesting `para monitor`

## Success Criteria
- Users immediately understand the difference between list and monitor
- No confusion about which command to use for orchestration
- Monitor command works exactly like watch did, just with better naming

When complete, run: para finish "Rename watch to monitor for clarity"