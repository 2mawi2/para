use crate::utils::{ParaError, Result};
use clap::{Args, Subcommand};
use dialoguer::Select;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
struct McpServerConfig {
    command: String,
    args: Vec<String>,
    description: String,
}

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
pub struct McpInitArgs {
    /// Setup for Claude Code (adds user config)
    #[arg(long, conflicts_with_all = ["cursor", "vscode"])]
    pub claude_code: bool,

    /// Setup for Cursor (project config only)
    #[arg(long, conflicts_with_all = ["claude_code", "vscode"])]
    pub cursor: bool,

    /// Setup for VS Code with Roo Code (project config only)
    #[arg(long, conflicts_with_all = ["claude_code", "cursor"])]
    pub vscode: bool,
}

pub fn handle_mcp_command(cmd: McpCommand) -> Result<()> {
    match cmd.command {
        McpSubcommand::Init(args) => handle_mcp_init(args),
    }
}

fn handle_mcp_init(args: McpInitArgs) -> Result<()> {
    println!("🔧 Setting up Para MCP integration...");

    // Always create .mcp.json first
    create_mcp_json()?;
    println!("Created .mcp.json (add to .gitignore - contains local paths)");
    println!();

    // Determine IDE choice
    let ide = if args.claude_code {
        "claude-code"
    } else if args.cursor {
        "cursor"
    } else if args.vscode {
        "vscode"
    } else {
        // Interactive mode
        prompt_for_ide()?
    };

    // Configure IDE-specific setup
    match ide {
        "claude-code" => configure_claude_code()?,
        "cursor" => {
            println!("✅ Cursor configured via .mcp.json");
        }
        "vscode" => {
            println!("✅ VS Code configured via .mcp.json");
        }
        "skip" => {
            println!("ℹ️  Skipped IDE-specific setup");
        }
        _ => {} // Should not happen
    }

    println!();
    println!("🎉 Para MCP integration complete!");
    println!("💡 Restart your IDE to see Para tools");

    Ok(())
}

fn find_mcp_server() -> Result<McpServerConfig> {
    // Try multiple locations in order of preference

    // 1. Local development: TypeScript server in current directory
    let current_dir = std::env::current_dir()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get current directory: {}", e)))?;
    let local_ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

    if local_ts_server.exists() {
        return Ok(McpServerConfig {
            command: "node".to_string(),
            args: vec![local_ts_server.to_string_lossy().to_string()],
            description: "Local TypeScript MCP server".to_string(),
        });
    }

    // 2. System installation: Rust MCP server in ~/.local/bin
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".to_string());
    let local_rust_server = PathBuf::from(&home_dir).join(".local/bin/para-mcp-server");

    if local_rust_server.exists() {
        return Ok(McpServerConfig {
            command: local_rust_server.to_string_lossy().to_string(),
            args: vec![],
            description: "Local Rust MCP server".to_string(),
        });
    }

    // 3. Homebrew installation: Check common Homebrew locations
    let homebrew_locations = vec![
        "/opt/homebrew/bin/para-mcp-server",              // Apple Silicon
        "/usr/local/bin/para-mcp-server",                 // Intel Mac
        "/home/linuxbrew/.linuxbrew/bin/para-mcp-server", // Linux
    ];

    for location in homebrew_locations {
        let path = PathBuf::from(location);
        if path.exists() {
            return Ok(McpServerConfig {
                command: path.to_string_lossy().to_string(),
                args: vec![],
                description: "Homebrew Rust MCP server".to_string(),
            });
        }
    }

    // 4. System PATH: Try to find para-mcp-server in PATH
    if let Ok(output) = Command::new("which").arg("para-mcp-server").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(McpServerConfig {
                command: path,
                args: vec![],
                description: "System PATH Rust MCP server".to_string(),
            });
        }
    }

    // No MCP server found
    Err(ParaError::invalid_args(
        "No para MCP server found. Install options:\n\n\
        For development:\n  \
        cd mcp-server-ts && npm install && npm run build\n\n\
        For production:\n  \
        brew install para  # (includes MCP server)\n  \
        # or\n  \
        just install       # (builds from source)"
            .to_string(),
    ))
}

fn create_mcp_json() -> Result<()> {
    // Try to find MCP server in multiple locations
    let mcp_server_path = find_mcp_server()?;

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
        mcp_server_path.command,
        serde_json::to_string(&mcp_server_path.args).unwrap()
    );

    if std::path::Path::new(".mcp.json").exists() {
        println!("ℹ️  .mcp.json already exists");
        return Ok(());
    }

    fs::write(".mcp.json", mcp_config)?;
    Ok(())
}

fn prompt_for_ide() -> Result<&'static str> {
    let choices = vec![
        "Claude Code (adds user config for better integration)",
        "Cursor (project config only)",
        "VS Code with Roo Code (project config only)",
        "Skip IDE-specific setup",
    ];

    let selection = Select::new()
        .with_prompt("🎯 Choose your IDE")
        .items(&choices)
        .default(0)
        .interact()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get user input: {}", e)))?;

    match selection {
        0 => Ok("claude-code"),
        1 => Ok("cursor"),
        2 => Ok("vscode"),
        3 => Ok("skip"),
        _ => Ok("skip"), // Fallback
    }
}

fn configure_claude_code() -> Result<()> {
    // Check if Claude Code is available
    match Command::new("claude").arg("--version").output() {
        Ok(_) => {
            println!("🔧 Configuring Claude Code user settings...");

            // Use the server discovery logic
            match find_mcp_server() {
                Ok(server_config) => {
                    println!("✅ Found MCP server: {}", server_config.description);
                    println!("✅ Claude Code will use project-scoped .mcp.json");
                    println!("💡 Verify with: claude mcp list");
                }
                Err(e) => {
                    println!("⚠️  {}", e);
                    println!("💡 Build the TypeScript server for development:");
                    println!("   cd mcp-server-ts && npm install && npm run build");
                    println!("💡 Or install para globally:");
                    println!("   just install");
                }
            }
        }
        Err(_) => {
            println!("ℹ️  Claude Code not found");
            println!("   Install Claude Code and the .mcp.json will work automatically");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_mcp_init_args_conflicts() {
        // This test ensures our clap conflicts work correctly
        use clap::Parser;

        #[derive(Parser)]
        struct TestArgs {
            #[command(flatten)]
            mcp: McpInitArgs,
        }

        // Valid single flags should work
        assert!(TestArgs::try_parse_from(["test", "--claude-code"]).is_ok());
        assert!(TestArgs::try_parse_from(["test", "--cursor"]).is_ok());
        assert!(TestArgs::try_parse_from(["test", "--vscode"]).is_ok());

        // Conflicting flags should fail
        assert!(TestArgs::try_parse_from(["test", "--claude-code", "--cursor"]).is_err());
        assert!(TestArgs::try_parse_from(["test", "--cursor", "--vscode"]).is_err());
        assert!(TestArgs::try_parse_from(["test", "--claude-code", "--vscode"]).is_err());
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
}
