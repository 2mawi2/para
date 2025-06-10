# Task 16: Complete Integrate Command with Proper Strategies

## Objective
Implement the complete `integrate` command with proper merge strategies (merge, squash, rebase) and full conflict resolution workflow to match the legacy shell implementation.

## Background
The current Rust `integrate` command is incomplete and doesn't implement the different integration strategies defined in the PRD. The legacy shell implementation has sophisticated integration logic with conflict handling that needs to be replicated.

## Requirements

### 1. Integration Strategy Implementation
Must support three integration strategies as defined in PRD Section 5.2:

**Merge Strategy:**
- Fast-forward merge when possible
- Create merge commit when fast-forward not possible
- Preserve commit history from feature branch

**Squash Strategy:**
- Combine all commits from feature branch into single commit
- Use `git reset --soft <base-branch>` to squash commits
- Create single commit with provided message

**Rebase Strategy:**
- Rebase feature branch onto latest base branch
- Maintain individual commit messages from feature branch
- Handle conflicts during rebase process

### 2. Integration Workflow
Complete workflow matching legacy implementation:

1. **Pre-integration validation:**
   - Verify session exists and is valid
   - Check for uncommitted changes
   - Validate base branch exists

2. **Base branch preparation:**
   - Switch to base branch
   - Pull latest changes from remote (if configured)
   - Verify clean working tree

3. **Strategy execution:**
   - Execute chosen integration strategy
   - Handle conflicts if they occur
   - Save integration state for conflict resolution

4. **Post-integration cleanup:**
   - Remove worktree directory
   - Clean up feature branch (if successful)
   - Close IDE window for session
   - Remove session state files

### 3. Conflict Resolution Integration
Implement conflict state management:

**Integration State File:**
```rust
#[derive(Serialize, Deserialize)]
struct IntegrationState {
    session_id: String,
    feature_branch: String,
    base_branch: String,
    strategy: IntegrationStrategy,
    conflict_files: Vec<PathBuf>,
    created_at: DateTime<Utc>,
}
```

**Conflict Handling:**
- Save integration state when conflicts occur
- Provide clear instructions for conflict resolution
- Support `para continue` command to resume integration
- Open IDE with conflicted files (if supported)

### 4. Command Line Interface
Update CLI parsing to support strategy selection:

```bash
# Default strategy (from config or squash)
para integrate "Implement user authentication"

# Explicit strategy selection
para integrate "Add feature" --strategy merge
para integrate "Fix bug" --strategy squash  
para integrate "Update docs" --strategy rebase

# Session-specific integration
para integrate session-name "Commit message" --strategy merge
```

### 5. Configuration Integration
Support strategy configuration:

```rust
#[derive(Serialize, Deserialize)]
struct GitConfig {
    branch_prefix: String,
    auto_stage: bool,
    auto_commit: bool,
    default_integration_strategy: IntegrationStrategy, // New field
}
```

## Implementation Details

### Files to Create/Modify
1. `para-rs/src/cli/commands/integrate.rs` - Complete rewrite
2. `para-rs/src/core/git/integration.rs` - New integration logic
3. `para-rs/src/core/git/strategy.rs` - Strategy implementations
4. `para-rs/src/core/session/integration_state.rs` - State management
5. `para-rs/src/cli/parser.rs` - Update for strategy flags

### Key Structs and Enums
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum IntegrationStrategy {
    Merge,
    Squash,
    Rebase,
}

#[derive(Debug)]
pub struct IntegrationResult {
    success: bool,
    conflicts: Vec<ConflictInfo>,
    branch_cleaned: bool,
    session_closed: bool,
}

#[derive(Debug)]
pub struct ConflictInfo {
    file_path: PathBuf,
    conflict_markers: Vec<String>,
    resolution_required: bool,
}
```

### Integration Logic Flow
```rust
pub fn execute_integration(
    session_id: &str,
    commit_message: &str,
    strategy: IntegrationStrategy,
) -> Result<IntegrationResult> {
    // 1. Validate session and get info
    let session_info = load_session_state(session_id)?;
    
    // 2. Prepare base branch
    prepare_base_branch(&session_info.base_branch)?;
    
    // 3. Execute strategy
    match strategy {
        IntegrationStrategy::Merge => execute_merge_strategy(&session_info, commit_message),
        IntegrationStrategy::Squash => execute_squash_strategy(&session_info, commit_message),
        IntegrationStrategy::Rebase => execute_rebase_strategy(&session_info, commit_message),
    }
}
```

### Error Handling
Comprehensive error handling for:
- Git operation failures
- Merge conflicts
- Missing branches or sessions
- Invalid repository state
- Network errors during remote pulls

### Testing Requirements
1. **Unit tests** for each integration strategy
2. **Integration tests** with temporary git repositories
3. **Conflict resolution tests** with simulated conflicts
4. **Legacy compatibility tests** using bats test suite
5. **Error condition tests** for edge cases

## Legacy Reference
Study the legacy implementation in:
- `lib/para-commands.sh` lines 163-173 (`handle_integrate_command`)
- `lib/para-session.sh` integration functions
- `lib/para-git.sh` git operation functions

Pay special attention to:
- Integration state management
- Conflict detection and handling
- Branch cleanup logic
- Error message formatting

## Validation Criteria
- [ ] All three integration strategies work correctly
- [ ] Conflict resolution workflow functions properly
- [ ] Integration state is properly saved and restored
- [ ] IDE window closing works during integration
- [ ] Feature branch cleanup happens correctly
- [ ] Error messages match legacy implementation style
- [ ] All tests pass (unit, integration, and legacy bats)
- [ ] Command line interface matches PRD specification

## Completion
When complete, call `para finish "Implement complete integrate command with merge strategies and conflict resolution"` to finish the task.

The agent should ensure thorough testing and verify that the implementation handles all edge cases present in the legacy shell version.