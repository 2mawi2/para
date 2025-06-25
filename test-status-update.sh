#\!/bin/bash
set -euo pipefail

PARA_BIN="/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para"

echo "=== Testing Container Status Update ==="

# Clean up
$PARA_BIN daemon stop 2>/dev/null || true
rm -rf /tmp/test-status-repo || true
sleep 1

# Create test repo
echo "1. Creating test repository..."
mkdir -p /tmp/test-status-repo
cd /tmp/test-status-repo
git init --quiet
echo "test" > README.md
git add README.md
git commit -m "init" --quiet

# Start daemon
echo -e "\n2. Starting daemon..."
$PARA_BIN daemon start
sleep 1

# Create a session
echo -e "\n3. Creating test session..."
$PARA_BIN start status-test-session

# Get session info
SESSION_PATH="/tmp/test-status-repo/.para/worktrees/status-test-session"
STATE_DIR="/tmp/test-status-repo/.para/state"

# Create .para directory in the worktree
mkdir -p "$SESSION_PATH/.para"

# Manually register with daemon (simulating container)
echo -e "\n4. Simulating container registration..."
cat > /tmp/register-session.rs << 'RUST'
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
    let command = r#"{"RegisterContainerSession":{"session_name":"status-test-session","worktree_path":"/tmp/test-status-repo/.para/worktrees/status-test-session","repo_root":"/tmp/test-status-repo"}}"#;
    
    if let Ok(mut stream) = UnixStream::connect("/tmp/para-daemon.sock") {
        stream.write_all(command.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
        
        let mut response = String::new();
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer) {
            response.push_str(&String::from_utf8_lossy(&buffer[..n]));
            println\!("Registration response: {}", response.trim());
        }
    }
}
RUST

if rustc /tmp/register-session.rs -o /tmp/register-session 2>/dev/null; then
    /tmp/register-session
else
    echo "Failed to compile registration program"
fi

# Create status update signal
echo -e "\n5. Creating status update signal..."
cat > "$SESSION_PATH/.para/status.json" << JSON
{
  "task": "Implementing authentication",
  "tests": "failed",
  "confidence": "high",
  "todos": "3/5",
  "blocked": false,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
JSON

echo "Status signal created:"
cat "$SESSION_PATH/.para/status.json"

# Wait for processing
echo -e "\n6. Waiting for daemon to process status..."
sleep 2

# Check if status was saved
echo -e "\n7. Checking saved status..."
STATUS_FILE="$STATE_DIR/status-test-session.status.json"

if [ -f "$STATUS_FILE" ]; then
    echo "✅ Status file created successfully\!"
    echo "Content:"
    cat "$STATUS_FILE" | jq '.' || cat "$STATUS_FILE"
else
    echo "❌ Status file not found at: $STATUS_FILE"
    echo "Checking state directory:"
    ls -la "$STATE_DIR" | grep status || echo "No status files found"
fi

# Try another status update
echo -e "\n8. Updating status again..."
cat > "$SESSION_PATH/.para/status.json" << JSON
{
  "task": "Fixed authentication tests",
  "tests": "passed",
  "confidence": "high",
  "todos": "5/5",
  "blocked": false,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
JSON

sleep 2

echo -e "\n9. Final status check..."
if [ -f "$STATUS_FILE" ]; then
    echo "Updated content:"
    cat "$STATUS_FILE" | jq '.' || cat "$STATUS_FILE"
fi

# Cleanup
echo -e "\n10. Cleaning up..."
$PARA_BIN daemon stop
rm -rf /tmp/test-status-repo /tmp/register-session*

echo -e "\n=== Test Complete ==="
