Your task is to orchestrate the review and merge of the following `para` branches: $ARGUMENTS.

**Phase 1: Parallel Review**
Use your internal `Task` tool to review each branch in parallel. IMPORTANT you must start all the initial review Tasks with your task tool in parallel for each review that can be performed in parallel. For each branch, start a new sub-task with the following instructions:
> **Sub-Task Prompt Template:**
> "You are a code reviewer. Review the changes on the branch `{{branch_name}}` against the current branch (run 'git status' first to confirm current branch). **CRITICAL:** Do NOT check out the branch; only use `git diff <current_branch>..{{branch_name}}` (this shows exactly what changes merging {{branch_name}} would bring to the current branch). If the changes are perfect, respond with only the word `OK`. Otherwise, respond with `NEEDS_FIX:` followed by a brief explanation."

**Phase 2: Decision and Merge**
1.  Wait for all reviews to complete.
2.  If any review failed, HALT and report the failures.
3.  If all reviews returned `OK`, merge the branches into `<current_branch>` sequentially, resolving any conflicts.
4.  After a successful merge, delete the local and remote feature branch.

IMPORTANT:

- Ensure to start each review TASK with our internal TASK in parallel if there are multiple reviews
- Ensure that each TASK is instructed not to checkout the branch
- Ensure that each TASK also knows the location of the original (or follow up) task file that the para dispatch agent was launched with if you know the task