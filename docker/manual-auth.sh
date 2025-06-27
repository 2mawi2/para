#!/bin/bash
# Manually authenticate a custom Docker image

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <custom-image>"
    echo "Example: $0 para-dev:latest"
    exit 1
fi

CUSTOM_IMAGE="$1"

echo "🔐 Manual authentication process for: $CUSTOM_IMAGE"
echo ""

# Step 1: Run the custom image interactively
echo "📦 Starting container from $CUSTOM_IMAGE..."
docker run -d --name para-auth-temp \
    -v para-auth-claude-60802:/root/.config \
    -v para-auth-claude-60802:/root/.claude \
    "$CUSTOM_IMAGE" sleep 3600

# Step 2: Check if claude works
echo "🔍 Checking Claude authentication..."
if docker exec para-auth-temp claude --version >/dev/null 2>&1; then
    echo "✅ Claude is working!"
else
    echo "❌ Claude authentication failed. Running interactive login..."
    docker exec -it para-auth-temp claude /login
fi

# Step 3: Commit the authenticated container
echo "💾 Creating authenticated image..."
docker commit para-auth-temp "${CUSTOM_IMAGE%-*}-authenticated:latest"

# Cleanup
echo "🧹 Cleaning up..."
docker stop para-auth-temp >/dev/null
docker rm para-auth-temp >/dev/null

echo ""
echo "✅ Created authenticated image: ${CUSTOM_IMAGE%-*}-authenticated:latest"
echo ""
echo "📝 Usage:"
echo "  para dispatch my-feature --container --docker-image ${CUSTOM_IMAGE%-*}-authenticated:latest"