Your task is to orchestrate the review and merge of the following `para` branches: $ARGUMENTS.

**Phase 1: Parallel Review**
Use your internal `Task` tool to review each branch in parallel. IMPORTANT you must start all the initial review Tasks with your task tool in parallel for each review that can be performed in parallel. For each branch, start a new sub-task with the following instructions:
> **Sub-Task Prompt Template:**
> "You are a code reviewer. Review the changes on the branch `{{branch_name}}` against the current branch (run 'git status' first to confirm current branch). **CRITICAL:** Do NOT check out the branch; only use `git diff <current_branch>...{{branch_name}}` (note the THREE dots - this shows only the changes made on the branch since it diverged, avoiding confusion from files added to main after the branch was created).
> 
> Provide a structured review response in the following format:
> 
> **STATUS:** OK or NEEDS_FIX
> 
> **SUMMARY:** (2-3 sentences describing what was implemented/changed)
> 
> **COMPLETENESS:** (List what was successfully completed)
> - Feature/change 1
> - Feature/change 2
> - etc.
> 
> **ISSUES:** (Only if NEEDS_FIX - list specific problems)
> - Issue 1: Description
> - Issue 2: Description
> - etc.
> 
> **QUALITY:** (Brief assessment of code quality, test coverage, and adherence to project standards)"

**Phase 2: Follow-up Fixes (if needed)**
1.  Wait for all reviews to complete.
2.  **Provide Review Summary to User:**
    - List all branches reviewed with their STATUS
    - For OK branches: Show the SUMMARY and COMPLETENESS sections
    - For NEEDS_FIX branches: Show SUMMARY, ISSUES, and planned fixes
    - Give user a brief overview before proceeding
3.  If any review returned NEEDS_FIX:
    a. Create follow-up task files with detailed fix instructions for each failed review
    b. Resume ALL failed sessions using `para resume <session-name> --file <follow-up-task-file> --dangerously-skip-permissions`
    c. Monitor progress: `para list` to see active sessions
    d. Wait for ALL agents to complete their fixes and finish their sessions

**Phase 3: Final Review and Merge**
1.  Once all follow-up fixes are complete (no more active para sessions):
    a. Re-review all previously failed branches to ensure issues are resolved
    b. If any still have issues, repeat Phase 2
2.  When all branches pass review:
    a. Merge branches sequentially into `<current_branch>`, resolving any conflicts
    b. Start with independent branches, then dependent ones
    c. After each successful merge, delete the local and remote feature branch

**Follow-up Task Requirements**
When creating follow-up tasks for failed reviews, ensure each task includes:
- Detailed explanation of what needs to be fixed based on the review feedback
- Complete code examples for the fixes required
- **CRITICAL**: Explicit instruction to commit all changes to the branch before calling `para finish`

**Follow-up Task Template:**
> At the end of your follow-up task, always include:
> "After implementing all fixes:
> 1. Commit all changes: `git add . && git commit -m 'Fix [description of fixes]'`
> 2. Verify build works: `just build`
> 3. Run: `para finish '[commit message]'`"

IMPORTANT:

- Ensure to start each review TASK with our internal TASK in parallel if there are multiple reviews
- Ensure that each TASK is instructed not to checkout the branch
- Ensure that each TASK also knows the location of the original (or follow up) task file that the para dispatch agent was launched with if you know the task
- For failed reviews, always resume sessions with follow-up tasks that include commit and finish instructions
- Succeeded sessions that are not dependent on non succeeded sessions, can and should be merged directly and already integreated after the failed sessions where resumed. 