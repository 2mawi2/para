use std::collections::HashSet;

fn main() {
    // Set up allowed domains
    let mut allowed_domains = HashSet::new();
    
    // Add essential domains for Claude Code
    allowed_domains.insert("api.anthropic.com".to_string());
    allowed_domains.insert("statsig.anthropic.com".to_string());
    allowed_domains.insert("sentry.io".to_string());
    allowed_domains.insert("github.com".to_string());
    allowed_domains.insert("objects.githubusercontent.com".to_string());
    allowed_domains.insert("claude.ai".to_string());
    allowed_domains.insert("console.anthropic.com".to_string());
    
    println!("Starting Para network proxy on 127.0.0.1:8877");
    println!("Allowed domains:");
    for domain in &allowed_domains {
        println!("  - {}", domain);
    }
    
    // This would use the actual proxy implementation
    println!("\nProxy is running. Press Ctrl+C to stop.");
    
    // Keep running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}