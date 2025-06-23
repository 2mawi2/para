#!/bin/bash
# Build script for Para Claude Docker image

set -e

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "🐳 Building Para base image with Claude..."

# Build the base image
docker build -f "$SCRIPT_DIR/Dockerfile.claude" -t para-claude:latest "$SCRIPT_DIR"

echo "✅ Base image built: para-claude:latest"
echo ""
echo "Note: This image includes Claude CLI pre-installed."
echo "Authentication will be handled separately when needed."