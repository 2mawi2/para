# Task: Automatic Container Cleanup

## Overview
Implement automatic cleanup of orphaned Docker containers when running common para commands. This ensures users don't accumulate stale containers from crashed sessions or system restarts.

## Problem Statement
- Container sessions that crash or are interrupted leave orphaned Docker containers
- These containers persist after system restarts
- Users must manually run cleanup commands
- No automatic cleanup mechanism exists

## Solution Design

### 1. Core Approach
- Add lightweight, time-based cleanup checks to frequently-used commands
- Run cleanup in background thread to avoid blocking
- Use marker file to prevent excessive cleanup runs
- Only clean containers without corresponding session state files

### 2. Implementation Details

#### A. Cleanup Trigger Logic
```rust
// Only run cleanup once per hour maximum
// Check .last_container_cleanup marker file
// Run in background thread if needed
```

#### B. Commands to Enhance
- `para list` - Most frequently run command
- `para start` - When creating new sessions
- `para status` - When checking session status
- `para finish` - When completing sessions

#### C. Cleanup Implementation
1. List all Docker containers with "para-" prefix
2. Check if corresponding session state exists
3. Remove orphaned containers silently
4. Update cleanup marker file

### 3. Testing Strategy

#### Unit Tests
- Test cleanup trigger logic with various marker file states
- Test container name parsing
- Test orphan detection logic

#### Integration Tests
- Create orphaned container, verify cleanup
- Verify cleanup doesn't affect active sessions
- Test cleanup marker file updates
- Test error handling when Docker unavailable

### 4. User Experience
- Completely transparent - no visible output unless errors
- No performance impact - runs in background
- Time-based throttling prevents excessive runs
- Manual `para clean --containers` still available

### 5. Safety Considerations
- Never remove containers with active sessions
- Fail silently if Docker unavailable
- Don't block main command execution
- Log errors but don't fail commands

### 6. Success Criteria
- [ ] Orphaned containers cleaned automatically
- [ ] No performance impact on commands
- [ ] Cleanup runs maximum once per hour
- [ ] All tests pass
- [ ] No impact on active sessions