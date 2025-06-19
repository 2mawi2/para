# Fix Tool Permissions in Extended Claude Action

## Problem
The extended Claude Code action (2mawi2/claude-code-action-extended) doesn't properly handle tool permissions when:
- Running on automated events (push, schedule, workflow_dispatch)
- Using OAuth authentication
- Claude agents request to use tools listed in allowed_tools

## Error Pattern
```
Claude requested permissions to use Edit, but you haven't granted it yet.
Claude requested permissions to use Bash, but you haven't granted it yet.
```

## Required Fix

### 1. In the extended action's prepare.ts:
- Ensure ALLOWED_TOOLS environment variable is properly passed to the base action
- Verify tool permissions are parsed correctly for automated events

### 2. In the extended action's action.yml:
- Ensure allowed_tools is passed through to claude-code-base-action
- Add explicit environment variable mapping

### 3. Potential fix in prepare.ts:
```typescript
// Ensure allowed_tools are properly formatted and passed
const allowedTools = process.env.ALLOWED_TOOLS || '';
if (allowedTools) {
  // Write to environment file for base action
  fs.appendFileSync(process.env.GITHUB_ENV!, `ALLOWED_TOOLS=${allowedTools}\n`);
}
```

### 4. Update the base action call:
```yaml
- uses: grll/claude-code-base-action@latest
  with:
    allowed_tools: ${{ env.ALLOWED_TOOLS }}
    # ... other parameters
```

## Testing
After fixing:
1. Test with push event + OAuth
2. Test with workflow_dispatch + OAuth
3. Verify all allowed tools work without permission errors

## Success Criteria
- ✅ No "requested permissions" errors for tools in allowed_tools
- ✅ Claude agents can use Edit, Write, Bash commands as configured
- ✅ Both Auditor and Gardener workflows complete successfully