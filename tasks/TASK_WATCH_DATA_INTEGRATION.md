# Task: Para Watch Real Data Integration

Integrate Para Watch TUI with actual session data instead of mock data.

## Data Integration Requirements

### 1. Real Session Data Loading
- **Replace mock data**: Load actual Para sessions from `.para/state/` directory
- **Session discovery**: Find all active Para sessions
- **State determination**: Map session status to UI states
- **Agent name extraction**: Get agent names from session metadata

### 2. Session State Mapping
```rust
// Current session status -> UI state mapping
fn determine_session_state(session_path: &Path) -> SessionState {
    // Active sessions with worktrees = Working
    // Finished sessions on branches = Human Review  
    // Keep AI Review as mock for now
}
```

### 3. Para Dispatch Description Parameter
- **Add description parameter**: `para dispatch --description "task description"`
- **Optional parameter**: Can be empty, but prompt agents to set it
- **Session metadata**: Store description in session data
- **CLI integration**: Update dispatch command arguments

### 4. Session Information Extraction
```rust
pub struct RealSessionInfo {
    pub task_name: String,        // From session name
    pub agent_name: String,       // From session metadata/generated
    pub description: String,      // From dispatch description parameter
    pub state: SessionState,      // Derived from session status
    pub branch_name: String,      // Git branch for session
    pub created_at: DateTime,     // Session creation time
    pub last_activity: DateTime,  // Last modification time
    pub worktree_path: PathBuf,   // Path to worktree
}
```

### 5. Data Mapping Logic

#### Working State
- **Criteria**: Active session with worktree exists
- **Check**: `.para/worktrees/session-name` directory exists
- **Status**: Currently being worked on

#### Human Review State  
- **Criteria**: Finished session with branch but no worktree
- **Check**: Branch `para/session-name` exists but worktree removed
- **Status**: Ready for human review/merge

#### AI Review State (Mock)
- **Keep mock data**: AI review not implemented yet
- **Future integration**: Will be real data later
- **Mixed display**: Show real Working/Human Review + mock AI Review

### 6. CLI Enhancement - Dispatch Description

#### Update Dispatch Arguments
```rust
#[derive(Args, Debug)]
pub struct DispatchArgs {
    // ... existing fields
    
    /// Task description for the session
    #[arg(long, short = 'd')]
    pub description: Option<String>,
}
```

#### Session Metadata Storage
```rust
// Store in .para/state/sessions/session-name/metadata.json
{
    "session_name": "auth-flow-20240115",
    "agent_name": "claude-agent-1",
    "description": "Implement OAuth2 authentication flow",
    "created_at": "2024-01-15T10:15:00Z",
    "status": "working"
}
```

### 7. File System Integration

#### Session Discovery
```rust
pub fn load_active_sessions() -> Result<Vec<RealSessionInfo>> {
    let state_dir = get_para_state_dir()?;
    let sessions_dir = state_dir.join("sessions");
    
    // Scan for session directories
    // Load metadata.json for each
    // Check worktree and branch status
    // Map to SessionState
}
```

#### Worktree Status Check
```rust
fn has_active_worktree(session_name: &str) -> bool {
    let worktree_path = Path::new(".para/worktrees").join(session_name);
    worktree_path.exists() && worktree_path.is_dir()
}
```

#### Branch Status Check
```rust
fn has_session_branch(session_name: &str) -> bool {
    // Check if para/session-name branch exists
    Command::new("git")
        .args(["branch", "--list", &format!("para/{}", session_name)])
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}
```

### 8. Mixed Data Display Strategy

#### For Now (Phase 1)
- **Real Working sessions**: From actual `.para/` data
- **Real Human Review sessions**: Finished sessions on branches
- **Mock AI Review sessions**: Keep current mock data for visual completeness

#### Future (Phase 2)
- Replace AI Review mock data with real implementation
- Add actual AI review process integration
- Full real-time data integration

### 9. Error Handling
- **Graceful fallback**: Show mock data if real data loading fails
- **Empty states**: Handle case where no sessions exist
- **Corrupt data**: Handle malformed session metadata
- **Permission errors**: Handle file system access issues

## Implementation Tasks

1. **CLI Enhancement**: Add `--description` parameter to `para dispatch`
2. **Session Metadata**: Store description and metadata in session files
3. **Data Loading**: Replace mock data with real session discovery
4. **State Mapping**: Implement Working/Human Review state detection
5. **Mixed Display**: Combine real data with mock AI Review data
6. **Error Handling**: Robust error handling for file system operations

## Testing Requirements

- Test with real Para sessions (create test sessions)
- Test empty state (no sessions)
- Test mixed data display (real + mock)
- Test dispatch with description parameter
- Test session state transitions
- Test error handling for missing files/permissions

Keep UI code unchanged. Focus only on data loading and integration.

When complete, run: para finish "Integrate Para Watch with real session data and dispatch descriptions"