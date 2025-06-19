# Task: Add --dangerously-skip-permissions Support to Extended Action

## Repository
https://github.com/2mawi2/claude-code-action-extended

## Problem Statement
The extended Claude Code Action needs to support the `--dangerously-skip-permissions` flag to bypass permission prompts in automated workflows. Currently, Claude is asking for permission to use tools (like Write) which blocks automated execution.

## Required Changes

### 1. Update `action.yml`

Add a new input parameter:

```yaml
inputs:
  # ... existing inputs ...
  dangerously_skip_permissions:
    description: 'Skip Claude permission prompts (use with caution)'
    required: false
    default: 'false'
```

### 2. Update the action execution

In the step that runs Claude (likely in the composite steps or the main execution), add the flag when the parameter is true:

```yaml
- name: Run Claude
  shell: bash
  run: |
    # ... existing setup ...
    
    # Add the flag if requested
    CLAUDE_FLAGS=""
    if [[ "${{ inputs.dangerously_skip_permissions }}" == "true" ]]; then
      CLAUDE_FLAGS="--dangerously-skip-permissions"
    fi
    
    # Run Claude with the flags
    claude $CLAUDE_FLAGS # ... rest of the command
```

Or if using the Claude Code npm package directly, pass it as a configuration option.

### 3. Handle in the prepare/execution scripts

If the action uses TypeScript/JavaScript to execute Claude, update the execution to include the flag:

```typescript
// Check if dangerously_skip_permissions is set
const skipPermissions = process.env.DANGEROUSLY_SKIP_PERMISSIONS === 'true';

// Add to Claude execution options
const claudeOptions = {
  // ... existing options ...
  ...(skipPermissions && { dangerouslySkipPermissions: true })
};
```

## Testing

After implementation, test with a workflow that includes:

```yaml
- uses: 2mawi2/claude-code-action-extended@main
  with:
    use_oauth: true
    dangerously_skip_permissions: true
    direct_prompt: "Create a test file"
    allowed_tools: "Write"
```

## Success Criteria

- ✅ The `dangerously_skip_permissions` parameter is available in action.yml
- ✅ When set to `true`, Claude doesn't prompt for permissions
- ✅ Automated workflows can write files without being blocked
- ✅ The Auditor workflow successfully creates task files
- ✅ The Gardener workflow successfully edits code files

## Security Note

This flag bypasses Claude's permission system, so it should only be used in trusted automated workflows where the allowed tools are explicitly controlled.