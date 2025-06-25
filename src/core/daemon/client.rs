//! Client for communicating with the para daemon

use super::{daemon_socket_path, DaemonCommand, DaemonResponse};
use crate::config::Config;
use anyhow::Result;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Register a container session with the daemon
pub fn register_container_session(
    session_name: &str,
    worktree_path: &Path,
    _config: &Config,
) -> Result<()> {
    // Get the repo root from the worktree path
    let repo_root = find_repo_root(worktree_path)?;

    let command = DaemonCommand::RegisterContainerSession {
        session_name: session_name.to_string(),
        worktree_path: worktree_path.to_path_buf(),
        repo_root,
    };

    // Try to send command, start daemon if needed
    match send_command(&command) {
        Ok(DaemonResponse::Ok) => Ok(()),
        Ok(DaemonResponse::Error(e)) => Err(anyhow::anyhow!("Daemon error: {}", e)),
        Ok(_) => Err(anyhow::anyhow!("Unexpected daemon response")),
        Err(_) => {
            // Daemon not running, try to start it
            start_daemon_if_needed()?;

            // Retry sending command
            match send_command(&command) {
                Ok(DaemonResponse::Ok) => Ok(()),
                Ok(DaemonResponse::Error(e)) => Err(anyhow::anyhow!("Daemon error: {}", e)),
                _ => Err(anyhow::anyhow!("Failed to communicate with daemon")),
            }
        }
    }
}

/// Send a command to the daemon
fn send_command(command: &DaemonCommand) -> Result<DaemonResponse> {
    let socket_path = daemon_socket_path();
    let mut stream = UnixStream::connect(&socket_path)?;

    // Set timeout
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // Send command
    let command_json = serde_json::to_string(command)?;
    stream.write_all(command_json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // Read response
    let mut response = String::new();
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => response.push_str(&String::from_utf8_lossy(&buffer[..n])),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e.into()),
        }
    }

    // Parse response
    let response: DaemonResponse = serde_json::from_str(response.trim())?;
    Ok(response)
}

/// Start the daemon if it's not already running
fn start_daemon_if_needed() -> Result<()> {
    // Check if daemon is already running
    if let Ok(DaemonResponse::Pong) = send_command(&DaemonCommand::Ping) {
        return Ok(());
    }

    // Get the current executable path
    let exe_path = std::env::current_exe()?;

    // Start daemon as a detached process
    Command::new(&exe_path)
        .arg("daemon")
        .arg("start")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Wait a bit for daemon to start
    std::thread::sleep(Duration::from_millis(500));

    // Verify daemon started
    match send_command(&DaemonCommand::Ping) {
        Ok(DaemonResponse::Pong) => Ok(()),
        _ => Err(anyhow::anyhow!("Failed to start daemon")),
    }
}

/// Find the git repository root from a worktree path
fn find_repo_root(worktree_path: &Path) -> Result<std::path::PathBuf> {
    // For para worktrees, the structure is:
    // <repo_root>/.para/worktrees/<session>/

    let mut current = worktree_path;

    // Walk up the directory tree looking for .para/worktrees in the path
    while let Some(parent) = current.parent() {
        if parent.ends_with(".para/worktrees") {
            // Found it, go up two more levels to get repo root
            if let Some(para_dir) = parent.parent() {
                if let Some(repo_root) = para_dir.parent() {
                    return Ok(repo_root.to_path_buf());
                }
            }
        }
        current = parent;
    }

    // Fallback: look for .git directory
    current = worktree_path;
    while let Some(parent) = current.parent() {
        if parent.join(".git").exists() {
            return Ok(parent.to_path_buf());
        }
        current = parent;
    }

    Err(anyhow::anyhow!("Could not find repository root"))
}
