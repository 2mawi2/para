#!/bin/bash
# Build a custom Docker image for para development
# Looks for Dockerfile in priority order:
# 1. .para/Dockerfile.custom (repository-specific)
# 2. docker/Dockerfile.para-dev (default)

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Function to print colored messages
print_info() {
    echo -e "${GREEN}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

# Check if para-claude:latest exists
if ! docker image inspect para-claude:latest &> /dev/null; then
    print_error "❌ Error: 'para-claude:latest' image not found."
    echo "Please ensure you have the base image available."
    exit 1
fi

# Determine which Dockerfile to use
DOCKERFILE=""
IMAGE_NAME="para-custom:latest"

# Check for repository-specific Dockerfile first
if [ -f "$PROJECT_ROOT/.para/Dockerfile.custom" ]; then
    DOCKERFILE="$PROJECT_ROOT/.para/Dockerfile.custom"
    print_info "🔍 Found repository-specific Dockerfile: .para/Dockerfile.custom"
    # Use repo name in image name if possible
    REPO_NAME=$(basename "$PROJECT_ROOT")
    IMAGE_NAME="para-${REPO_NAME}:latest"
elif [ -f "$PROJECT_ROOT/docker/Dockerfile.para-dev" ]; then
    DOCKERFILE="$PROJECT_ROOT/docker/Dockerfile.para-dev"
    print_info "🔍 Using default Dockerfile: docker/Dockerfile.para-dev"
    IMAGE_NAME="para-dev:latest"
else
    print_error "❌ No Dockerfile found!"
    echo "Please create one of:"
    echo "  - .para/Dockerfile.custom (for this repository)"
    echo "  - docker/Dockerfile.para-dev (default)"
    exit 1
fi

print_info "🐳 Building custom Docker image: $IMAGE_NAME"
echo "Using Dockerfile: $DOCKERFILE"
echo ""

# Build the image
if docker build -t "$IMAGE_NAME" -f "$DOCKERFILE" "$PROJECT_ROOT"; then
    echo ""
    print_info "✅ Successfully built $IMAGE_NAME"
    echo ""
    echo "📝 Next steps:"
    echo "1. Create authenticated version:"
    echo "   ./create-custom-authenticated.sh $IMAGE_NAME"
    echo ""
    echo "2. Use with para:"
    echo "   para dispatch my-feature --container --docker-image ${IMAGE_NAME%-*}-authenticated:latest"
else
    print_error "❌ Build failed!"
    exit 1
fi