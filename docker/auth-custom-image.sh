#!/bin/bash
# Authenticate a custom Docker image using para auth
# This works by temporarily replacing para-claude:latest with your custom image

set -euo pipefail

# Check arguments
if [ $# -ne 1 ]; then
    echo "Usage: $0 <custom-image>"
    echo "Example: $0 para-dev:latest"
    exit 1
fi

CUSTOM_IMAGE="$1"
ORIGINAL_CLAUDE_IMAGE=""

echo "🔐 Authenticating custom image: $CUSTOM_IMAGE"
echo ""

# Check if custom image exists
if ! docker image inspect "$CUSTOM_IMAGE" >/dev/null 2>&1; then
    echo "❌ Error: Custom image '$CUSTOM_IMAGE' not found!"
    echo "Please build it first."
    exit 1
fi

# Check if para-claude:latest exists (we'll need to restore it)
if docker image inspect "para-claude:latest" >/dev/null 2>&1; then
    echo "📦 Backing up existing para-claude:latest..."
    ORIGINAL_CLAUDE_IMAGE="para-claude-backup:$(date +%s)"
    docker tag "para-claude:latest" "$ORIGINAL_CLAUDE_IMAGE"
fi

# Tag our custom image as para-claude:latest
echo "🏷️  Tagging $CUSTOM_IMAGE as para-claude:latest..."
docker tag "$CUSTOM_IMAGE" "para-claude:latest"

# Run para auth (it will use our custom image)
echo "🔐 Running para auth..."
echo ""
para auth

# The authenticated image is now at para-authenticated:latest
# Tag it with our custom name
AUTHENTICATED_IMAGE="${CUSTOM_IMAGE%-*}-authenticated:latest"
echo ""
echo "🏷️  Tagging authenticated image as $AUTHENTICATED_IMAGE..."
docker tag "para-authenticated:latest" "$AUTHENTICATED_IMAGE"

# Clean up - restore original para-claude:latest if it existed
if [ -n "$ORIGINAL_CLAUDE_IMAGE" ]; then
    echo "🔄 Restoring original para-claude:latest..."
    docker tag "$ORIGINAL_CLAUDE_IMAGE" "para-claude:latest"
    docker rmi "$ORIGINAL_CLAUDE_IMAGE" >/dev/null 2>&1 || true
fi

echo ""
echo "✅ Success! Your authenticated custom image is ready: $AUTHENTICATED_IMAGE"
echo ""
echo "📝 Usage:"
echo "  para dispatch my-feature --container --docker-image $AUTHENTICATED_IMAGE"
echo ""