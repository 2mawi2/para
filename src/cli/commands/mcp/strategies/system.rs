use super::{is_node_script, McpServerConfig, McpServerDetectionStrategy};
use std::path::PathBuf;

/// Strategy for detecting system-installed MCP servers
pub struct SystemDetectionStrategy;

impl McpServerDetectionStrategy for SystemDetectionStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let home_dir = directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~"));
        let local_server = home_dir.join(".local/bin/para-mcp-server");

        if !local_server.exists() {
            return None;
        }

        if is_node_script(&local_server) {
            // Try to find the original installation with dependencies
            let possible_source_locations = vec![
                directories::BaseDirs::new().map(|dirs| {
                    dirs.home_dir()
                        .join("Documents/git/para/mcp-server-ts/build/para-mcp-server.js")
                }),
                directories::BaseDirs::new().map(|dirs| {
                    dirs.home_dir()
                        .join("git/para/mcp-server-ts/build/para-mcp-server.js")
                }),
                directories::BaseDirs::new().map(|dirs| {
                    dirs.home_dir()
                        .join("repos/para/mcp-server-ts/build/para-mcp-server.js")
                }),
                directories::BaseDirs::new().map(|dirs| {
                    dirs.home_dir()
                        .join("projects/para/mcp-server-ts/build/para-mcp-server.js")
                }),
                Some(PathBuf::from(
                    "/opt/homebrew/opt/para/mcp-server-ts/build/para-mcp-server.js",
                )),
                Some(PathBuf::from(
                    "/usr/local/opt/para/mcp-server-ts/build/para-mcp-server.js",
                )),
            ];

            // Check if any of these locations have both the server and node_modules
            for source in possible_source_locations.into_iter().flatten() {
                if source.exists() {
                    if let Some(parent) = source.parent() {
                        if parent.join("../node_modules").exists() {
                            return Some(McpServerConfig {
                                command: "node".to_string(),
                                args: vec![source.to_string_lossy().to_string()],
                                description: "Para TypeScript MCP server with dependencies"
                                    .to_string(),
                            });
                        }
                    }
                }
            }

            // Use the installed one anyway (it might fail, but we'll provide a clear error)
            Some(McpServerConfig {
                command: "node".to_string(),
                args: vec![local_server.to_string_lossy().to_string()],
                description: "Local Node.js MCP server (may need dependencies)".to_string(),
            })
        } else {
            // Otherwise treat it as a binary
            Some(McpServerConfig {
                command: local_server.to_string_lossy().to_string(),
                args: vec![],
                description: "Local MCP server".to_string(),
            })
        }
    }

    fn description(&self) -> &str {
        "System installation MCP server detection"
    }
}
