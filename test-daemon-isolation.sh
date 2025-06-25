#\!/bin/bash
set -euo pipefail

PARA_BIN="/Users/marius.wichtner/Documents/git/para/.para/worktrees/docker-finish-agent-3/target/debug/para"

echo "=== Testing Daemon Multi-Repository Isolation ==="

# Clean up
$PARA_BIN daemon stop 2>/dev/null || true
rm -rf /tmp/test-repo-1 /tmp/test-repo-2 || true

# Create test repos
echo "1. Creating test repositories..."
mkdir -p /tmp/test-repo-1/.para/worktrees/container-1/.para
mkdir -p /tmp/test-repo-2/.para/worktrees/container-2/.para

# Initialize repos (minimal)
cd /tmp/test-repo-1
git init --quiet
echo "test" > .gitignore
git add .gitignore
git commit -m "init" --quiet

cd /tmp/test-repo-2  
git init --quiet
echo "test" > .gitignore
git add .gitignore
git commit -m "init" --quiet

# Start daemon
echo -e "\n2. Starting daemon..."
$PARA_BIN daemon start
sleep 1

# Test registration using the actual para code
echo -e "\n3. Testing daemon registration via Rust test program..."

cat > /tmp/test-registration.rs << 'RUST'
use std::path::Path;

fn main() {
    // This simulates what para does internally
    let result1 = register_session("container-1", "/tmp/test-repo-1/.para/worktrees/container-1");
    let result2 = register_session("container-2", "/tmp/test-repo-2/.para/worktrees/container-2");
    
    println\!("Registration 1: {:?}", result1);
    println\!("Registration 2: {:?}", result2);
}

fn register_session(name: &str, worktree: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;
    
    let repo_root = find_repo_root(Path::new(worktree))
        .map_err(|e| format\!("Failed to find repo root: {}", e))?;
    
    let command = format\!(
        r#"{{"RegisterContainerSession":{{"session_name":"{}","worktree_path":"{}","repo_root":"{}"}}}}"#,
        name, worktree, repo_root.display()
    );
    
    let mut stream = UnixStream::connect("/tmp/para-daemon.sock")
        .map_err(|e| format\!("Failed to connect: {}", e))?;
    
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    
    stream.write_all(command.as_bytes()).map_err(|e| format\!("Write failed: {}", e))?;
    stream.write_all(b"\n").map_err(|e| format\!("Write newline failed: {}", e))?;
    stream.flush().map_err(|e| format\!("Flush failed: {}", e))?;
    
    let mut response = String::new();
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(n) => {
            response.push_str(&String::from_utf8_lossy(&buffer[..n]));
            println\!("  Response: {}", response.trim());
            if response.contains("Ok") {
                Ok(())
            } else {
                Err(format\!("Bad response: {}", response))
            }
        }
        Err(e) => Err(format\!("Read failed: {}", e))
    }
}

fn find_repo_root(worktree_path: &Path) -> Result<std::path::PathBuf, String> {
    let mut current = worktree_path;
    
    // Walk up looking for .para/worktrees in the path
    while let Some(parent) = current.parent() {
        if parent.ends_with(".para/worktrees") {
            if let Some(para_dir) = parent.parent() {
                if let Some(repo_root) = para_dir.parent() {
                    return Ok(repo_root.to_path_buf());
                }
            }
        }
        current = parent;
    }
    
    // Fallback: look for .git
    current = worktree_path;
    while let Some(parent) = current.parent() {
        if parent.join(".git").exists() {
            return Ok(parent.to_path_buf());
        }
        current = parent;
    }
    
    Err("Could not find repository root".to_string())
}
RUST

rustc /tmp/test-registration.rs -o /tmp/test-registration 2>/dev/null || {
    echo "Failed to compile test program"
    exit 1
}

/tmp/test-registration

# Create cancel signals
echo -e "\n4. Creating cancel signals in both repos..."
echo '{"force": true}' > /tmp/test-repo-1/.para/worktrees/container-1/.para/cancel_signal.json
echo '{"force": true}' > /tmp/test-repo-2/.para/worktrees/container-2/.para/cancel_signal.json

# Wait for processing
echo "Waiting for signal processing..."
sleep 3

# Check results
echo -e "\n5. Checking signal processing results..."
if [ \! -f "/tmp/test-repo-1/.para/worktrees/container-1/.para/cancel_signal.json" ]; then
    echo "✅ Repo 1 signal processed correctly"
else
    echo "❌ Repo 1 signal NOT processed"
fi

if [ \! -f "/tmp/test-repo-2/.para/worktrees/container-2/.para/cancel_signal.json" ]; then
    echo "✅ Repo 2 signal processed correctly"  
else
    echo "❌ Repo 2 signal NOT processed"
fi

# Cleanup
echo -e "\n6. Cleaning up..."
$PARA_BIN daemon stop
rm -rf /tmp/test-repo-1 /tmp/test-repo-2 /tmp/test-registration*

echo -e "\n=== Test Complete ==="
