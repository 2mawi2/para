# Task: Fix Extended Claude Action for Automated Events

## Repository
https://github.com/2mawi2/claude-code-action-extended

## Problem Statement
The extended Claude Code Action was modified to support automated events (`workflow_dispatch`, `schedule`, `push`), but it's still trying to create GitHub issue/PR comments for these events, which fails because automated events don't have an associated issue or PR (entityNumber = 0).

### Current Error
```
Error: Not Found - https://docs.github.com/rest/issues/comments#create-an-issue-comment
POST /repos/2mawi2/para/issues/0/comments - 404
```

## Our Intent and Goal

### What We Want to Achieve
1. **Autonomous Workflows**: Enable GitHub Actions workflows that run automatically on schedules or manual dispatch
2. **Claude Max Subscription**: Use OAuth tokens from expensive Claude Max subscription (not API keys)
3. **Technical Debt Automation**: Automatically find and fix technical debt (like `.unwrap()` usage) without human intervention

### How It Should Work
1. **Auditor Workflow** (`workflow_dispatch` or `schedule`):
   - Analyzes codebase for technical debt
   - Creates task files
   - Uses Claude to generate the task content via `direct_prompt`
   - Creates a PR with the task file

2. **Gardener Workflow** (`push` event when task files are added):
   - Picks up new task files
   - Creates a feature branch
   - Uses Claude to implement the fix via `direct_prompt`
   - Creates a PR with the fix

### Key Point
For automated events, we don't need GitHub comments or tracking - we just need Claude to execute the `direct_prompt` and return the result.

## Required Changes

### 1. Update `src/entrypoints/prepare.ts`

The prepare script needs to handle automated events differently:

```typescript
import { isAutomatedEvent } from "../github/context";

async function run() {
  try {
    // ... existing setup code ...

    // Step 2: Parse GitHub context
    const context = parseGitHubContext();

    // Step 3: Check trigger conditions
    const containsTrigger = await checkTriggerAction(context);
    if (!containsTrigger) {
      console.log("No trigger found, skipping remaining steps");
      return;
    }

    // NEW: Skip GitHub integration for automated events
    if (isAutomatedEvent(context)) {
      console.log(`Automated event detected: ${context.eventName}`);
      
      // For automated events, we only need to:
      // 1. Validate the direct prompt exists
      // 2. Set up minimal outputs for the action to proceed
      
      if (!context.inputs.directPrompt) {
        throw new Error("Automated events require a direct_prompt input");
      }

      // Set minimal outputs needed for Claude execution
      core.setOutput("trigger_found", "true");
      core.setOutput("is_automated", "true");
      
      // Skip all GitHub comment/branch operations
      console.log("Skipping GitHub operations for automated event");
      return;
    }

    // ... existing code for interactive events (issues, PRs) ...
  } catch (error) {
    // ... error handling ...
  }
}
```

### 2. Update the main action runner (if needed)

Ensure the main action execution handles automated events by:
- Skipping comment updates for automated events
- Executing Claude with the `direct_prompt` directly
- Returning results without trying to post comments

### 3. Test Scenarios

After making changes, test these scenarios:

1. **Test Workflow** (`workflow_dispatch`):
   ```yaml
   - uses: 2mawi2/claude-code-action-extended@main
     with:
       use_oauth: true
       direct_prompt: "Simple test message"
   ```
   Should execute without trying to create comments.

2. **Auditor Workflow** (`workflow_dispatch`):
   - Should analyze code
   - Create task file using Claude
   - Create PR with task

3. **Gardener Workflow** (`push` event):
   - Should pick up task file
   - Execute fix using Claude
   - Create PR with fix

## Implementation Notes

1. **Check your current implementation** - You may have already added some of this logic
2. **The key insight**: Automated events should bypass ALL GitHub comment/issue operations
3. **Keep it simple**: For automated events, we just need to pass the prompt to Claude and get the result
4. **Don't break interactive events**: Make sure the original functionality for issues/PRs still works

## Success Criteria

- ✅ No more "404 Not Found" errors for automated events
- ✅ `workflow_dispatch` events work with `direct_prompt`
- ✅ `schedule` events work with `direct_prompt`
- ✅ `push` events work with `direct_prompt`
- ✅ Auditor workflow successfully creates task files
- ✅ Gardener workflow successfully implements fixes
- ✅ Interactive events (issues, PRs) still work as before

## Testing Commands

After implementation, test with:

```bash
# Test basic automated event
gh workflow run "Test Extended Claude Action" -R 2mawi2/para

# Test full Auditor flow
gh workflow run "Auditor - Technical Debt Analysis" -R 2mawi2/para

# Watch the results
gh run list --workflow="Auditor - Technical Debt Analysis" -R 2mawi2/para
```

The goal is to have fully autonomous technical debt remediation using Claude Max subscription OAuth tokens!