//! Para daemon for managing background tasks like signal file watchers
//!
//! The daemon runs as a single process and manages watchers for all repositories.
//! It uses Unix domain sockets for IPC.

pub mod client;
pub mod server;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Commands that can be sent to the daemon
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    /// Register a new container session to watch
    RegisterContainerSession {
        session_name: String,
        worktree_path: PathBuf,
        repo_root: PathBuf,
    },
    /// Stop watching a specific session
    UnregisterSession { session_name: String },
    /// Check if daemon is alive
    Ping,
    /// Shutdown the daemon
    Shutdown,
}

/// Response from the daemon
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    Ok,
    Error(String),
    Pong,
}

/// Get the path to the daemon socket
pub fn daemon_socket_path() -> PathBuf {
    // Use a global location that works across all repos
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Fallback for macOS and other systems
            PathBuf::from("/tmp")
        });

    runtime_dir.join("para-daemon.sock")
}

/// Get the path to the daemon PID file
pub fn daemon_pid_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    runtime_dir.join("para-daemon.pid")
}
