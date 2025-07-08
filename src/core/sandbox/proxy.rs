//! HTTP CONNECT proxy for network sandboxing
//!
//! This module implements a simple HTTP proxy that only allows HTTPS connections
//! to a predefined set of domains. It's designed to work with macOS sandbox-exec
//! to provide network isolation while still allowing necessary connections for
//! Claude Code and other development tools.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

/// Default port for the proxy server
pub const DEFAULT_PROXY_PORT: u16 = 8877;

/// Essential domains that are always allowed for Claude Code functionality
pub const ESSENTIAL_DOMAINS: &[&str] = &[
    "api.anthropic.com",
    "statsig.anthropic.com",
    "sentry.io",
    "github.com",
    "objects.githubusercontent.com",
    "claude.ai",
    "console.anthropic.com",
];

/// HTTP CONNECT proxy server that filters connections by domain
pub struct ProxyServer {
    allowed_domains: HashSet<String>,
    port: u16,
}

impl ProxyServer {
    /// Create a new proxy server with the given allowed domains
    pub fn new(allowed_domains: HashSet<String>, port: u16) -> Self {
        let mut domains = allowed_domains;

        // Always include essential domains
        for domain in ESSENTIAL_DOMAINS {
            domains.insert(domain.to_string());
        }

        Self {
            allowed_domains: domains,
            port,
        }
    }

    /// Start the proxy server in a background thread
    pub fn start(self) -> Result<ProxyHandle> {
        let listener = TcpListener::bind(("127.0.0.1", self.port))
            .with_context(|| format!("Failed to bind to port {}", self.port))?;

        let local_addr = listener.local_addr()?;
        let allowed_domains = Arc::new(self.allowed_domains);

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let domains = Arc::clone(&allowed_domains);
                        thread::spawn(move || {
                            if let Err(e) = handle_connection(stream, &domains) {
                                eprintln!("Proxy connection error: {e}");
                            }
                        });
                    }
                    Err(e) => eprintln!("Failed to accept connection: {e}"),
                }
            }
        });

        Ok(ProxyHandle {
            _thread: handle,
            address: local_addr.to_string(),
        })
    }
}

/// Handle to a running proxy server
pub struct ProxyHandle {
    _thread: thread::JoinHandle<()>,
    address: String,
}

impl ProxyHandle {
    /// Get the address the proxy is listening on
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Check if the proxy thread is still running
    #[cfg(test)]
    pub fn is_running(&self) -> bool {
        !self._thread.is_finished()
    }
}

/// Handle a single client connection
fn handle_connection(mut stream: TcpStream, allowed_domains: &HashSet<String>) -> Result<()> {
    // Set timeout for initial request
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();

    // Read the request line
    reader.read_line(&mut request_line)?;

    // Parse CONNECT request
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 3 || parts[0] != "CONNECT" {
        // Not a CONNECT request, reject
        stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")?;
        return Ok(());
    }

    // Extract host and port from "host:port"
    let host_port = parts[1];
    let host_parts: Vec<&str> = host_port.split(':').collect();
    if host_parts.len() != 2 {
        stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")?;
        return Ok(());
    }

    let host = host_parts[0];
    let port: u16 = host_parts[1].parse().unwrap_or(0);

    // Only allow HTTPS (port 443)
    if port != 443 {
        eprintln!("Proxy: Denying non-HTTPS connection to {host}:{port}");
        stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n")?;
        return Ok(());
    }

    // Check if domain is allowed
    if !is_domain_allowed(host, allowed_domains) {
        eprintln!("Proxy: Denying connection to blocked domain: {host}");
        stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n")?;
        return Ok(());
    }

    // Skip remaining headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.trim().is_empty() {
            break;
        }
    }

    // Try to connect to the target
    eprintln!("Proxy: Allowing connection to {host_port}");
    match TcpStream::connect(host_port) {
        Ok(mut target) => {
            // Send 200 OK response
            stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;

            // Set up bidirectional forwarding
            let mut stream_clone = stream.try_clone()?;
            let mut target_clone = target.try_clone()?;

            // Forward from client to target
            let handle1 = thread::spawn(move || {
                let _ = forward_data(&mut reader.into_inner(), &mut target);
            });

            // Forward from target to client
            let handle2 = thread::spawn(move || {
                let _ = forward_data(&mut target_clone, &mut stream_clone);
            });

            // Wait for either direction to finish
            let _ = handle1.join();
            let _ = handle2.join();
        }
        Err(e) => {
            eprintln!("Proxy: Failed to connect to {host_port}: {e}");
            stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n")?;
        }
    }

    Ok(())
}

/// Check if a domain is allowed
fn is_domain_allowed(host: &str, allowed_domains: &HashSet<String>) -> bool {
    // Direct match
    if allowed_domains.contains(host) {
        return true;
    }

    // Check if it's a subdomain of an allowed domain
    for allowed in allowed_domains {
        if host == *allowed || host.ends_with(&format!(".{allowed}")) {
            return true;
        }
    }

    false
}

/// Forward data between two streams
fn forward_data(from: &mut dyn Read, to: &mut dyn Write) -> Result<()> {
    let mut buffer = [0; 8192];
    loop {
        match from.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                to.write_all(&buffer[..n])?;
                to.flush()?;
            }
            Err(e) => {
                // Ignore common disconnection errors
                if e.kind() != std::io::ErrorKind::UnexpectedEof
                    && e.kind() != std::io::ErrorKind::BrokenPipe
                    && e.kind() != std::io::ErrorKind::ConnectionAborted
                {
                    return Err(e.into());
                }
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_allowed() {
        let mut allowed = HashSet::new();
        allowed.insert("example.com".to_string());
        allowed.insert("api.test.com".to_string());

        // Direct matches
        assert!(is_domain_allowed("example.com", &allowed));
        assert!(is_domain_allowed("api.test.com", &allowed));

        // Subdomains
        assert!(is_domain_allowed("sub.example.com", &allowed));
        assert!(is_domain_allowed("deep.sub.example.com", &allowed));

        // Not allowed
        assert!(!is_domain_allowed("notallowed.com", &allowed));
        assert!(!is_domain_allowed("example.org", &allowed));
        assert!(!is_domain_allowed("test.com", &allowed)); // api.test.com is allowed, not test.com
    }

    #[test]
    fn test_proxy_server_creation() {
        let mut domains = HashSet::new();
        domains.insert("custom.com".to_string());

        let proxy = ProxyServer::new(domains.clone(), 8877);

        // Check that essential domains are included
        assert!(proxy.allowed_domains.contains("api.anthropic.com"));
        assert!(proxy.allowed_domains.contains("github.com"));

        // Check that custom domain is included
        assert!(proxy.allowed_domains.contains("custom.com"));
    }
}

// Include additional proxy tests
#[cfg(test)]
#[path = "proxy_test.rs"]
mod proxy_test;
