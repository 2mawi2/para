# Task 17: Fix Session State Consistency Across Commands

## Objective
Unify session state management across all commands by implementing a consistent state format, centralized state handling, and proper session lifecycle management.

## Background
The current Rust implementation has inconsistent session state handling - different commands use different state file formats (`.state` vs JSON), and there's no unified session management system. This causes compatibility issues and makes the codebase harder to maintain.

## Current Problems
1. **Multiple state formats**: Some commands use `.state` files, others use JSON
2. **Scattered state logic**: Each command handles state differently
3. **Missing metadata**: Session creation time, last modified, etc. not tracked
4. **Inconsistent paths**: State file naming and location varies
5. **No validation**: State files can become corrupted or inconsistent

## Requirements

### 1. Unified Session State Format
Define a single, comprehensive session state structure:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    // Core identification
    pub id: String,
    pub name: String,
    pub session_type: SessionType,
    
    // Git information
    pub branch: String,
    pub base_branch: String,
    pub worktree_path: PathBuf,
    pub repository_root: PathBuf,
    
    // Session metadata
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub status: SessionStatus,
    
    // Optional data
    pub initial_prompt: Option<String>,
    pub commit_count: u32,
    pub last_commit_hash: Option<String>,
    
    // Configuration snapshot
    pub config_snapshot: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Manual,      // Created via `start`
    Dispatched,  // Created via `dispatch`
    Recovered,   // Restored via `recover`
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Finishing,
    Integrating,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub branch_prefix: String,
    pub subtrees_dir: String,
    pub ide_name: String,
    pub auto_stage: bool,
    pub auto_commit: bool,
}
```

### 2. Centralized State Management
Create a unified session manager:

```rust
pub struct SessionManager {
    state_dir: PathBuf,
    config: Config,
}

impl SessionManager {
    pub fn new(config: Config) -> Result<Self>;
    
    // Core operations
    pub fn create_session(&mut self, params: CreateSessionParams) -> Result<SessionState>;
    pub fn load_session(&self, session_id: &str) -> Result<SessionState>;
    pub fn save_session(&self, session: &SessionState) -> Result<()>;
    pub fn delete_session(&self, session_id: &str) -> Result<()>;
    
    // Discovery and listing
    pub fn list_active_sessions(&self) -> Result<Vec<SessionSummary>>;
    pub fn find_session_by_path(&self, path: &Path) -> Result<Option<SessionState>>;
    pub fn auto_detect_session(&self) -> Result<SessionState>;
    
    // Lifecycle management
    pub fn update_session_status(&mut self, session_id: &str, status: SessionStatus) -> Result<()>;
    pub fn cleanup_orphaned_sessions(&mut self) -> Result<Vec<String>>;
    pub fn validate_session_integrity(&self, session_id: &str) -> Result<ValidationResult>;
}
```

### 3. State File Format Standardization
**File naming convention:**
- Primary state: `.para_state/{session_id}.json`
- Backup state: `.para_state/backups/{session_id}.json.bak`
- Integration state: `.para_state/integration/{session_id}.json`

**File structure:**
```json
{
  "version": "1.0",
  "session": {
    "id": "clever_phoenix_20250609-123456",
    "name": "clever_phoenix",
    "session_type": "Dispatched",
    "branch": "para/clever_phoenix_20250609-123456",
    "base_branch": "master",
    "worktree_path": "/path/to/subtrees/para/clever_phoenix_20250609-123456",
    "repository_root": "/path/to/repo",
    "created_at": "2025-06-09T12:34:56Z",
    "last_modified": "2025-06-09T12:35:12Z",
    "status": "Active",
    "initial_prompt": "Implement user authentication",
    "commit_count": 2,
    "last_commit_hash": "abc123def456",
    "config_snapshot": {
      "branch_prefix": "para",
      "subtrees_dir": "subtrees/para",
      "ide_name": "claude",
      "auto_stage": true,
      "auto_commit": true
    }
  }
}
```

### 4. Migration Strategy
Handle existing state files:

```rust
pub struct StateMigrator {
    state_dir: PathBuf,
}

impl StateMigrator {
    pub fn migrate_legacy_state_files(&self) -> Result<MigrationReport>;
    pub fn convert_shell_state_file(&self, path: &Path) -> Result<SessionState>;
    pub fn backup_existing_state(&self) -> Result<()>;
    pub fn validate_migration(&self) -> Result<ValidationReport>;
}

#[derive(Debug)]
pub struct MigrationReport {
    pub migrated_count: usize,
    pub failed_migrations: Vec<String>,
    pub backup_location: PathBuf,
}
```

### 5. Command Integration
Update all commands to use unified state management:

**Start Command:**
```rust
pub fn execute_start(args: StartArgs) -> Result<()> {
    let mut session_manager = SessionManager::new(config)?;
    
    let params = CreateSessionParams {
        name: args.name,
        session_type: SessionType::Manual,
        initial_prompt: None,
        // ... other params
    };
    
    let session = session_manager.create_session(params)?;
    // Launch IDE and continue...
}
```

**Finish Command:**
```rust
pub fn execute_finish(args: FinishArgs) -> Result<()> {
    let mut session_manager = SessionManager::new(config)?;
    
    let session = if let Some(id) = args.session_id {
        session_manager.load_session(&id)?
    } else {
        session_manager.auto_detect_session()?
    };
    
    session_manager.update_session_status(&session.id, SessionStatus::Finishing)?;
    // Continue with finish logic...
}
```

### 6. Validation and Integrity Checks
Implement comprehensive validation:

```rust
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug)]
pub enum ValidationIssue {
    MissingWorktree,
    InvalidBranch,
    CorruptedStateFile,
    MismatchedPaths,
    OutdatedFormat,
}

impl SessionManager {
    pub fn validate_all_sessions(&self) -> Result<Vec<(String, ValidationResult)>>;
    pub fn repair_session(&mut self, session_id: &str) -> Result<RepairResult>;
    pub fn cleanup_invalid_sessions(&mut self) -> Result<CleanupReport>;
}
```

## Implementation Details

### Files to Create/Modify
1. `para-rs/src/core/session/mod.rs` - Session manager module
2. `para-rs/src/core/session/state.rs` - State definitions and serialization
3. `para-rs/src/core/session/manager.rs` - SessionManager implementation
4. `para-rs/src/core/session/migration.rs` - Legacy state migration
5. `para-rs/src/core/session/validation.rs` - State validation logic
6. Update all command files to use SessionManager

### State Directory Structure
```
.para_state/
├── sessions/              # Active session state files
│   ├── session1.json
│   └── session2.json
├── backups/              # Backup state files
│   ├── session1.json.bak
│   └── session2.json.bak
├── integration/          # Integration state for conflict resolution
│   └── session1.json
├── archive/              # Archived session metadata
│   └── archived_sessions.json
└── migration/            # Migration logs and backups
    ├── migration.log
    └── pre_migration_backup/
```

### Error Handling
Comprehensive error handling for:
- State file corruption
- Permission issues
- Concurrent access
- Invalid JSON format
- Missing directories
- Migration failures

### Testing Requirements
1. **Unit tests** for SessionManager operations
2. **Migration tests** with various legacy state formats
3. **Validation tests** for corrupted state scenarios
4. **Concurrency tests** for multiple para instances
5. **Integration tests** with all commands using new state system
6. **Backward compatibility tests** with existing workflows

## Legacy Compatibility
Ensure compatibility with:
- Existing `.state` files from shell implementation
- Current session naming conventions
- Existing worktree directory structures
- Legacy backup system expectations

## Validation Criteria
- [ ] All commands use unified SessionManager
- [ ] State files have consistent JSON format
- [ ] Legacy state files can be migrated successfully
- [ ] Session validation catches common corruption issues
- [ ] Concurrent access is handled safely
- [ ] Error messages are clear and actionable
- [ ] All tests pass including legacy bats tests
- [ ] Performance is not degraded from current implementation

## Completion
When complete, call `para finish "Implement unified session state management across all commands"` to finish the task.

The agent should prioritize maintaining backward compatibility while implementing the new unified system, and ensure thorough testing of the migration process.