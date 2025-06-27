#!/bin/bash
# Build the para development Docker image on top of para-claude

set -euo pipefail

# Check if para-claude:latest exists
if ! docker image inspect para-claude:latest &> /dev/null; then
    echo "❌ Error: 'para-claude:latest' image not found."
    echo "Please run 'para pull' to download the base image."
    exit 1
fi

echo "🐳 Building para-dev Docker image..."
echo "This image will be based on 'para-claude:latest'."
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Get the project root (parent of docker directory)
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Build the image from project root with full context
docker build -t para-dev:latest -f "$SCRIPT_DIR/Dockerfile.para-dev" "$PROJECT_ROOT"

echo ""
echo "✅ Successfully built para-dev:latest"
echo ""
echo "This image is ready to be used as a base for authenticated images."
echo "For example, run 'para auth' which uses this as a default."
