//! Daemon command implementation

use crate::cli::parser::{DaemonArgs, DaemonCommands};
use crate::core::daemon::server::{is_daemon_running, DaemonServer};
use crate::core::daemon::{daemon_pid_path, daemon_socket_path, DaemonCommand, DaemonResponse};
use crate::utils::Result;
use std::io::Write;
use std::os::unix::net::UnixStream;

pub fn execute(args: DaemonArgs) -> Result<()> {
    match args.command {
        DaemonCommands::Start => start_daemon(),
        DaemonCommands::Stop => stop_daemon(),
        DaemonCommands::Status => check_status(),
    }
}

fn start_daemon() -> Result<()> {
    // Check if already running
    if is_daemon_running() {
        println!("Para daemon is already running");
        return Ok(());
    }

    // Fork to create daemon process
    match unsafe { libc::fork() } {
        -1 => Err(crate::utils::ParaError::worktree_operation(
            "Failed to fork process",
        )),
        0 => {
            // Child process - become the daemon

            // Create new session
            unsafe {
                libc::setsid();
            }

            // Close standard file descriptors
            unsafe {
                libc::close(0);
                libc::close(1);
                libc::close(2);
            }

            // Run the daemon server
            let server = DaemonServer::new();
            if let Err(e) = server.run() {
                eprintln!("Daemon error: {e}");
                std::process::exit(1);
            }

            std::process::exit(0);
        }
        _ => {
            // Parent process
            println!("Para daemon started");
            Ok(())
        }
    }
}

fn stop_daemon() -> Result<()> {
    if !is_daemon_running() {
        println!("Para daemon is not running");
        return Ok(());
    }

    // Send shutdown command
    match send_daemon_command(&DaemonCommand::Shutdown) {
        Ok(DaemonResponse::Ok) => {
            println!("Para daemon stopped");
            Ok(())
        }
        _ => Err(crate::utils::ParaError::worktree_operation(
            "Failed to stop daemon",
        )),
    }
}

fn check_status() -> Result<()> {
    if is_daemon_running() {
        // Try to ping
        match send_daemon_command(&DaemonCommand::Ping) {
            Ok(DaemonResponse::Pong) => {
                println!("Para daemon is running");

                // Show PID
                if let Ok(pid_str) = std::fs::read_to_string(daemon_pid_path()) {
                    println!("PID: {}", pid_str.trim());
                }

                // Show socket path
                println!("Socket: {}", daemon_socket_path().display());
            }
            _ => {
                println!("Para daemon is running but not responding");
            }
        }
    } else {
        println!("Para daemon is not running");
    }

    Ok(())
}

fn send_daemon_command(command: &DaemonCommand) -> Result<DaemonResponse> {
    let socket_path = daemon_socket_path();
    let mut stream = UnixStream::connect(&socket_path)?;

    // Send command
    let command_json = serde_json::to_string(command)?;
    stream.write_all(command_json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // Read response with timeout
    use std::io::Read;
    use std::time::Duration;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut response = String::new();
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(n) => response.push_str(&String::from_utf8_lossy(&buffer[..n])),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            return Err(crate::utils::ParaError::worktree_operation(
                "Daemon not responding",
            ));
        }
        Err(e) => return Err(e.into()),
    }

    // Parse response
    let response: DaemonResponse = serde_json::from_str(response.trim()).map_err(|e| {
        crate::utils::ParaError::worktree_operation(format!("Failed to parse daemon response: {e}"))
    })?;
    Ok(response)
}
