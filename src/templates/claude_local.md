<!-- Para Agent Instructions - DO NOT COMMIT -->
# Para Session Status Commands

You are working in para session: {session_name}

Use these commands to communicate your progress:

**Required status updates:**
```bash
# Every status update MUST be SHORT (5 words max) with test status
para status "Starting authentication" --tests unknown
para status "Implementing JWT" --tests passed --todos 2/5
para status "Fixing middleware" --tests failed --todos 3/5
para status "Blocked Redis mocking" --tests failed --blocked

# IMPORTANT: --tests flag MUST reflect ALL tests in the codebase, not just current feature!
# Run full test suite before updating status
```


**Test Status Guidelines:**
- `--tests passed`: ALL tests in the entire codebase are passing (just test succeeded)
- `--tests failed`: One or more tests are failing anywhere in the codebase (just test failed)
- `--tests unknown`: Haven't run tests yet or tests are currently running

**CRITICAL: Report ACTUAL test status, not your progress:**
- If `just test` fails → use `--tests failed` (even if you're "about to fix it")
- If `just test` passes → use `--tests passed`
- If you haven't run `just test` yet → use `--tests unknown`

**MANDATORY: Immediate test failure reporting:**
- **As soon as you discover ANY failing test** → IMMEDIATELY update status with `--tests failed`
- **Don't wait** until you try to fix it - report the failure the moment you see it
- **Even if you didn't cause the failure** - if you see red tests, report `--tests failed`
- **Before starting any new work** - always run `just test` first and report results

NEVER report partial test results. Always run the complete test suite.
ALWAYS report the current reality of test status, not your intentions or progress.

**Required Workflow:**
```bash
# 1. ALWAYS start by checking test status
just test
para status "Starting work" --tests [result]

# 2. If you discover failing tests at any point
para status "Found failing tests" --tests failed

# 3. Work on your task, updating status regularly
para status "Implementing feature" --tests [current_status] --todos X/Y

# 4. Before finishing, ensure all tests pass
just test
para status "Ready to finish" --tests passed --todos X/X
```

**MANDATORY: When ALL work is complete AND you are confident in the solution:**
```bash
# 1. REQUIRED: Final status update (5 words max)
para status "Completed auth module" --tests passed --todos 5/5

# 2. Provide your summary to the user
# [Your summary here explaining what was accomplished]

# 3. REQUIRED: Finish the session with commit message ONLY if task is fully implemented
para finish "Add user authentication with JWT tokens"
```

**DO NOT call `para finish` if:**
- You need user input or clarification
- The task is only partially implemented
- You're unsure about the solution
- Tests are failing or you haven't verified the implementation works correctly

**CRITICAL: You MUST call `para finish` ONLY when your work is completely implemented and you are confident in the solution!**

Remember: 
- **STATUS MUST BE 5 WORDS MAX** (e.g., "Fixing auth tests", "Adding API endpoint")
- EVERY status update must include: task description and --tests flag
- Run ALL tests before updating status (not just tests for current feature)
- **MANDATORY: After using TodoWrite tool, IMMEDIATELY update status with --todos flag**
- Update status when:
  - Starting new work
  - **IMMEDIATELY after every TodoWrite tool use** (with progress --todos X/Y)
  - After running tests
  - Getting blocked
  - Making significant progress

**CRITICAL: TodoWrite → Status Update Pattern:**
```bash
# 1. Update your todos first
TodoWrite tool with updated progress

# 2. IMMEDIATELY report status with progress
para status "Current task description" --tests [status] --todos X/Y
```

This ensures the orchestrator can see your progress in real-time!

## MANDATORY: Final Status, Summary, and Finish

**CRITICAL REQUIREMENT**: When your work is complete AND you are confident in the solution, you MUST follow this exact sequence:

1. Send a final status update with your current state
2. Include test results and any remaining todos
3. Provide your summary
4. **CALL `para finish` TO COMPLETE THE SESSION - BUT ONLY IF TASK IS FULLY IMPLEMENTED**

Example:
```bash
# STEP 1: REQUIRED final status (5 WORDS MAX!)
para status "Completed auth module" --tests passed --todos 5/5

# STEP 2: Provide your summary to the user
# [Your summary explaining what was accomplished]

# STEP 3: ONLY if task is completely implemented - Finish the session
para finish "Add user authentication with JWT tokens"
```

**When NOT to call `para finish`:**
- If you need user input or clarification on requirements
- If the implementation is incomplete or only partially working
- If you're unsure about the correctness of your solution
- If tests are failing or you haven't properly verified the implementation

**CRITICAL RULES:**
1. Status messages MUST be 5 words or less
2. NEVER provide a summary without first sending a final status update
3. Final status MUST show actual test results (not your intentions)
4. **ONLY call `para finish` when your work is completely implemented and you are confident in the solution - DO NOT call finish if you need user input, are unsure, or the task is incomplete!**