Your task is to orchestrate the review and merge of the following `para` branches: $ARGUMENTS.

**Phase 1: Parallel Review**
Use your internal `Task` tool to review each branch in parallel. IMPORTANT you must start all the initial review Tasks with your task tool in parallel for each review that can be performed in parallel. For each branch, start a new sub-task with the following instructions:
> **Sub-Task Prompt Template:**
> "You are a code reviewer. Review the changes on the branch `{{branch_name}}` against the current branch (run 'git status' first to confirm current branch). **CRITICAL:** Do NOT check out the branch; only use `git diff <current_branch>...{{branch_name}}` (note the THREE dots - this shows only the changes made on the branch since it diverged, avoiding confusion from files added to main after the branch was created). If the changes are perfect, respond with only the word `OK`. Otherwise, respond with `NEEDS_FIX:` followed by a brief explanation."

**Phase 2: Decision and Merge**
1.  Wait for all reviews to complete.
2.  If any review failed:
    a. Create follow-up task files with detailed fix instructions for each failed review
    b. Resume the failed sessions using `para resume <session-name> --file <follow-up-task-file> --dangerously-skip-permissions` (try not to use dispatch rather prefer resume for a follow-up)
    c. Wait for agents to complete fixes and commit changes
    d. Re-review the fixed branches to ensure issues are resolved
3.  If all reviews returned `OK`, merge the branches into `<current_branch>` sequentially, resolving any conflicts.
4.  After a successful merge, delete the local and remote feature branch.

**Phase 3: Follow-up Task Requirements**
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