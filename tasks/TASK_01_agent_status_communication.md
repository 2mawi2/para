# Task: Design and Implement Agent Status Communication System

## Objective
Create a simple, effective status communication system that allows para agents to report their progress, test status, and blockers back to the orchestrator without manual polling.

## Requirements

### 1. Simple CLI API
Design a single `para status` command that combines all status information:
```bash
para status "Working on auth middleware" --tests passed --confidence high --todos 3/7
para status "Fixing test failures" --tests failed --confidence low --todos 5/7
para status "Need help with Redis mocking" --tests failed --confidence low --blocked
```

**Command structure:**
- Base: `para status "<current task description>"`
- Required flags: `--tests <passed|failed|unknown>` and `--confidence <high|medium|low>`
- Optional flags: `--todos <completed>/<total>` and `--blocked`

This single-command approach ensures agents always provide complete status in one atomic update.

### 2. CLAUDE.local.md Auto-generation
When `para dispatch` creates a new agent session:
- Automatically create/append to `CLAUDE.local.md` in the worktree
- Include clear instructions on using the status commands
- Handle edge case where file already exists (append with separator)
- Ensure file is in .gitignore (never committed)

### 3. Storage and Communication Mechanism
Investigate and implement:
- Status storage in `.para/sessions/{session-name}/status.json`
- File structure for status data (current task, test status, blockers, timestamps)
- How orchestrator reads status (file watching vs polling)
- Integration with existing para list command

**Status JSON Schema:**
```json
{
  "session_name": "auth-api",
  "current_task": "Implementing JWT validation",
  "test_status": "passed" | "failed" | "unknown",
  "is_blocked": false,
  "blocked_reason": null,
  "todos_completed": 3,
  "todos_total": 7,
  "confidence": "high",
  "last_update": "2024-01-18T10:30:00Z"
}
```

Note: If todos are never reported, `todos_completed` and `todos_total` are omitted from JSON.

### 4. Architecture Decisions
Determine:
- Where to implement status module (suggest: `src/core/status.rs`)
- How to integrate with existing session management
- File watching vs simple file reads for monitoring
- JSON schema for status files
- **Worktree handling**: If para is run inside a worktree, detect session from path
  - Look for `.para` in parent directories
  - Read/write status to correct `.para/sessions/{name}/status.json`
  - Context-aware commands work normally

## Investigation Tasks

1. **Research file watching in Rust**
   - Look at notify crate for cross-platform file watching
   - Evaluate if needed for v1 or if polling is sufficient

2. **Design status.json schema**
   - What fields are essential vs nice-to-have
   - How to handle message history (array with limit?)
   - Timestamp formats and timezone handling

3. **CLI argument parsing**
   - How to add status subcommand to existing clap structure
   - Validation of status types (tests, blocked, etc.)

4. **Integration points**
   - Where in dispatch command to create CLAUDE.local.md
   - How to make status updates context-aware (detect current session)
   - Updates to para list to show status summary

## Implementation Plan

### Phase 1: Core Status Module
1. Create `src/core/status.rs` with:
   - Status struct definition
   - Read/write functions for status.json
   - Status update logic

2. Add CLI command in `src/cli/commands/`:
   - Parse status command arguments  
   - Call into status module
   - Handle context detection

### Phase 2: Agent Instructions
1. Create CLAUDE.local.md template
2. Modify dispatch command to:
   - Check for existing CLAUDE.local.md
   - Create/append with para instructions if exists
   - but usually CLAUDE.local.md is ignored so it should not exist and get created but ensure its gitignored

### Phase 3: Status Display
1. Enhance `para list` to show status inline
2. Add `para status show` to display current session status
3. Consider future `para monitor` command for live view
4. Remove "Task:" display from UI (replaced by status API)
5. Remove session number (#) from list to save space
6. Keep "Last Activity" timestamp but clarify naming

## Success Criteria
- Agents can update status with simple, memorable commands
- Status persists across agent restarts
- Orchestrator can quickly see all agent statuses
- No permissions or confirmations required
- Works seamlessly in Docker environments (future-proof)
- Todo progress is automatically tracked when agent uses TodoWrite tool

## Example CLAUDE.local.md Content
```markdown
<!-- Para Agent Instructions - DO NOT COMMIT -->
# Para Session Status Commands

You are working in para session: {session_name}

Use these commands to communicate your progress:

**Required status updates:**
```bash
# Every status update MUST include current task, test status, and confidence
para status "Starting user authentication" --tests unknown --confidence medium
para status "Implementing JWT tokens" --tests passed --confidence high --todos 2/5
para status "Fixing auth middleware" --tests failed --confidence low --todos 3/5
para status "Need help with Redis mocking" --tests failed --confidence low --blocked

# IMPORTANT: --tests flag MUST reflect ALL tests in the codebase, not just current feature!
# Run full test suite before updating status
```

**Test Status Guidelines:**
- `--tests passed`: ALL tests in the entire codebase are passing
- `--tests failed`: One or more tests are failing anywhere in the codebase
- `--tests unknown`: Haven't run tests yet or tests are currently running

NEVER report partial test results. Always run the complete test suite.

**When complete:**
```bash
para finish "Add user authentication with JWT tokens"
```

Remember: 
- EVERY status update must include: task description, --tests flag, and --confidence flag
- Run ALL tests before updating status (not just tests for current feature)
- After using TodoWrite tool, include --todos flag with completed/total
- Update status when:
  - Starting new work
  - After running tests
  - Confidence level changes
  - Getting blocked
  - Making significant progress
```

## Workflow Instructions Template

The CLAUDE.local.md should contain clear instructions based on the workflow type:

### Standard Workflow (with para finish)
```markdown
When your task is complete:
1. Run ALL tests and update final status:
   `para status "Task complete" --tests passed --confidence high`
2. Commit and create branch: `para finish "Your descriptive commit message"`
```

### Integration Workflow (direct to main)
```markdown
When your task is complete:
1. Run ALL tests and update final status:
   `para status "Task complete" --tests passed --confidence high`
2. Integrate directly: `para integrate "Your commit message"`
```

### Review Workflow (no auto-finish)
```markdown
When your task is complete:
1. Run ALL tests and update final status:
   `para status "Task complete, ready for review" --tests passed --confidence high`
2. Wait for orchestrator to handle integration
```

The dispatch command should determine which workflow template to include based on task requirements or command flags.

## Design Decisions

1. **No history** - Only current state is stored (but persisted in status.json)
2. **Stale status** - Follow existing patterns (use last_update timestamp, UI can show age)
3. **No severity levels** - Simple blocked/not blocked is sufficient  
4. **Test status** - Simple passed/failed string, no detailed error messages
5. **Todo format** - Use `--todos completed/total` format (e.g., `--todos 3/7`)
   - CLI parses as two integers separated by '/'
   - Agent provides numbers, UI computes and shows percentage
   - If no todos reported, UI doesn't show todo field
   - If agent finishes without completing todos, status shows "finished" but todo percentage remains as last reported

## UI Display Logic

- Show todo progress as percentage: "42% (3/7 todos)"
- If no --todos ever called: Don't show todo field
- If session finished but todos incomplete: Show "Finished" status with last todo percentage
- Example format:
  ```
  Session         State      Last Modified    Current Task                    Tests   Progress  Confidence
  auth-api        Active     2min ago         Implementing JWT validation     Passed  71%       High
  frontend-ui     Active     15min ago        Setting up Redux store         Failed  43%       Low
  ```
- Remove old "Task:" field and session numbers from display
- "Last Modified" shows when worktree files were last changed (existing behavior)
- Low confidence + blocked = needs immediate attention
- Low confidence + working = may need proactive check-in
- Medium/High confidence = agent is on track