#!/bin/bash
# Build the para development Docker image

set -euo pipefail

echo "🐳 Building para-dev Docker image..."
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Build the image
docker build -t para-dev:latest -f "$SCRIPT_DIR/Dockerfile.para-dev" "$SCRIPT_DIR"

echo ""
echo "✅ Successfully built para-dev:latest"
echo ""
echo "📝 Usage:"
echo "  para start my-feature --container --docker-image para-dev:latest"
echo ""
echo "Or test directly:"
echo "  docker run --rm -it -v \$(pwd):/workspace para-dev:latest"