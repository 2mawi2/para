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
- **MANDATORY: After using TodoWrite tool, IMMEDIATELY update status with --todos flag**
- Update status when:
  - Starting new work
  - **IMMEDIATELY after every TodoWrite tool use** (with progress --todos X/Y)
  - After running tests
  - Confidence level changes
  - Getting blocked
  - Making significant progress

**CRITICAL: TodoWrite → Status Update Pattern:**
```bash
# 1. Update your todos first
TodoWrite tool with updated progress

# 2. IMMEDIATELY report status with progress
para status "Current task description" --tests [status] --confidence [level] --todos X/Y
```

This ensures the orchestrator can see your progress in real-time!

## MANDATORY: Final Status Before Summary

**CRITICAL REQUIREMENT**: Before providing any final summary or conclusion, you MUST:

1. Send a final status update with your current state
2. Include test results, confidence level, and any remaining todos
3. ONLY THEN provide your summary

Example:
```bash
# REQUIRED: Send final status first
para status "Completed authentication module implementation" --tests passed --confidence high --todos 5/5

# Then provide your summary...
```

**NEVER** provide a summary without first sending a final status update. This is essential for proper monitoring and tracking.