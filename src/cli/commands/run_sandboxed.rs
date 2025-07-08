use crate::core::sandbox::proxy::{ProxyServer, ESSENTIAL_DOMAINS};
use crate::Result;
use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Run a command with network sandboxing via proxy
/// This is used internally by the VS Code task to start Claude with the proxy
pub fn execute(
    command: Vec<String>,
    port: u16,
    allowed_domains: Vec<String>,
    sandbox_profile: String,
    worktree_path: String,
    temp_dir: String,
    home_dir: String,
    cache_dir: String,
) -> Result<()> {
    if command.is_empty() {
        return Err(anyhow::anyhow!("No command specified"));
    }

    // Set up allowed domains
    let mut domains = HashSet::new();
    for domain in allowed_domains {
        domains.insert(domain);
    }

    println!("🌐 Starting network proxy on 127.0.0.1:{}", port);
    println!("📋 Essential domains:");
    for domain in ESSENTIAL_DOMAINS {
        println!("   ✓ {}", domain);
    }

    if !domains.is_empty() {
        println!("📋 Additional allowed domains:");
        for domain in &domains {
            println!("   ✓ {}", domain);
        }
    }

    // Start the proxy
    let proxy = ProxyServer::new(domains, port);
    let proxy_handle = proxy.start()?;
    let proxy_addr = proxy_handle.address().to_string();
    println!("✅ Proxy started on {}", proxy_addr);

    // Wait for proxy to be ready
    println!("⏳ Waiting for proxy to be ready...");
    for _ in 0..10 {
        match std::net::TcpStream::connect(&proxy_addr) {
            Ok(_) => {
                println!("✅ Proxy is ready");
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }

    // Set up the sandboxed command
    let mut sandbox_cmd = Command::new("sandbox-exec");
    sandbox_cmd
        .arg("-D")
        .arg(format!("TARGET_DIR={}", worktree_path))
        .arg("-D")
        .arg(format!("TMP_DIR={}", temp_dir))
        .arg("-D")
        .arg(format!("HOME_DIR={}", home_dir))
        .arg("-D")
        .arg(format!("CACHE_DIR={}", cache_dir))
        .arg("-D")
        .arg(format!("PROXY_ADDR={}", proxy_addr))
        .arg("-f")
        .arg(sandbox_profile)
        .arg("sh")
        .arg("-c")
        .arg(format!(
            "export HTTP_PROXY=http://{}; export HTTPS_PROXY=http://{}; {}",
            proxy_addr,
            proxy_addr,
            command.join(" ")
        ));

    println!("🔒 Starting sandboxed command: {}", command.join(" "));

    // Create a channel to communicate between threads
    let (tx, rx) = std::sync::mpsc::channel();
    let process_handle = Arc::new(Mutex::new(None));
    let process_handle_clone = Arc::clone(&process_handle);

    // Set up signal handlers
    ctrlc::set_handler(move || {
        println!("\n🛑 Received interrupt signal, cleaning up...");
        if let Ok(mut handle) = process_handle_clone.lock() {
            if let Some(ref mut child) = *handle {
                let _ = child.kill();
            }
        }
        std::process::exit(0);
    })?;

    // Spawn the sandboxed process
    let mut child = sandbox_cmd.spawn()?;
    *process_handle.lock().unwrap() = Some(child.try_clone()?);

    // Monitor proxy health in a separate thread
    let proxy_monitor = thread::spawn(move || {
        loop {
            if !proxy_handle.is_running() {
                println!("❌ Proxy stopped unexpectedly");
                let _ = tx.send(());
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    // Wait for the child process to finish
    let status = child.wait()?;

    // Clean up
    drop(proxy_monitor);

    if status.success() {
        println!("✅ Command completed successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Command failed with exit code: {:?}",
            status.code()
        ))
    }
}