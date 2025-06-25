#\!/bin/bash
set -euo pipefail

echo "=== Testing Multi-Repository Isolation ==="

# Clean up any existing test directories
rm -rf /tmp/test-repo-1 /tmp/test-repo-2 || true

# Create two test repositories
echo "1. Creating test repositories..."
mkdir -p /tmp/test-repo-1 /tmp/test-repo-2

# Initialize first repository
cd /tmp/test-repo-1
git init
echo "# Test Repo 1" > README.md
git add README.md
git commit -m "Initial commit repo 1"

# Initialize second repository  
cd /tmp/test-repo-2
git init
echo "# Test Repo 2" > README.md
git add README.md
git commit -m "Initial commit repo 2"

# Check daemon status
echo -e "\n2. Checking daemon status..."
/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para daemon status

# If daemon not running, start it
if \! /Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para daemon status | grep -q "running"; then
    echo "Starting daemon..."
    /Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para daemon start
    sleep 1
fi

# Create container sessions in both repositories
echo -e "\n3. Creating container sessions in both repositories..."

# Session in repo 1
cd /tmp/test-repo-1
echo "Creating session 'multi-test-1' in repo 1..."
/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para start multi-test-1 --container || echo "Failed to start session in repo 1"

# Session in repo 2
cd /tmp/test-repo-2
echo "Creating session 'multi-test-2' in repo 2..."
/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para start multi-test-2 --container || echo "Failed to start session in repo 2"

# List sessions in each repo
echo -e "\n4. Listing sessions in each repository..."
echo "Sessions in repo 1:"
cd /tmp/test-repo-1
/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para list

echo -e "\nSessions in repo 2:"
cd /tmp/test-repo-2
/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para list

# Test signal processing from each container
echo -e "\n5. Testing signal processing..."

# Create signal files to simulate container operations
echo "Creating cancel signal in repo 1 session..."
mkdir -p /tmp/test-repo-1/.para/worktrees/multi-test-1/.para
echo '{"force": true}' > /tmp/test-repo-1/.para/worktrees/multi-test-1/.para/cancel_signal.json

echo "Creating cancel signal in repo 2 session..."
mkdir -p /tmp/test-repo-2/.para/worktrees/multi-test-2/.para
echo '{"force": true}' > /tmp/test-repo-2/.para/worktrees/multi-test-2/.para/cancel_signal.json

# Wait for signals to be processed
echo "Waiting for signals to be processed..."
sleep 3

# Check if signal files were removed (indicating they were processed)
echo -e "\n6. Verifying signal processing..."
if [ \! -f "/tmp/test-repo-1/.para/worktrees/multi-test-1/.para/cancel_signal.json" ]; then
    echo "✅ Repo 1 signal processed correctly"
else
    echo "❌ Repo 1 signal NOT processed"
fi

if [ \! -f "/tmp/test-repo-2/.para/worktrees/multi-test-2/.para/cancel_signal.json" ]; then
    echo "✅ Repo 2 signal processed correctly"
else
    echo "❌ Repo 2 signal NOT processed"
fi

# Verify sessions were cancelled
echo -e "\n7. Verifying sessions were cancelled..."
cd /tmp/test-repo-1
if /Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para list | grep -q "multi-test-1"; then
    echo "❌ Session multi-test-1 still exists (should have been cancelled)"
else
    echo "✅ Session multi-test-1 cancelled successfully"
fi

cd /tmp/test-repo-2
if /Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para list | grep -q "multi-test-2"; then
    echo "❌ Session multi-test-2 still exists (should have been cancelled)"
else
    echo "✅ Session multi-test-2 cancelled successfully"
fi

# Cleanup
echo -e "\n8. Cleaning up..."
rm -rf /tmp/test-repo-1 /tmp/test-repo-2

echo -e "\n=== Multi-Repository Isolation Test Complete ==="
