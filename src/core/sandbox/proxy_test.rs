#[cfg(test)]
mod tests {
    use crate::core::sandbox::proxy::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_proxy_server_start() {
        let mut allowed = HashSet::new();
        allowed.insert("example.com".to_string());

        let proxy = ProxyServer::new(allowed, 0); // Use port 0 for automatic assignment
        let handle = proxy.start().expect("Failed to start proxy");

        // Check that proxy is running
        assert!(handle.is_running());

        // Get the actual address
        let addr = handle.address();
        assert!(addr.contains("127.0.0.1:"));
    }

    #[test]
    fn test_proxy_allows_essential_domains() {
        let proxy = ProxyServer::new(HashSet::new(), 0);

        // Essential domains should always be included
        assert!(proxy.allowed_domains.contains("api.anthropic.com"));
        assert!(proxy.allowed_domains.contains("github.com"));
        assert!(proxy.allowed_domains.contains("statsig.anthropic.com"));
    }

    #[test]
    fn test_proxy_custom_domains() {
        let mut custom = HashSet::new();
        custom.insert("custom.example.com".to_string());
        custom.insert("api.mycompany.com".to_string());

        let proxy = ProxyServer::new(custom, 0);

        // Check custom domains are included
        assert!(proxy.allowed_domains.contains("custom.example.com"));
        assert!(proxy.allowed_domains.contains("api.mycompany.com"));

        // Essential domains should still be included
        assert!(proxy.allowed_domains.contains("api.anthropic.com"));
    }

    #[test]
    fn test_connect_request_handling() {
        let mut allowed = HashSet::new();
        allowed.insert("example.com".to_string());

        let proxy = ProxyServer::new(allowed, 0);
        let handle = proxy.start().expect("Failed to start proxy");

        // Parse the port from the address
        let addr = handle.address();
        let port: u16 = addr.split(':').nth(1).unwrap().parse().unwrap();

        // Give the proxy time to start
        thread::sleep(Duration::from_millis(100));

        // Try to connect to the proxy
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("Failed to connect to proxy");

        // Send a CONNECT request for an allowed domain
        stream
            .write_all(b"CONNECT example.com:443 HTTP/1.1\r\n\r\n")
            .expect("Failed to send request");

        // Read response
        let mut response = vec![0; 1024];
        let n = stream.read(&mut response).expect("Failed to read response");
        let response_str = String::from_utf8_lossy(&response[..n]);

        // Since we can't actually connect to example.com in tests,
        // we expect either 200 (if connection succeeds) or 502 (if it fails)
        assert!(response_str.contains("HTTP/1.1 502") || response_str.contains("HTTP/1.1 200"));
    }

    #[test]
    fn test_blocked_domain() {
        let allowed = HashSet::new(); // No custom domains, only essentials

        let proxy = ProxyServer::new(allowed, 0);
        let handle = proxy.start().expect("Failed to start proxy");

        // Parse the port from the address
        let addr = handle.address();
        let port: u16 = addr.split(':').nth(1).unwrap().parse().unwrap();

        // Give the proxy time to start
        thread::sleep(Duration::from_millis(100));

        // Try to connect to the proxy
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("Failed to connect to proxy");

        // Send a CONNECT request for a blocked domain
        stream
            .write_all(b"CONNECT malicious.com:443 HTTP/1.1\r\n\r\n")
            .expect("Failed to send request");

        // Read response
        let mut response = vec![0; 1024];
        let n = stream.read(&mut response).expect("Failed to read response");
        let response_str = String::from_utf8_lossy(&response[..n]);

        // Should get 403 Forbidden
        assert!(response_str.contains("HTTP/1.1 403"));
    }

    #[test]
    fn test_non_https_port_blocked() {
        let mut allowed = HashSet::new();
        allowed.insert("example.com".to_string());

        let proxy = ProxyServer::new(allowed, 0);
        let handle = proxy.start().expect("Failed to start proxy");

        // Parse the port from the address
        let addr = handle.address();
        let port: u16 = addr.split(':').nth(1).unwrap().parse().unwrap();

        // Give the proxy time to start
        thread::sleep(Duration::from_millis(100));

        // Try to connect to the proxy
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("Failed to connect to proxy");

        // Send a CONNECT request for non-HTTPS port
        stream
            .write_all(b"CONNECT example.com:80 HTTP/1.1\r\n\r\n")
            .expect("Failed to send request");

        // Read response
        let mut response = vec![0; 1024];
        let n = stream.read(&mut response).expect("Failed to read response");
        let response_str = String::from_utf8_lossy(&response[..n]);

        // Should get 403 Forbidden
        assert!(response_str.contains("HTTP/1.1 403"));
    }
}
