#!/bin/bash
# Create an authenticated version of a custom Docker image
# This is a workaround until para auth supports custom base images

set -euo pipefail

# Check arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <source-image> <output-image>"
    echo "Example: $0 para-dev:latest para-dev-authenticated:latest"
    exit 1
fi

SOURCE_IMAGE="$1"
OUTPUT_IMAGE="$2"

echo "🔧 Creating authenticated version of $SOURCE_IMAGE"
echo ""

# Check if source image exists
if ! docker image inspect "$SOURCE_IMAGE" >/dev/null 2>&1; then
    echo "❌ Error: Source image '$SOURCE_IMAGE' not found!"
    echo "Please build it first."
    exit 1
fi

# Check if para-authenticated exists
if ! docker image inspect "para-authenticated:latest" >/dev/null 2>&1; then
    echo "❌ Error: 'para-authenticated:latest' not found!"
    echo "Please run 'para auth' first."
    exit 1
fi

# Create a temporary container from authenticated image
TEMP_CONTAINER="temp-auth-transfer-$$"
echo "📦 Creating temporary container..."
docker create --name "$TEMP_CONTAINER" para-authenticated:latest sleep 1

# Export the .config directory from authenticated container
echo "📤 Exporting authentication data..."
docker cp "$TEMP_CONTAINER:/root/.config" /tmp/para-auth-config-$$
docker cp "$TEMP_CONTAINER:/root/.claude" /tmp/para-claude-config-$$ 2>/dev/null || true
docker cp "$TEMP_CONTAINER:/root/.claude.json" /tmp/para-claude-json-$$ 2>/dev/null || true

# Clean up temporary container
docker rm "$TEMP_CONTAINER" >/dev/null

# Create Dockerfile for authenticated image
cat > /tmp/Dockerfile-auth-$$ << EOF
FROM $SOURCE_IMAGE

# Copy authentication data
COPY para-auth-config-$$ /root/.config
COPY para-claude-config-$$ /root/.claude
COPY para-claude-json-$$ /root/.claude.json

# Fix permissions
RUN chown -R root:root /root/.config /root/.claude /root/.claude.json || true
EOF

# Build the authenticated image
echo "🏗️  Building authenticated image..."
cd /tmp
docker build -t "$OUTPUT_IMAGE" -f Dockerfile-auth-$$ .

# Clean up temporary files
rm -rf /tmp/para-auth-config-$$ /tmp/para-claude-config-$$ /tmp/para-claude-json-$$ /tmp/Dockerfile-auth-$$

echo ""
echo "✅ Successfully created authenticated image: $OUTPUT_IMAGE"
echo ""
echo "📝 Usage:"
echo "  para dispatch my-feature --container --docker-image $OUTPUT_IMAGE"
echo ""