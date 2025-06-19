# Task: Implement Extended Claude Code Action with Automated Event Support

## Repository
https://github.com/2mawi2/claude-code-action-extended

## Objective
Modify the forked Claude Code Action to support automated GitHub events (`workflow_dispatch`, `schedule`, `push`) while maintaining OAuth authentication with Claude Max subscription.

## Background
The original `grll/claude-code-action` only supports interactive events (issues, comments, PRs) but throws an error for automated events:
```
Error: Unsupported event type: workflow_dispatch
```

This task adds support for automated events to enable autonomous workflows with Claude Max subscription OAuth tokens.

## Required Changes

### 1. Modify `src/github/context.ts`

**Find this section (around line 108):**
```typescript
    default:
      throw new Error(`Unsupported event type: ${context.eventName}`);
```

**Replace with:**
```typescript
    case "workflow_dispatch":
    case "schedule":
    case "push": {
      return {
        ...commonFields,
        payload: context.payload as any, // Generic payload for automated events
        entityNumber: 0, // No specific entity for automated events
        isPR: false,
      };
    }
    default:
      throw new Error(`Unsupported event type: ${context.eventName}`);
```

**Also update the type definition at the top (around line 25):**
```typescript
  payload:
    | IssuesEvent
    | IssueCommentEvent
    | PullRequestEvent
    | PullRequestReviewEvent
    | PullRequestReviewCommentEvent
    | any; // Allow any payload for automated events
```

**Add these helper functions at the end of the file:**
```typescript
// New helper functions for automated events
export function isWorkflowDispatchEvent(
  context: ParsedGitHubContext,
): boolean {
  return context.eventName === "workflow_dispatch";
}

export function isScheduleEvent(
  context: ParsedGitHubContext,
): boolean {
  return context.eventName === "schedule";
}

export function isPushEvent(
  context: ParsedGitHubContext,
): boolean {
  return context.eventName === "push";
}

export function isAutomatedEvent(
  context: ParsedGitHubContext,
): boolean {
  return isWorkflowDispatchEvent(context) || isScheduleEvent(context) || isPushEvent(context);
}
```

### 2. Modify `src/github/validation/trigger.ts`

**Add the import at the top:**
```typescript
import {
  isIssuesEvent,
  isIssueCommentEvent,
  isPullRequestEvent,
  isPullRequestReviewEvent,
  isPullRequestReviewCommentEvent,
  isAutomatedEvent, // Add this line
} from "../context";
```

**Find the `checkContainsTrigger` function and add this logic after the directPrompt check (around line 22):**
```typescript
  // For automated events (workflow_dispatch, schedule, push), always trigger when directPrompt is set
  if (isAutomatedEvent(context)) {
    console.log(`Automated event '${context.eventName}' detected, checking for direct prompt`);
    if (directPrompt) {
      console.log(`Direct prompt found for automated event, triggering action`);
      return true;
    } else {
      console.log(`No direct prompt for automated event '${context.eventName}', skipping`);
      return false;
    }
  }
```

### 3. Update `README.md` (Optional but Recommended)

Add a section explaining the extended functionality:

```markdown
# Claude Code Action Extended

Extended version of [grll/claude-code-action](https://github.com/grll/claude-code-action) with support for automated GitHub events.

## Key Differences from Original

### Supported Events

**Original grll/claude-code-action:**
- `issues` 
- `issue_comment`
- `pull_request`
- `pull_request_review`
- `pull_request_review_comment`

**Extended version (this fork):**
- All original events PLUS:
- ✅ `workflow_dispatch`
- ✅ `schedule` 
- ✅ `push`

### Usage for Autonomous Workflows

```yaml
name: Autonomous Workflow
on:
  schedule:
    - cron: '0 5 * * *'
  workflow_dispatch:

jobs:
  autonomous-task:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Run Claude
        uses: 2mawi2/claude-code-action-extended@main
        with:
          use_oauth: true
          claude_access_token: ${{ secrets.CLAUDE_ACCESS_TOKEN }}
          claude_refresh_token: ${{ secrets.CLAUDE_REFRESH_TOKEN }}
          claude_expires_at: ${{ secrets.CLAUDE_EXPIRES_AT }}
          direct_prompt: |
            Your autonomous task instructions here...
          allowed_tools: "Edit,Read,Write,Glob,Grep,Bash"
```
```

## Testing Instructions

After making the changes:

1. **Commit and push** the changes to the main branch
2. **Test with a simple workflow** using `workflow_dispatch` event
3. **Verify** that the action no longer throws "Unsupported event type" errors

## Success Criteria

- ✅ Action supports `workflow_dispatch`, `schedule`, and `push` events
- ✅ OAuth authentication still works with Claude Max subscription
- ✅ `direct_prompt` parameter triggers action for automated events
- ✅ No breaking changes to existing interactive event functionality
- ✅ Action can be used in autonomous workflows

## Key Points

- The main blocker was line 109 in `context.ts` throwing an error for unsupported events
- We're adding generic support for automated events with `entityNumber: 0` and `isPR: false`
- The trigger logic ensures automated events only run when `direct_prompt` is provided
- This enables autonomous workflows while maintaining security (no accidental triggers)

When complete, the para repository workflows will be able to use `2mawi2/claude-code-action-extended@main` for autonomous technical debt remediation with Claude Max subscription OAuth tokens.