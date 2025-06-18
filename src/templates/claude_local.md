<!-- Para Agent Instructions - DO NOT COMMIT -->
# Para Session Status Commands

You are working in para session: {session_name}

Use these commands to communicate your progress:

**Required status updates:**
```bash
# Every status update MUST be SHORT (5 words max) with test status and confidence
para status "Starting authentication" --tests unknown --confidence medium
para status "Implementing JWT" --tests passed --confidence high --todos 2/5
para status "Fixing middleware" --tests failed --confidence low --todos 3/5
para status "Blocked Redis mocking" --tests failed --confidence low --blocked

# IMPORTANT: --tests flag MUST reflect ALL tests in the codebase, not just current feature!
# Run full test suite before updating status
```

**DO NOT CONFUSE confidence with test status:**
- `--confidence` = How confident you are about solving the problem
- `--tests` = Current actual state of ALL tests in the codebase
- Example: You can be confident (`--confidence high`) while tests are still failing (`--tests failed`)

**Test Status Guidelines:**
- `--tests passed`: ALL tests in the entire codebase are passing (just test succeeded)
- `--tests failed`: One or more tests are failing anywhere in the codebase (just test failed)
- `--tests unknown`: Haven't run tests yet or tests are currently running

**CRITICAL: Report ACTUAL test status, not your progress:**
- If `just test` fails → use `--tests failed` (even if you're "about to fix it")
- If `just test` passes → use `--tests passed`
- If you haven't run `just test` yet → use `--tests unknown`

NEVER report partial test results. Always run the complete test suite.
ALWAYS report the current reality of test status, not your intentions or progress.

**When complete:**
```bash
para finish "Add user authentication with JWT tokens"
```

Remember: 
- **STATUS MUST BE 5 WORDS MAX** (e.g., "Fixing auth tests", "Adding API endpoint")
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
# REQUIRED: Send final status first (5 WORDS MAX!)
para status "Completed auth module" --tests passed --confidence high --todos 5/5

# Then provide your summary...
```

**CRITICAL RULES:**
1. Status messages MUST be 5 words or less
2. NEVER provide a summary without first sending a final status update
3. Final status MUST show actual test results (not your intentions)