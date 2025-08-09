# Para Repository: Add JSON Output Support

## Objective
Add JSON output support to the `para list` command to enable programmatic consumption by para-ui and other tools.

## Changes Required

### 1. Update CLI Parser
**File**: `src/cli/parser.rs`

Add JSON flag to the ListArgs struct (around line 307):
```rust
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show additional session details
    #[arg(long, short = 'v', help = "Show verbose session information")]
    pub verbose: bool,

    /// Show archived sessions
    #[arg(long, short = 'a', help = "Show archived sessions")]
    pub archived: bool,

    /// Quiet output (minimal formatting for completion)
    #[arg(long, short = 'q', help = "Quiet output for completion")]
    pub quiet: bool,
    
    /// Output as JSON
    #[arg(long, help = "Output as JSON")]
    pub json: bool,  // NEW FIELD
}
```

### 2. Make Data Structures Serializable
**File**: `src/cli/commands/list/formatters.rs`

Add serde derives to existing structs:
```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
    pub merge_mode: String,
    pub status: SessionStatus,
    pub last_modified: Option<DateTime<Utc>>,
    pub has_uncommitted_changes: Option<bool>,
    pub is_current: bool,
    pub session_type: SessionType,
    pub container_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Dirty,
    Missing,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Worktree,
    Container,
}
```

### 3. Add JSON Display Function
**File**: `src/cli/commands/list/formatters.rs`

Add new function after the existing display functions:
```rust
pub fn display_json_sessions(sessions: &[SessionInfo]) -> Result<()> {
    let json = serde_json::to_string_pretty(sessions)
        .map_err(|e| crate::utils::ParaError::config_error(
            format!("Failed to serialize sessions to JSON: {}", e)
        ))?;
    println!("{}", json);
    Ok(())
}
```

Update the main display_sessions function (around line 55):
```rust
pub fn display_sessions(sessions: &[SessionInfo], args: &ListArgs) -> Result<()> {
    let result = if args.json {
        display_json_sessions(sessions)  // NEW
    } else if args.quiet {
        display_quiet_sessions(sessions)
    } else if args.verbose {
        display_verbose_sessions(sessions)
    } else {
        display_compact_sessions(sessions)
    };

    // Only show tip for non-JSON, non-quiet output
    if !args.quiet && !args.json && result.is_ok() {
        println!("\nTip: Use 'para monitor' for interactive session management");
    }

    result
}
```

### 4. Add Serde Dependency
**File**: `Cargo.toml`

Ensure serde_json is in dependencies (it should already be there for other features):
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Expected JSON Output Format

When running `para list --json`, the output should be:
```json
[
  {
    "session_id": "auth-feature",
    "branch": "para/auth-feature",
    "worktree_path": "/Users/user/project/.para/worktrees/auth-feature",
    "base_branch": "main",
    "merge_mode": "squash",
    "status": "active",
    "last_modified": "2024-01-15T14:30:00Z",
    "has_uncommitted_changes": false,
    "is_current": true,
    "session_type": "worktree",
    "container_status": null
  },
  {
    "session_id": "payment-api",
    "branch": "para/payment-api",
    "worktree_path": "/Users/user/project/.para/worktrees/payment-api",
    "base_branch": "main",
    "merge_mode": "squash",
    "status": "dirty",
    "last_modified": "2024-01-15T12:00:00Z",
    "has_uncommitted_changes": true,
    "is_current": false,
    "session_type": "container",
    "container_status": "running"
  }
]
```

Empty list should return:
```json
[]
```

## Testing Requirements

### Unit Tests
**File**: `src/cli/commands/list/formatters.rs` (in tests module)

Add test for JSON output:
```rust
#[test]
fn test_display_json_sessions() -> Result<()> {
    let sessions = vec![
        create_test_session_info(
            "test-session-1",
            "para/test-branch-1",
            SessionStatus::Active,
            false,
        ),
        create_test_session_info(
            "test-session-2",
            "para/test-branch-2",
            SessionStatus::Dirty,
            true,
        ),
    ];

    // Capture output or verify it serializes without error
    let json = serde_json::to_string_pretty(&sessions)?;
    assert!(json.contains("test-session-1"));
    assert!(json.contains("test-session-2"));
    assert!(json.contains("\"status\": \"active\""));
    assert!(json.contains("\"status\": \"dirty\""));
    
    Ok(())
}
```

### Integration Tests
Test the full command:
```bash
# Test with no sessions
para list --json
# Should output: []

# Test with active sessions
para start test-session
para list --json
# Should output valid JSON array with session

# Test with archived sessions
para list --json --archived
# Should include archived sessions

# Test that JSON is valid
para list --json | jq .
# Should parse without errors
```

## Error Handling

1. **Serialization errors**: Should return proper error message if serialization fails
2. **Invalid UTF-8 in paths**: PathBuf should serialize to string properly
3. **Null values**: Optional fields should serialize as null when None

## Compatibility Notes

- This change is backward compatible - existing usage without --json flag remains unchanged
- The JSON structure matches the internal SessionInfo structure for consistency
- Field names use snake_case in JSON (Rust convention)
- Timestamps use ISO 8601 format (RFC 3339)
- Enums are serialized as lowercase strings

## Success Criteria

1. ✅ `para list --json` outputs valid JSON
2. ✅ All session fields are included in JSON output
3. ✅ Empty session list returns `[]`
4. ✅ JSON can be parsed by standard tools (jq, etc.)
5. ✅ Existing text output modes still work
6. ✅ Tests pass for JSON serialization
7. ✅ Documentation updated to mention --json flag

## Implementation Order

1. Add the `json` field to `ListArgs`
2. Add `Serialize` derives to data structures
3. Implement `display_json_sessions` function
4. Update `display_sessions` to handle JSON flag
5. Add unit tests
6. Test manually with real sessions
7. Update help text/documentation

## Additional Considerations

- Consider adding `--json` to other commands in the future (monitor, recover, etc.)
- The same pattern can be applied to other list-like commands
- Consider adding a global `--output-format` flag in the future for consistency