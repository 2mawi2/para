#!/bin/bash
# Test script to demonstrate container pool limit enforcement

echo "=== Testing Container Pool Limit (max: 3) ==="
echo

# Try to create 4 containers to test the limit
for i in 1 2 3 4; do
    echo "Attempting to create container #$i..."
    if cargo run -- start --container "test-pool-$i" 2>&1 | grep -E "(Container pool status|pool exhausted|Created session)"; then
        echo "✓ Container #$i created"
    else
        echo "✗ Failed to create container #$i"
    fi
    echo
done

echo "=== Current Para Containers ==="
docker ps --filter "name=^para-" --format "table {{.Names}}\t{{.Status}}"

echo
echo "=== Cleanup ==="
echo "Run 'para clean' to remove all test containers"