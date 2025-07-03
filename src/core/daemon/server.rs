//! Para daemon server implementation

use super::{daemon_pid_path, daemon_socket_path, DaemonCommand, DaemonResponse};
use crate::config::ConfigManager;
use crate::core::docker::watcher::{SignalFileWatcher, WatcherHandle};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

/// The daemon server that manages all watchers
pub struct DaemonServer {
    /// Map of session_name -> (repo_root, watcher_handle)
    watchers: Arc<Mutex<HashMap<String, (PathBuf, WatcherHandle)>>>,
}

impl DaemonServer {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for DaemonServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonServer {
    /// Run the daemon server
    pub fn run(&self) -> anyhow::Result<()> {
        // Clean up any existing socket
        let socket_path = daemon_socket_path();
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        // Write PID file
        let pid = std::process::id();
        std::fs::write(daemon_pid_path(), pid.to_string())?;

        // Create Unix socket
        let listener = UnixListener::bind(&socket_path)?;
        println!("Para daemon started (PID: {pid})");

        // Handle incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let watchers = self.watchers.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_client(stream, watchers) {
                            eprintln!("Error handling client: {e}");
                        }
                    });
                }
                Err(e) => eprintln!("Error accepting connection: {e}"),
            }
        }

        Ok(())
    }
}

/// Handle a client connection
fn handle_client(
    stream: UnixStream,
    watchers: Arc<Mutex<HashMap<String, (PathBuf, WatcherHandle)>>>,
) -> anyhow::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;

    for line in reader.lines() {
        let line = line?;
        let command: DaemonCommand = serde_json::from_str(&line)?;

        let response = match command {
            DaemonCommand::RegisterContainerSession {
                session_name,
                worktree_path,
                repo_root,
            } => match register_watcher(&session_name, &worktree_path, &repo_root, &watchers) {
                Ok(()) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error(e.to_string()),
            },
            DaemonCommand::UnregisterSession { session_name } => {
                match unregister_watcher(&session_name, &watchers) {
                    Ok(()) => DaemonResponse::Ok,
                    Err(e) => DaemonResponse::Error(e.to_string()),
                }
            }
            DaemonCommand::Ping => DaemonResponse::Pong,
            DaemonCommand::Version => {
                DaemonResponse::Version(env!("CARGO_PKG_VERSION").to_string())
            }
            DaemonCommand::Shutdown => {
                // Clean up all watchers
                if let Ok(mut watchers_guard) = watchers.lock() {
                    watchers_guard.clear();
                }

                // Remove PID file
                let _ = std::fs::remove_file(daemon_pid_path());

                // Send response before exiting
                let response_json = serde_json::to_string(&DaemonResponse::Ok)?;
                stream.write_all(response_json.as_bytes())?;
                stream.write_all(b"\n")?;
                stream.flush()?;

                // Exit the process
                std::process::exit(0);
            }
        };

        // Send response
        let response_json = serde_json::to_string(&response)?;
        stream.write_all(response_json.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
    }

    Ok(())
}

/// Register a new watcher for a container session
fn register_watcher(
    session_name: &str,
    worktree_path: &Path,
    repo_root: &Path,
    watchers: &Arc<Mutex<HashMap<String, (PathBuf, WatcherHandle)>>>,
) -> anyhow::Result<()> {
    // Load config for this repository
    let config_path = repo_root.join(".para/config.json");
    let config = if config_path.exists() {
        // Load repo-specific config
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content)?
    } else {
        // Use default config
        ConfigManager::load_or_create()?
    };

    // Spawn watcher
    let watcher_handle = SignalFileWatcher::spawn(
        session_name.to_string(),
        worktree_path.to_path_buf(),
        config,
    );

    // Store watcher
    let mut watchers_guard = watchers.lock().unwrap();
    watchers_guard.insert(
        session_name.to_string(),
        (repo_root.to_path_buf(), watcher_handle),
    );

    println!(
        "Registered watcher for session: {} in repo: {}",
        session_name,
        repo_root.display()
    );
    Ok(())
}

/// Unregister and stop a watcher
fn unregister_watcher(
    session_name: &str,
    watchers: &Arc<Mutex<HashMap<String, (PathBuf, WatcherHandle)>>>,
) -> anyhow::Result<()> {
    let mut watchers_guard = watchers.lock().unwrap();

    if let Some((_, handle)) = watchers_guard.remove(session_name) {
        handle.stop()?;
        println!("Unregistered watcher for session: {session_name}");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Session not found: {}", session_name))
    }
}

/// Check if the daemon is already running
pub fn is_daemon_running() -> bool {
    let pid_path = daemon_pid_path();

    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Check if process is still alive
            // On Unix, we can use kill with signal 0
            unsafe { libc::kill(pid as i32, 0) == 0 }
        } else {
            false
        }
    } else {
        false
    }
}
