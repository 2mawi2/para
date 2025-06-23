# Docker Claude Authentication

This document describes how Para handles Claude authentication for Docker containers.

## Overview

Para automatically retrieves Claude credentials from the macOS Keychain and passes them to Docker containers, allowing Claude to run authenticated inside containers without manual login.

## Authentication Flow

1. **Credential Retrieval**: Para reads credentials from macOS Keychain using the `security` command
2. **JSON Validation**: The credentials JSON is validated to ensure it contains all required fields
3. **Environment Variable**: The complete credentials JSON is passed to the container via `CLAUDE_CREDENTIALS_JSON`

## Credential Structure

The credentials JSON retrieved from the Keychain contains:

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1750714269974,
    "scopes": ["user:inference", "user:profile"],
    "subscriptionType": "max"
  }
}
```

## Container Setup

Inside the Docker container, the Claude application can use the credentials by:

1. Reading the `CLAUDE_CREDENTIALS_JSON` environment variable
2. Writing the JSON to the appropriate credentials file location
3. Starting Claude with the authenticated session

## Example Container Script

```bash
#!/bin/bash
# Script to setup Claude credentials in container

if [ -n "$CLAUDE_CREDENTIALS_JSON" ]; then
    # Create Claude config directory
    mkdir -p ~/.claude
    
    # Write credentials to file
    echo "$CLAUDE_CREDENTIALS_JSON" > ~/.claude/credentials.json
    
    echo "Claude credentials configured successfully"
else
    echo "Warning: No Claude credentials found in environment"
fi
```

## Security Considerations

- Credentials are only passed to containers created by Para
- The credentials JSON contains sensitive tokens and should not be logged
- Containers should handle credentials securely and not expose them

## Platform Support

- **macOS**: Full automatic credential retrieval from Keychain
- **Linux**: Manual credential setup required (future enhancement)
- **Windows**: Not yet supported

## Troubleshooting

If authentication fails:

1. Ensure you're logged into Claude on the host machine: `claude /login`
2. Verify credentials exist in Keychain: `security find-generic-password -s "Claude Code-credentials"`
3. Check that Docker container receives the environment variable
4. Ensure the container properly processes the credentials JSON