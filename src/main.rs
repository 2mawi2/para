mod cli;
mod config;
mod core;
mod platform;
mod ui;
mod utils;

#[cfg(test)]
mod test_utils;

use clap::Parser;
use cli::{execute_command, Cli};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

fn setup_cleanup_handler() {
    if CLEANUP_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let cleanup_flag = Arc::new(AtomicBool::new(false));
        let cleanup_flag_clone = cleanup_flag.clone();

        ctrlc::set_handler(move || {
            if cleanup_flag_clone.load(Ordering::SeqCst) {
                // Already cleaning up, force exit
                std::process::exit(1);
            }
            cleanup_flag_clone.store(true, Ordering::SeqCst);
            cleanup_docker_containers();
            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");
    }
}

fn cleanup_docker_containers() {
    // CLI-only approach: always attempt cleanup regardless of config
    use std::process::Command;

    // Find and stop all para pool containers
    if let Ok(output) = Command::new("docker")
        .args(["ps", "-a", "--filter", "name=para-pool-", "-q"])
        .output()
    {
        let container_ids = String::from_utf8_lossy(&output.stdout);
        for id in container_ids.lines() {
            if !id.trim().is_empty() {
                let _ = Command::new("docker").args(["stop", id.trim()]).output();
                let _ = Command::new("docker").args(["rm", id.trim()]).output();
            }
        }
    }
}

fn main() {
    setup_cleanup_handler();

    let cli = Cli::parse();

    if let Err(e) = execute_command(cli) {
        eprintln!("para: {e}");
        std::process::exit(1);
    }
}
