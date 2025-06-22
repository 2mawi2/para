use std::path::PathBuf;

/// Configuration for an MCP server
#[derive(Debug, Clone, PartialEq)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub description: String,
}

/// Trait for different MCP server detection strategies
pub trait McpServerDetectionStrategy {
    /// Attempt to detect an MCP server using this strategy
    fn detect(&self) -> Option<McpServerConfig>;

    /// Get a description of this detection strategy
    fn description(&self) -> &str;
}

mod development;
mod homebrew;
mod path;
mod system;

pub use development::DevelopmentDetectionStrategy;
pub use homebrew::HomebrewDetectionStrategy;
pub use path::PathDetectionStrategy;
pub use system::SystemDetectionStrategy;

/// Get all detection strategies in order of preference
pub fn get_detection_strategies() -> Vec<Box<dyn McpServerDetectionStrategy>> {
    vec![
        Box::new(DevelopmentDetectionStrategy),
        Box::new(SystemDetectionStrategy),
        Box::new(PathDetectionStrategy),
    ]
}

/// Utility function to check if a file is a Node.js script
pub fn is_node_script(path: &PathBuf) -> bool {
    if let Ok(first_line) = std::fs::read_to_string(path)
        .map(|content| content.lines().next().unwrap_or("").to_string())
    {
        first_line.starts_with("#!/usr/bin/env node")
            || (first_line.starts_with("#!") && first_line.contains("node"))
    } else {
        false
    }
}
