Your task is to orchestrate the review and merge of the following `para` branches: $ARGUMENTS.

**Phase 1: Parallel Review**
Use your internal `Task` tool to review each branch in parallel. IMPORTANT you must start all the initial review Tasks with your task tool in parallel for each review that can be performed in parallel. For each branch, start a new sub-task with the following instructions:
> **Sub-Task Prompt Template:**
> "You are a code reviewer. Review the changes on the branch `{{branch_name}}` against `main`. **CRITICAL:** Do NOT check out the branch; only use `git diff main..{{branch_name}}`. If the changes are perfect, respond with only the word `OK`. Otherwise, respond with `NEEDS_FIX:` followed by a brief explanation."

**Phase 2: Decision and Merge**
1.  Wait for all reviews to complete.
2.  If any review failed, HALT and report the failures.
3.  If all reviews returned `OK`, merge the branches into `main` sequentially, resolving any conflicts.
4.  After a successful merge, delete the local and remote feature branch.