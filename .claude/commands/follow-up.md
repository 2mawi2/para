Handle follow-up actions after parallel reviews of para branches.

Arguments: $ARGUMENTS (optional comma-separated list of session names to follow up on)

This command addresses review feedback by canceling old sessions and dispatching fixes.

Steps:
1. Identify sessions that need fixes from review feedback (or use $ARGUMENTS if provided)
2. For each session, modify the original task file (e.g., `tasks/TASK_1_feature.md`):
   - Keep the original requirements (so reviewers can verify fixes)
   - Add a "Follow-up" section with review feedback and fix instructions
   - Update the finish command to use branch `follow-up/<session_name>`
3. Dispatch new sessions with the modified task files

Task file modification:
```
[Keep original task content...]

## Follow-up Instructions
Review the implementation on branch `para/<session_name>` and address:
- Fix: [specific issue from review]
- Update: [required change]

When done: para finish "Fix review feedback for [feature]" --branch follow-up/<session_name>
```

Implementation:
- Dispatch: `para dispatch <session> --file tasks/TASK_X_feature.md --dangerously-skip-permissions`

IMPORTANT:
- Do not use resume for now (resume currenlty cannot add instructions, ensure to use dispatch)
- Ensure that the para agent is instructed to checkout the original branch or worktree first in the task
- Ensure that the agent is instructed to use 'para finish' to complete the task
  
Example usage:
- `/follow-up` - Process all sessions mentioned in review feedback
- `/follow-up auth-api frontend-ui` - Only process these specific sessions

Note: The para MCP server provides integrated tools for session management that can be used instead of or alongside CLI commands.