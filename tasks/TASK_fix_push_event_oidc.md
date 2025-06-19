# Task: Fix OIDC Authentication for Push Events

## Repository
https://github.com/2mawi2/claude-code-action-extended

## Problem Statement
The extended Claude Code Action fails with "Invalid OIDC token" error when triggered by push events. The action tries to exchange OIDC tokens for GitHub app tokens even when OAuth authentication is being used.

## Error Details
```
App token exchange failed: 401 Unauthorized - Invalid OIDC token
Failed to setup GitHub token: Error: Invalid OIDC token.
```

## Root Cause
The action is attempting OIDC authentication for all events, but push events from automated workflows don't have the same OIDC context as workflow_dispatch events.

## Required Fix

### In the prepare.ts or token setup logic:

1. **Check if OAuth is being used**:
   - If `use_oauth: true` is set, skip OIDC token exchange entirely
   - Use the provided OAuth tokens directly

2. **Handle push events differently**:
   - For push events with OAuth, don't attempt OIDC authentication
   - Use the provided `github_token` if available

3. **Update the token setup logic**:
```typescript
// Pseudo-code for the fix
if (useOAuth && (claudeAccessToken && claudeRefreshToken)) {
  // Skip OIDC token exchange for OAuth mode
  console.log("Using OAuth authentication, skipping OIDC");
  // Use the OAuth tokens directly
} else if (githubToken) {
  // Use provided GitHub token
  console.log("Using provided GitHub token");
} else {
  // Attempt OIDC token exchange (existing logic)
  console.log("Attempting OIDC token exchange");
}
```

## Testing
After fixing, test with:
1. Push event with OAuth tokens
2. Workflow_dispatch with OAuth tokens
3. Ensure both work without OIDC errors

## Success Criteria
- ✅ Push events work with OAuth authentication
- ✅ No "Invalid OIDC token" errors when use_oauth is true
- ✅ Gardener workflow successfully processes tasks on push events
- ✅ Auditor workflow continues to work with workflow_dispatch