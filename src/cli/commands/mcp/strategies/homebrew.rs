use super::{is_node_script, McpServerConfig, McpServerDetectionStrategy};
use std::path::PathBuf;

/// Strategy for detecting Homebrew-installed MCP servers
pub struct HomebrewDetectionStrategy;

impl McpServerDetectionStrategy for HomebrewDetectionStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let homebrew_locations = vec![
            "/opt/homebrew/bin/para-mcp-server",              // Apple Silicon
            "/usr/local/bin/para-mcp-server",                 // Intel Mac
            "/home/linuxbrew/.linuxbrew/bin/para-mcp-server", // Linux
        ];

        for location in homebrew_locations {
            let path = PathBuf::from(location);
            if path.exists() {
                if is_node_script(&path) {
                    return Some(McpServerConfig {
                        command: "node".to_string(),
                        args: vec![path.to_string_lossy().to_string()],
                        description: "Homebrew Node.js MCP server".to_string(),
                    });
                } else {
                    return Some(McpServerConfig {
                        command: path.to_string_lossy().to_string(),
                        args: vec![],
                        description: "Homebrew MCP server".to_string(),
                    });
                }
            }
        }
        None
    }

    fn description(&self) -> &str {
        "Homebrew MCP server detection"
    }
}
