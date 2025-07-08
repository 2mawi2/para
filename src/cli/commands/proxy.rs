use crate::core::sandbox::proxy::{ProxyServer, ESSENTIAL_DOMAINS};
use crate::utils::Result;
use std::collections::HashSet;

/// Run the network proxy standalone for testing
pub fn execute(port: u16, allowed_domains: Vec<String>) -> Result<()> {
    // Set up allowed domains
    let mut domains = HashSet::new();

    // Add user-specified domains (filter out empty strings)
    for domain in allowed_domains {
        if !domain.is_empty() {
            domains.insert(domain);
        }
    }

    // Essential domains are always added by ProxyServer
    println!("🌐 Starting Para network proxy on 127.0.0.1:{port}");
    println!("\nEssential domains (always allowed):");
    for domain in ESSENTIAL_DOMAINS {
        println!("  ✓ {domain}");
    }

    if !domains.is_empty() {
        println!("\nAdditional allowed domains:");
        for domain in &domains {
            println!("  ✓ {domain}");
        }
    }

    let proxy = ProxyServer::new(domains, port);
    let handle = proxy
        .start()
        .map_err(|e| crate::utils::ParaError::proxy_error(e.to_string()))?;

    let addr = handle.address();
    println!("\n✅ Proxy started on {addr}");
    println!("📋 Export these environment variables:");
    println!("   export HTTP_PROXY=http://{addr}");
    println!("   export HTTPS_PROXY=http://{addr}");
    println!("\nPress Ctrl+C to stop the proxy...");

    // Keep the main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
