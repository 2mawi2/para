use super::{is_node_script, McpServerConfig, McpServerDetectionStrategy};
use std::path::{Path, PathBuf};

/// Strategy for detecting system-installed MCP servers
pub struct SystemDetectionStrategy;

impl McpServerDetectionStrategy for SystemDetectionStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let home_dir = get_home_directory();
        let local_server = build_local_server_path(&home_dir);

        if !local_server.exists() {
            return None;
        }

        if is_node_script(&local_server) {
            find_typescript_server_with_dependencies(&home_dir)
                .or_else(|| create_fallback_node_server_config(&local_server))
        } else {
            create_binary_server_config(&local_server)
        }
    }

    fn description(&self) -> &str {
        "System installation MCP server detection"
    }
}

fn get_home_directory() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("~"))
}

fn build_local_server_path(home_dir: &Path) -> PathBuf {
    home_dir.join(".local/bin/para-mcp-server")
}

fn get_possible_typescript_source_locations(home_dir: &Path) -> Vec<PathBuf> {
    vec![
        home_dir.join("Documents/git/para/mcp-server-ts/build/para-mcp-server.js"),
        home_dir.join("git/para/mcp-server-ts/build/para-mcp-server.js"),
        home_dir.join("repos/para/mcp-server-ts/build/para-mcp-server.js"),
        home_dir.join("projects/para/mcp-server-ts/build/para-mcp-server.js"),
        PathBuf::from("/opt/homebrew/opt/para/mcp-server-ts/build/para-mcp-server.js"),
        PathBuf::from("/usr/local/opt/para/mcp-server-ts/build/para-mcp-server.js"),
    ]
}

fn find_typescript_server_with_dependencies(home_dir: &Path) -> Option<McpServerConfig> {
    let possible_locations = get_possible_typescript_source_locations(home_dir);

    for source in possible_locations {
        if source.exists() {
            if let Some(parent) = source.parent() {
                if parent.join("../node_modules").exists() {
                    return Some(create_node_server_config(
                        source,
                        "Para TypeScript MCP server with dependencies".to_string(),
                    ));
                }
            }
        }
    }
    None
}

fn create_fallback_node_server_config(local_server: &Path) -> Option<McpServerConfig> {
    Some(create_node_server_config(
        local_server.to_path_buf(),
        "Local Node.js MCP server (may need dependencies)".to_string(),
    ))
}

fn create_node_server_config(server_path: PathBuf, description: String) -> McpServerConfig {
    McpServerConfig {
        command: "node".to_string(),
        args: vec![server_path.to_string_lossy().to_string()],
        description,
    }
}

fn create_binary_server_config(server_path: &Path) -> Option<McpServerConfig> {
    Some(McpServerConfig {
        command: server_path.to_string_lossy().to_string(),
        args: vec![],
        description: "Local MCP server".to_string(),
    })
}
