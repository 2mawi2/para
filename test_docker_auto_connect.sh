#!/bin/bash
# Test Docker automatic VS Code connection

set -e

echo "=== Testing Para Docker Automatic VS Code Connection ==="

# Ensure config is set up for VS Code
echo "Setting up VS Code configuration..."
mkdir -p ~/.config/para
cat > ~/.config/para/config.json <<EOF
{
  "ide": {
    "name": "code",
    "command": "code",
    "user_data_dir": null,
    "wrapper": {
      "enabled": true,
      "name": "code",
      "command": "code"
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

# Test dispatch with container (includes automatic connection)
echo -e "\n=== Test: Dispatch with automatic container connection ==="
echo "Implement a hello world function" | para dispatch --container test-auto-connect

echo -e "\n=== Checking container status ==="
docker ps | grep para-test-auto-connect || echo "Container not found"

echo -e "\n=== Checking VS Code tasks ==="
ls -la .para/worktrees/test-auto-connect/.vscode/tasks.json || echo "Tasks file not found"
cat .para/worktrees/test-auto-connect/.vscode/tasks.json || echo "Could not read tasks file"

echo -e "\n=== Test completed ==="
echo "If VS Code opened automatically and connected to the container, the test was successful!"
echo "The VS Code window should show:"
echo "  - Connected to container 'para-test-auto-connect'"
echo "  - Workspace: /workspace"
echo "  - Claude command should run automatically in the terminal"

# Cleanup
echo -e "\n=== Cleanup (after 10 seconds) ==="
sleep 10
docker stop para-test-auto-connect 2>/dev/null || true
docker rm para-test-auto-connect 2>/dev/null || true
para cancel test-auto-connect --force || true

echo "Done!"