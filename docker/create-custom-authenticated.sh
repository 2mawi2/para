#!/bin/bash
# Create an authenticated image from a custom image

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored messages
print_info() {
    echo -e "${GREEN}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

# Get custom image name from argument or use default
CUSTOM_IMAGE="${1:-para-dev:latest}"
AUTH_IMAGE="${CUSTOM_IMAGE%-*}-authenticated:latest"

# Check if custom image exists
if ! docker image inspect "$CUSTOM_IMAGE" &> /dev/null; then
    print_error "❌ Error: Image '$CUSTOM_IMAGE' not found."
    echo "Please build it first with: ./build-custom-image.sh"
    exit 1
fi

# Check if authenticated base exists
if ! docker image inspect "para-authenticated:latest" &> /dev/null; then
    print_error "❌ Error: 'para-authenticated:latest' not found."
    echo "Please run 'para auth' first to create the authenticated base image."
    exit 1
fi

print_info "🐳 Creating authenticated image from: $CUSTOM_IMAGE"
echo ""

# Create a temporary Dockerfile with the dynamic image name
cat > /tmp/Dockerfile.custom-auth << EOF
# Start from the authenticated image to get the auth state
FROM para-authenticated:latest AS auth

# Now build from our custom image
FROM $CUSTOM_IMAGE

# Copy the authentication state from the authenticated image
COPY --from=auth /home/para/.claude /home/para/.claude
COPY --from=auth /home/para/.claude.json /home/para/.claude.json
EOF

# Build the new authenticated custom image
if docker build -t "$AUTH_IMAGE" -f /tmp/Dockerfile.custom-auth .; then
    # Clean up
    rm -f /tmp/Dockerfile.custom-auth
    
    echo ""
    print_info "✅ Created $AUTH_IMAGE"
    echo ""
    echo "📝 This image includes:"
    echo "   - All tools from $CUSTOM_IMAGE"
    echo "   - Authentication from para-authenticated:latest"
    echo ""
    echo "🚀 Usage:"
    echo "   para dispatch my-feature --container --docker-image $AUTH_IMAGE"
else
    print_error "❌ Build failed!"
    rm -f /tmp/Dockerfile.custom-auth
    exit 1
fi