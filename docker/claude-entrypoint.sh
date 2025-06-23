#!/bin/bash
# Claude authentication entrypoint script
# This script sets up Claude authentication using environment variables

set -e

# Check if we have the required environment variable
if [ -z "$CLAUDE_CREDENTIALS_JSON" ]; then
    echo "Error: Claude credentials not provided"
    echo "Please ensure CLAUDE_CREDENTIALS_JSON environment variable is set"
    exit 1
fi

# Create Claude config directory and cache directory
mkdir -p /home/para/.claude
mkdir -p /home/para/.cache/claude-cli-nodejs

# Write the credentials JSON directly to the file
echo "$CLAUDE_CREDENTIALS_JSON" > /home/para/.claude/.credentials.json

# Set proper permissions
chmod 600 /home/para/.claude/.credentials.json

echo "✅ Claude authentication configured successfully"

# Execute the command passed to the container (or default to bash)
exec "$@"