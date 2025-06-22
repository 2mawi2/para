use super::{is_node_script, McpServerConfig, McpServerDetectionStrategy};
use std::path::PathBuf;
use std::process::Command;

/// Strategy for detecting MCP servers in system PATH
pub struct PathDetectionStrategy;

impl McpServerDetectionStrategy for PathDetectionStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        if let Ok(output) = Command::new("which").arg("para-mcp-server").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let path = PathBuf::from(&path_str);

                if is_node_script(&path) {
                    Some(McpServerConfig {
                        command: "node".to_string(),
                        args: vec![path_str],
                        description: "System PATH Node.js MCP server".to_string(),
                    })
                } else {
                    Some(McpServerConfig {
                        command: path_str,
                        args: vec![],
                        description: "System PATH MCP server".to_string(),
                    })
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn description(&self) -> &str {
        "System PATH MCP server detection"
    }
}
