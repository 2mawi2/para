# Para Monitor Data Integration - Follow-up Plan

## Problem Summary
The para-ui needs to display the same rich information shown in `para monitor`, but the current JSON output is missing key fields and has mismatched field names. The UI needs to show:
- Session name
- Session state (Active/Idle/Review/Stale)
- Last modified timestamp
- Current task description
- Test status (passed/failed/unknown)
- Progress percentage from todos
- Git diff stats (lines added/removed and files changed)

## Current Issues

### 1. Field Mismatch in DiffStats
**Para outputs:**
```json
"diff_stats": {
  "additions": 0,
  "deletions": 0
}
```

**Para-UI expects:**
```json
"diff_stats": {
  "files_changed": 5,
  "insertions": 245,
  "deletions": 32
}
```

### 2. Missing Session State Information
The `para list --json` output has a generic `status` field (active/dirty/missing/archived) but doesn't include the monitor-specific states (Active/Idle/Review/Stale) that are based on time-based activity detection.

### 3. Missing Files Changed Count
The DiffStats in para only tracks additions/deletions but not the number of files changed.

## Changes Required in Para Repository

### 1. Enhance DiffStats Structure
**File:** `src/core/status.rs`

Update the DiffStats structure to include files_changed:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,  // Keep as additions for backward compatibility
    pub deletions: usize,
    #[serde(alias = "insertions")]  // Allow both additions and insertions
    pub insertions: usize,  // Duplicate of additions for UI compatibility
}

impl DiffStats {
    pub fn new(files_changed: usize, additions: usize, deletions: usize) -> Self {
        Self {
            files_changed,
            additions,
            deletions,
            insertions: additions,  // Set both fields
        }
    }
}
```

### 2. Update Git Diff Calculation
**File:** `src/core/status.rs` (in calculate_diff_stats_for_session function)

Update to count files changed:
```rust
pub fn calculate_diff_stats_for_session(
    session_state: &crate::core::session::SessionState,
) -> Result<DiffStats> {
    // ... existing code ...
    
    let mut files_changed = 0;
    let mut additions = 0;
    let mut deletions = 0;
    
    for line in output.lines() {
        if let Some((add, del, _)) = parse_diff_line(line) {
            files_changed += 1;  // Each line represents a file
            additions += add;
            deletions += del;
        }
    }
    
    Ok(DiffStats::new(files_changed, additions, deletions))
}
```

### 3. Add Monitor Data Export Command
**File:** `src/cli/parser.rs`

Add a new subcommand for monitor data:
```rust
#[derive(Subcommand, Debug)]
pub enum StatusCommands {
    // ... existing commands ...
    
    /// Export monitor-style data for all sessions
    Monitor {
        /// Output format
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
}
```

### 4. Implement Monitor Data Export
**File:** `src/cli/commands/status.rs`

Add new function to export monitor-style data:
```rust
fn export_monitor_data(config: Config, json: bool) -> Result<()> {
    let service = SessionService::new(config.clone());
    let sessions = service.load_sessions(true)?;  // Include stale sessions
    
    // Convert SessionInfo to a serializable format
    let monitor_data: Vec<MonitorSessionData> = sessions.into_iter().map(|s| {
        MonitorSessionData {
            session_id: s.name.clone(),
            branch: s.branch,
            worktree_path: s.worktree_path.to_string_lossy().to_string(),
            session_state: match s.status {
                SessionStatus::Active => "active",
                SessionStatus::Idle => "idle",
                SessionStatus::Review => "review",
                SessionStatus::Ready => "ready",
                SessionStatus::Stale => "stale",
            }.to_string(),
            last_activity: s.last_activity,
            current_task: s.task,
            test_status: s.test_status.map(|t| match t {
                TestStatus::Passed => "passed",
                TestStatus::Failed => "failed",
                TestStatus::Unknown => "unknown",
            }.to_string()),
            todo_percentage: s.todo_percentage,
            is_blocked: s.is_blocked,
            diff_stats: s.diff_stats,
        }
    }).collect();
    
    if json {
        println!("{}", serde_json::to_string_pretty(&monitor_data)?);
    } else {
        // Display in table format
        display_monitor_data(&monitor_data);
    }
    
    Ok(())
}

#[derive(Serialize)]
struct MonitorSessionData {
    pub session_id: String,
    pub branch: String,
    pub worktree_path: String,
    pub session_state: String,  // active/idle/review/stale
    pub last_activity: DateTime<Utc>,
    pub current_task: String,
    pub test_status: Option<String>,
    pub todo_percentage: Option<u8>,
    pub is_blocked: bool,
    pub diff_stats: Option<DiffStats>,
}
```

### 5. Alternative: Enhance Existing Commands
Instead of a new command, enhance the existing commands to include monitor data:

**Option A: Add monitor fields to `para list --json`**
```rust
// In src/cli/commands/list/formatters.rs
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    // ... existing fields ...
    
    // Add monitor-specific fields
    pub session_state: String,  // active/idle/review/stale
    pub current_task: Option<String>,
    pub test_status: Option<String>,
    pub todo_percentage: Option<u8>,
    pub diff_stats: Option<DiffStats>,
}
```

**Option B: Add `--monitor` flag to status command**
```rust
// In src/cli/parser.rs
Show {
    session: Option<String>,
    #[arg(long, help = "Output as JSON")]
    json: bool,
    #[arg(long, help = "Include monitor-style enriched data")]
    monitor: bool,
}
```

## Changes Required in Para-UI Repository

### 1. Update DiffStats Type
**File:** `src-tauri/src/para_cli/types.rs`

Make DiffStats more flexible:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    #[serde(default)]
    pub files_changed: usize,
    #[serde(alias = "additions")]
    pub insertions: usize,
    pub deletions: usize,
}
```

### 2. Add Session State Enum
**File:** `src-tauri/src/para_cli/types.rs`

Add monitor-specific session states:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Active,  // Recent activity (< 5 min)
    Idle,    // No activity (5-30 min)
    Review,  // Finished, ready for review
    Ready,   // Legacy finished state
    Stale,   // No activity (> 30 min)
}
```

### 3. Update Service to Use Monitor Data
**File:** `src-tauri/src/para_cli/service.rs`

Add method to fetch monitor-style data:
```rust
impl ParaService {
    pub async fn get_monitor_sessions(&self) -> Result<Vec<MonitorSession>> {
        // Try new monitor command first
        match self.client.execute_json(&["status", "monitor", "--json"]).await {
            Ok(sessions) => Ok(sessions),
            Err(_) => {
                // Fall back to combining list and status
                let list_data = self.client.list_sessions(false).await?;
                let status_data = self.client.get_all_statuses().await?;
                self.combine_to_monitor_format(list_data, status_data)
            }
        }
    }
}
```

### 4. Update Frontend Display
**File:** `src/components/SessionsTab.tsx`

Display the rich monitor data:
```typescript
function SessionCard({ session }: { session: MonitorSession }) {
  const stateColor = {
    active: 'text-green-500',
    idle: 'text-amber-500',
    review: 'text-purple-500',
    ready: 'text-indigo-500',
    stale: 'text-gray-500'
  }[session.session_state];
  
  return (
    <div className="session-card">
      <div className="flex justify-between">
        <h3>{session.session_id}</h3>
        <span className={stateColor}>{session.session_state}</span>
      </div>
      
      <p className="text-sm">{session.current_task}</p>
      
      <div className="flex gap-4 text-xs">
        {session.test_status && (
          <span className={session.test_status === 'passed' ? 'text-green-400' : 'text-red-400'}>
            Tests: {session.test_status}
          </span>
        )}
        
        {session.todo_percentage !== undefined && (
          <span>Progress: {session.todo_percentage}%</span>
        )}
        
        {session.diff_stats && (
          <span>
            {session.diff_stats.files_changed || 0} files, 
            +{session.diff_stats.insertions} -{session.diff_stats.deletions}
          </span>
        )}
        
        {session.is_blocked && (
          <span className="text-red-500">BLOCKED</span>
        )}
      </div>
      
      <p className="text-xs text-gray-400">
        Last activity: {new Date(session.last_activity).toLocaleString()}
      </p>
    </div>
  );
}
```

## Implementation Priority

### Phase 1: Quick Fix (Para-UI only)
1. Update DiffStats type to handle both field names
2. Make files_changed optional with default value 0
3. Use additions as insertions if insertions is missing

### Phase 2: Para CLI Enhancement
1. Add files_changed to DiffStats calculation
2. Implement `para status monitor --json` command
3. Include all monitor fields in the output

### Phase 3: Full Integration
1. Update para-ui to use the new monitor command
2. Implement real-time updates
3. Add session state colors and icons

## Testing Requirements

### Para CLI Tests
```bash
# Test monitor data export
para status monitor --json | jq .

# Verify all fields are present
para status monitor --json | jq '.[0] | keys'

# Test backward compatibility
para status show --json  # Should still work
```

### Para-UI Tests
```typescript
// Test handling of both old and new formats
const oldFormat = { additions: 10, deletions: 5 };
const newFormat = { files_changed: 3, insertions: 10, deletions: 5 };

// Both should work with the updated types
```

## Success Criteria
1. ✅ UI displays all monitor data fields
2. ✅ DiffStats shows files changed, additions, deletions
3. ✅ Session states match monitor (active/idle/review/stale)
4. ✅ Current task descriptions are shown
5. ✅ Test status and todo progress displayed
6. ✅ Last activity timestamps are accurate
7. ✅ Backward compatibility maintained