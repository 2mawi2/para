#!/bin/bash
# Test Docker MVP flow

set -e

echo "=== Testing Para Docker MVP ==="

# Build para
echo "Building para..."
cargo build --release

# Check if Docker is available
echo "Checking Docker..."
docker version > /dev/null || { echo "Docker not available"; exit 1; }

# Create a test directory
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

# Initialize git repo
echo "Setting up test repo..."
git init
git config user.name "Test User"
git config user.email "test@example.com"
echo "# Test Project" > README.md
git add README.md
git commit -m "Initial commit"

# Enable Docker in config
echo "Configuring para for Docker..."
mkdir -p ~/.config/para
cat > ~/.config/para/config.json <<EOF
{
  "ide": {
    "name": "code",
    "command": "code",
    "user_data_dir": null,
    "wrapper": {
      "enabled": false,
      "name": "",
      "command": ""
    }
  },
  "directories": {
    "subtrees_dir": ".para/worktrees",
    "state_dir": ".para/state"
  },
  "git": {
    "branch_prefix": "para",
    "auto_stage": true,
    "auto_commit": true
  },
  "session": {
    "default_name_format": "%Y%m%d-%H%M%S",
    "preserve_on_finish": false,
    "auto_cleanup_days": 30
  },
  "docker": {
    "enabled": true,
    "default_image": "ubuntu:latest",
    "mount_workspace": true
  }
}
EOF

# Test 1: Start container session
echo -e "\n=== Test 1: Start container session ==="
~/Documents/git/para/target/release/para start --container test-docker || true

# Check if container is running
echo "Checking container status..."
docker ps | grep "para-test-docker" || echo "Container not found"

# Test 2: Dispatch with container
echo -e "\n=== Test 2: Dispatch with container ==="
echo "test task" | ~/Documents/git/para/target/release/para dispatch --container test-dispatch || true

# Check devcontainer.json
echo "Checking devcontainer.json..."
ls -la .para/worktrees/test-dispatch/.devcontainer/ || true
cat .para/worktrees/test-dispatch/.devcontainer/devcontainer.json || true

# Test 3: Finish container session
echo -e "\n=== Test 3: Finish from container session ==="
cd .para/worktrees/test-dispatch || true
~/Documents/git/para/target/release/para finish "Test commit from container" || true

# Cleanup
echo -e "\n=== Cleanup ==="
docker stop para-test-docker para-test-dispatch 2>/dev/null || true
docker rm para-test-docker para-test-dispatch 2>/dev/null || true
cd /
rm -rf "$TEST_DIR"

echo -e "\n=== Docker MVP test complete ==="