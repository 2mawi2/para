#!/bin/bash
# Authenticate a custom image with fresh Claude login

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <custom-image>"
    echo "Example: $0 para-dev:latest"
    exit 1
fi

IMAGE="$1"
OUTPUT_IMAGE="${IMAGE%-*}-authenticated:latest"

echo "🔐 Fresh authentication for: $IMAGE"
echo ""
echo "This will:"
echo "1. Start a container from your image"
echo "2. Run 'claude /login' for fresh authentication"
echo "3. Save the authenticated state"
echo ""

# Start container
echo "📦 Starting container..."
docker run -d --name para-fresh-auth "$IMAGE" sleep infinity

# Run authentication
echo "🔑 Starting Claude authentication..."
echo "Please complete the authentication process:"
docker exec -it para-fresh-auth claude /login

# Check if it worked
if docker exec para-fresh-auth claude --version >/dev/null 2>&1; then
    echo "✅ Authentication successful!"
    
    # Commit the authenticated container
    echo "💾 Saving authenticated image..."
    docker commit para-fresh-auth "$OUTPUT_IMAGE"
    
    echo ""
    echo "✅ Created: $OUTPUT_IMAGE"
    echo ""
    echo "📝 Usage:"
    echo "  para dispatch my-feature --container --docker-image $OUTPUT_IMAGE"
else
    echo "❌ Authentication failed!"
fi

# Cleanup
docker stop para-fresh-auth >/dev/null
docker rm para-fresh-auth >/dev/null