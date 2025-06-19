# Fix Environment Variable Format Error in Extended Claude Action

## Current Error
The extended action is failing with:
```
##[error]Unable to process file command 'env' successfully.
##[error]Invalid format 'Read'
```

## Root Cause
The recent fix (commit 4f22e1b) attempts to export ALLOWED_TOOLS to the GitHub environment file, but the format is incorrect. The error occurs because the allowed_tools string contains commas and special characters that break the environment file format.

## Example of the Problem
Current allowed_tools value:
```
"Edit,Read,Write,MultiEdit,Glob,Grep,LS,TodoRead,TodoWrite,Bash(just test),Bash(git add:*),..."
```

When written to GITHUB_ENV, this breaks because:
1. The value contains unescaped special characters
2. GitHub Actions environment file expects specific formatting
3. Commas and parentheses need proper handling

## Required Fix

### In prepare.ts
Instead of:
```typescript
fs.appendFileSync(process.env.GITHUB_ENV!, `ALLOWED_TOOLS=${allowedTools}\n`);
```

Use proper formatting:
```typescript
// Method 1: Use delimiter for multiline values
const delimiter = `EOF_${Math.random().toString(36).substr(2, 9)}`;
fs.appendFileSync(process.env.GITHUB_ENV!, `ALLOWED_TOOLS<<${delimiter}\n${allowedTools}\n${delimiter}\n`);

// OR Method 2: Properly escape the value
const escapedTools = allowedTools.replace(/\n/g, '%0A').replace(/\r/g, '%0D');
fs.appendFileSync(process.env.GITHUB_ENV!, `ALLOWED_TOOLS=${escapedTools}\n`);

// OR Method 3: Use core.exportVariable (if @actions/core is available)
import * as core from '@actions/core';
core.exportVariable('ALLOWED_TOOLS', allowedTools);
```

## Testing the Fix
1. The Auditor workflow should run without the "Invalid format" error
2. The ALLOWED_TOOLS should be properly passed to the base action
3. Claude agents should be able to use all specified tools without permission errors

## Additional Context
- The error happens in the prepare step of the extended action
- Both Auditor and Gardener workflows are affected
- The comma-separated format of allowed_tools is correct, but needs proper escaping for env vars
- Reference: https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#multiline-strings

## Success Criteria
- ✅ No "Unable to process file command 'env'" errors
- ✅ No "Invalid format" errors  
- ✅ ALLOWED_TOOLS environment variable is properly set
- ✅ Claude agents can use all tools specified in allowed_tools
- ✅ Both Auditor and Gardener workflows complete successfully