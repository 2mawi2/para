use super::{McpServerConfig, McpServerDetectionStrategy};

/// Strategy for detecting local development MCP servers
pub struct DevelopmentDetectionStrategy;

impl McpServerDetectionStrategy for DevelopmentDetectionStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let current_dir = std::env::current_dir().ok()?;
        let local_ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

        if local_ts_server.exists() {
            Some(McpServerConfig {
                command: "node".to_string(),
                args: vec![local_ts_server.to_string_lossy().to_string()],
                description: "Local development TypeScript MCP server".to_string(),
            })
        } else {
            None
        }
    }

    fn description(&self) -> &str {
        "Local development MCP server detection"
    }
}
