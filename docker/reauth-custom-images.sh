#!/bin/bash
# Reauthenticate para images when tokens expire

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

echo "🔐 Para Docker Reauthentication"
echo "==============================="
echo ""

# Step 1: Check current auth status
print_info "📊 Checking current authentication status..."
if ! para auth status; then
    print_warning "⚠️  Authentication check failed"
fi
echo ""

# Step 2: Reauthenticate base image
print_info "1️⃣  Reauthenticating base para image..."
if ! para auth reauth; then
    print_error "❌ Base reauthentication failed!"
    echo "Try running: para auth cleanup && para auth setup"
    exit 1
fi
echo ""

# Step 3: Check for custom images
if [ -f .para/Dockerfile.custom ]; then
    print_info "2️⃣  Found custom Dockerfile.custom, rebuilding authenticated image..."
    
    # Get the script directory
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    
    # Build custom image
    if [ -x "$SCRIPT_DIR/build-custom-image.sh" ]; then
        print_info "   Building custom image..."
        if ! "$SCRIPT_DIR/build-custom-image.sh"; then
            print_error "❌ Custom image build failed!"
            exit 1
        fi
    else
        print_warning "⚠️  build-custom-image.sh not found, skipping build"
    fi
    
    # Get the image name
    REPO_NAME=$(basename "$PWD")
    CUSTOM_IMAGE="para-${REPO_NAME}:latest"
    
    # Create authenticated version
    if [ -x "$SCRIPT_DIR/create-custom-authenticated.sh" ]; then
        print_info "3️⃣  Creating authenticated version of $CUSTOM_IMAGE..."
        if ! "$SCRIPT_DIR/create-custom-authenticated.sh" "$CUSTOM_IMAGE"; then
            print_error "❌ Failed to create authenticated custom image!"
            exit 1
        fi
    else
        print_warning "⚠️  create-custom-authenticated.sh not found"
    fi
else
    print_info "ℹ️  No custom Dockerfile found, using standard para-authenticated:latest"
fi

echo ""
print_info "✅ Reauthentication complete!"
echo ""
echo "🚀 You can now use:"
echo "   para dispatch my-feature --container --docker-image para-authenticated:latest"
echo ""
echo "💡 Tips:"
echo "   - Test authentication with: docker run --rm para-authenticated:latest claude --help"
echo "   - Check status with: para auth status"
echo "   - If issues persist, try: para auth cleanup && para auth setup"