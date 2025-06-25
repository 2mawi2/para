#\!/bin/bash
set -euo pipefail

PARA_BIN="/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para"

echo "=== Testing Multi-Repository Isolation (Simplified) ==="

# Clean up
rm -rf /tmp/test-repo-1 /tmp/test-repo-2 || true

# Create test repos
echo "1. Creating test repositories..."
mkdir -p /tmp/test-repo-1 /tmp/test-repo-2

cd /tmp/test-repo-1
git init
echo "# Repo 1" > README.md
git add README.md
git commit -m "Initial commit"

cd /tmp/test-repo-2
git init
echo "# Repo 2" > README.md
git add README.md
git commit -m "Initial commit"

# Start daemon
echo -e "\n2. Starting daemon..."
$PARA_BIN daemon start
sleep 1
$PARA_BIN daemon status

# Create regular sessions (not containers) to test the daemon registration
echo -e "\n3. Creating test sessions..."
cd /tmp/test-repo-1
$PARA_BIN start test-session-1

cd /tmp/test-repo-2
$PARA_BIN start test-session-2

# List sessions
echo -e "\n4. Sessions created:"
echo "Repo 1:"
cd /tmp/test-repo-1
$PARA_BIN list

echo -e "\nRepo 2:"
cd /tmp/test-repo-2
$PARA_BIN list

# Manually test daemon registration by creating a mock container session
echo -e "\n5. Testing daemon registration directly..."

# Create worktree structure for simulated container sessions
mkdir -p /tmp/test-repo-1/.para/worktrees/container-test-1/.para
mkdir -p /tmp/test-repo-2/.para/worktrees/container-test-2/.para

# Create a small test program to register with daemon
cat > /tmp/test-daemon-client.rs << 'RUST'
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
    let command = r#"{"RegisterContainerSession":{"session_name":"container-test-1","worktree_path":"/tmp/test-repo-1/.para/worktrees/container-test-1","repo_root":"/tmp/test-repo-1"}}"#;
    
    if let Ok(mut stream) = UnixStream::connect("/tmp/para-daemon.sock") {
        stream.write_all(command.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
        
        let mut response = String::new();
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer) {
            response.push_str(&String::from_utf8_lossy(&buffer[..n]));
            println!("Response: {}", response);
        }
    } else {
        println!("Failed to connect to daemon");
    }
}
RUST

# Compile and run test client
echo "Compiling test client..."
rustc /tmp/test-daemon-client.rs -o /tmp/test-daemon-client
echo "Registering container session with daemon..."
/tmp/test-daemon-client

# Create cancel signal
echo -e "\n6. Creating cancel signal..."
echo '{"force": true}' > /tmp/test-repo-1/.para/worktrees/container-test-1/.para/cancel_signal.json

# Wait for processing
echo "Waiting for signal processing..."
sleep 3

# Check if processed
if [ \! -f "/tmp/test-repo-1/.para/worktrees/container-test-1/.para/cancel_signal.json" ]; then
    echo "✅ Signal was processed\!"
else
    echo "❌ Signal was NOT processed"
    ls -la /tmp/test-repo-1/.para/worktrees/container-test-1/.para/
fi

# Cleanup
echo -e "\n7. Cleaning up..."
$PARA_BIN daemon stop
rm -rf /tmp/test-repo-1 /tmp/test-repo-2 /tmp/test-daemon-client*

echo -e "\n=== Test Complete ==="
