use crate::utils::Result;
use clap::{Args, Subcommand};

pub mod config;
pub mod strategies;
pub mod utils;

use config::{check_claude_code_availability, create_mcp_json};
use utils::add_to_gitignore;

#[derive(Args)]
pub struct McpCommand {
    #[command(subcommand)]
    pub command: McpSubcommand,
}

#[derive(Subcommand)]
pub enum McpSubcommand {
    /// Initialize MCP integration for Para
    Init(McpInitArgs),
}

#[derive(Args)]
pub struct McpInitArgs {}

pub fn handle_mcp_command(cmd: McpCommand) -> Result<()> {
    match cmd.command {
        McpSubcommand::Init(args) => handle_mcp_init(args),
    }
}

fn handle_mcp_init(_args: McpInitArgs) -> Result<()> {
    println!("🔧 Setting up Para MCP integration...");

    // Always create .mcp.json first
    let created = create_mcp_json()?;
    if created {
        println!("✓ Created .mcp.json with Para MCP server configuration");
    } else {
        println!("✓ .mcp.json already exists with Para configuration");
    }

    // Automatically add .mcp.json to .gitignore if it's not already there
    match add_to_gitignore(".mcp.json") {
        Ok(true) => println!("✓ Added .mcp.json to .gitignore (contains user-specific paths)"),
        Ok(false) => println!("✓ .mcp.json already in .gitignore"),
        Err(e) => println!("⚠️  Could not update .gitignore: {e}"),
    }
    println!();

    // Check Claude Code availability (informational only)
    check_claude_code_availability();

    println!();
    println!("🎉 Para MCP server configured!");
    println!("   The .mcp.json file has been created with Para server settings.");
    println!("   This enables Para tools in IDEs that support MCP.");
    println!();
    println!("💡 To use Para tools:");
    println!("   - In Claude Code: Tools will appear automatically after restart");
    println!("   - In other IDEs: Check their MCP documentation");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strategies::{is_node_script, McpServerConfig};

    #[test]
    fn test_mcp_json_creation_logic() {
        // Test .mcp.json creation logic directly
        let server_config = McpServerConfig {
            command: "node".to_string(),
            args: vec!["/test/path/para-mcp-server.js".to_string()],
            description: "Test TypeScript MCP server".to_string(),
        };

        let mcp_config = format!(
            r#"{{
  "mcpServers": {{
    "para": {{
      "type": "stdio",
      "command": "{}",
      "args": {}
    }}
  }}
}}"#,
            server_config.command,
            serde_json::to_string(&server_config.args).unwrap()
        );

        // Verify the JSON structure
        assert!(mcp_config.contains("mcpServers"));
        assert!(mcp_config.contains("para"));
        assert!(mcp_config.contains("node"));
        assert!(mcp_config.contains("stdio"));

        // Verify we can parse it back
        let parsed: serde_json::Value = serde_json::from_str(&mcp_config).unwrap();
        assert!(parsed["mcpServers"]["para"]["command"].as_str().unwrap() == "node");
        assert!(parsed["mcpServers"]["para"]["type"].as_str().unwrap() == "stdio");
    }

    #[test]
    fn test_server_discovery_paths() {
        // Test server path construction logic
        use std::path::PathBuf;

        // Test TypeScript server path
        let current_dir = PathBuf::from("/test/dir");
        let ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");
        assert_eq!(
            ts_server.to_string_lossy(),
            "/test/dir/mcp-server-ts/build/para-mcp-server.js"
        );

        // Test Homebrew paths
        let homebrew_paths = vec![
            "/opt/homebrew/bin/para-mcp-server",
            "/usr/local/bin/para-mcp-server",
            "/home/linuxbrew/.linuxbrew/bin/para-mcp-server",
        ];

        for path in homebrew_paths {
            let path_buf = PathBuf::from(path);
            assert!(path_buf.to_string_lossy().contains("para-mcp-server"));
        }
    }

    #[test]
    fn test_mcp_init_args_parse() {
        // Test that McpInitArgs can be parsed without any flags
        use clap::Parser;

        #[derive(Parser)]
        struct TestArgs {
            #[command(flatten)]
            mcp: McpInitArgs,
        }

        // Should parse with no flags
        assert!(TestArgs::try_parse_from(["test"]).is_ok());
    }

    #[test]
    fn test_mcp_server_config_structure() {
        // Test McpServerConfig struct creation and serialization
        let config = McpServerConfig {
            command: "test-command".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
            description: "Test server".to_string(),
        };

        assert_eq!(config.command, "test-command");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.description, "Test server");

        // Test JSON serialization of args
        let json_args = serde_json::to_string(&config.args).unwrap();
        assert!(json_args.contains("arg1"));
        assert!(json_args.contains("arg2"));
    }

    #[test]
    fn test_add_to_gitignore() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Test the GitignoreManager directly without changing directories
        let gitignore_manager =
            crate::utils::gitignore::GitignoreManager::new(temp_dir.path().to_str().unwrap());

        // Test adding to new gitignore
        let added = gitignore_manager.add_entry(".mcp.json").unwrap();
        assert!(added);
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".mcp.json"));

        // Test adding duplicate entry (should not duplicate)
        let added_again = gitignore_manager.add_entry(".mcp.json").unwrap();
        assert!(!added_again);
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content.matches(".mcp.json").count(), 1);
    }

    // Tests for MCP server detection strategies
    mod strategy_tests {
        use super::*;
        use std::fs;
        use strategies::*;
        use tempfile::TempDir;

        #[test]
        fn test_homebrew_detection_strategy() {
            let strategy = HomebrewDetectionStrategy;
            assert_eq!(strategy.description(), "Homebrew MCP server detection");
        }

        #[test]
        fn test_development_detection_strategy() {
            let strategy = DevelopmentDetectionStrategy;
            assert_eq!(
                strategy.description(),
                "Local development MCP server detection"
            );
        }

        #[test]
        fn test_system_detection_strategy() {
            let strategy = SystemDetectionStrategy;
            assert_eq!(
                strategy.description(),
                "System installation MCP server detection"
            );
        }

        #[test]
        fn test_path_detection_strategy() {
            let strategy = PathDetectionStrategy;
            assert_eq!(strategy.description(), "System PATH MCP server detection");
        }

        #[test]
        fn test_node_script_detection() {
            let temp_dir = TempDir::new().unwrap();

            // Test Node.js script with shebang
            let node_script = temp_dir.path().join("node-script");
            fs::write(&node_script, "#!/usr/bin/env node\nconsole.log('test');").unwrap();
            assert!(is_node_script(&node_script));

            // Test alternative Node.js shebang
            let node_script2 = temp_dir.path().join("node-script2");
            fs::write(&node_script2, "#!/bin/node\nconsole.log('test');").unwrap();
            assert!(is_node_script(&node_script2));

            // Test non-Node.js script
            let bash_script = temp_dir.path().join("bash-script");
            fs::write(&bash_script, "#!/bin/bash\necho 'test'").unwrap();
            assert!(!is_node_script(&bash_script));

            // Test binary file
            let binary = temp_dir.path().join("binary");
            fs::write(&binary, [0x7f, 0x45, 0x4c, 0x46]).unwrap(); // ELF header
            assert!(!is_node_script(&binary));
        }
    }
}
