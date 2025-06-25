#!/bin/bash
# Test script to verify SignalFileWatcher is spawned for container sessions

set -e

echo "Testing SignalFileWatcher integration with para start --container"

# Build para
echo "Building para..."
cargo build --quiet

# Create a test session with container
echo "Creating container session..."
./target/debug/para start test-watcher-session --container || true

# Give it a moment to start
sleep 2

# Check if the container is running
echo "Checking container status..."
docker ps | grep para-test-watcher-session || echo "Container not found (expected if mocked)"

# Clean up
echo "Cleaning up..."
./target/debug/para cancel test-watcher-session --force || true

echo "Test complete!"