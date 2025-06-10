# TASK 20: Simplify Unified Session Management

## Overview

The unified session management system (commit `92222bdc0f29c8df2745b2921bc1344987c461f3`) introduces good architectural principles but is over-engineered with unnecessary metadata complexity. This task simplifies the implementation while preserving the unified architecture benefits.

## Prerequisites

**IMPORTANT**: First checkout the specific commit to work from:
```bash
git checkout 92222bdc0f29c8df2745b2921bc1344987c461f3
git checkout -b simplified-session-management
```

## Goals

Maintain the unified architecture benefits:
- ✅ Single `SessionManager` used by all commands
- ✅ Consistent error handling across commands  
- ✅ Elimination of code duplication
- ✅ Centralized validation logic

Remove unnecessary complexity:
- ❌ Session types (Manual/Dispatched/Recovered) - not needed
- ❌ Configuration snapshots - config rarely changes
- ❌ Commit tracking metadata - Git already handles this
- ❌ Repository root storage - can be discovered when needed
- ❌ Complex state file format versioning - overkill
- ❌ Migration system - premature optimization

## Simplified State Model

Replace the complex `SessionState` in `para-rs/src/core/session/state.rs` with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Finished,
    Cancelled,
}
```

That's it. No more metadata bloat.

## Files to Simplify

### 1. Remove Unnecessary Files
- Delete `para-rs/src/core/session/migration.rs` entirely
- Delete `para-rs/src/core/session/validation.rs` entirely  
- Remove complex validation logic - basic file existence checks are sufficient

### 2. Simplify `para-rs/src/core/session/state.rs`
- Remove `SessionType`, `SessionConfig`, `SessionSummary`, `StateFileFormat`
- Keep only the simplified `SessionState` and `SessionStatus` above
- Remove the three constructor methods (`new_manual`, `new_dispatched`, `new_recovered`)
- Replace with single `SessionState::new(name, branch, worktree_path)` constructor

### 3. Simplify `para-rs/src/core/session/manager.rs`
- Remove `CreateSessionParams` struct - pass parameters directly
- Remove session type handling logic
- Remove config snapshot logic
- Remove commit tracking
- Keep core methods: `create_session`, `load_session`, `save_session`, `list_sessions`, `delete_session`

### 4. Update Command Files
Update all command files to use simplified API:
- `para-rs/src/cli/commands/start.rs`
- `para-rs/src/cli/commands/dispatch.rs`
- `para-rs/src/cli/commands/finish.rs`
- `para-rs/src/cli/commands/list.rs`
- `para-rs/src/cli/commands/cancel.rs`
- `para-rs/src/cli/commands/recover.rs`

Remove complex parameter passing, use direct method calls.

### 5. Update `para-rs/src/core/session.rs`
- Remove exports for deleted modules (`migration`, `validation`)
- Remove complex type re-exports
- Keep only the essential exports: `SessionState`, `SessionStatus`, `SessionManager`

## Implementation Notes

### Session Creation
```rust
// Simple, direct approach
let session = SessionState::new(name, branch, worktree_path);
session_manager.save_session(&session)?;
```

### Session Discovery
When repository root is needed, discover it dynamically:
```rust
let git_service = GitService::discover()?;
let repo_root = git_service.repository().root;
```

### State File Format
Save sessions as simple JSON files:
```json
{
  "name": "feature-auth",
  "branch": "pc/20250110-143022",  
  "worktree_path": "/path/to/subtrees/pc/feature-auth-20250110-143022",
  "created_at": "2025-01-10T14:30:22Z",
  "status": "Active"
}
```

## Testing Requirements

- Run `just test` to ensure all Rust unit tests pass
- Verify that all commands still work with the simplified state model
- Test session lifecycle: create → use → finish → cleanup
- Ensure session listing and recovery still function correctly

## Success Criteria

1. **Unified Architecture Maintained**: All commands use `SessionManager` consistently
2. **Reduced Complexity**: State model contains only essential data
3. **Code Reduction**: Significant reduction in lines of code while maintaining functionality
4. **Functional Equivalence**: All para commands work exactly as before
5. **Clean Codebase**: No unused imports, dead code, or over-engineered abstractions

## Final Steps

After implementing the simplified design:
1. Run all tests: `just test`
2. Verify linting passes: `just lint`
3. Ensure formatting is correct: `just fmt`
4. Test the full workflow manually with a few commands
5. Call `para finish 'Simplify unified session management for better maintainability'`

This task transforms an over-engineered system into a clean, maintainable unified architecture that preserves all benefits while eliminating unnecessary complexity.