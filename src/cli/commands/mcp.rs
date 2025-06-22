use crate::utils::{gitignore::GitignoreManager, ParaError, Result};
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
    let created = create_mcp_json()?;
    if created {
        println!("Created .mcp.json");
    } else {
        println!("✓ .mcp.json already exists");
    }

    // Automatically add .mcp.json to .gitignore if it's not already there
    match add_to_gitignore(".mcp.json") {
        Ok(true) => println!("Added .mcp.json to .gitignore (contains user-specific paths)"),
        Ok(false) => println!("✓ .mcp.json already in .gitignore"),
        Err(e) => println!("⚠️  Could not update .gitignore: {}", e),
    }
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
    // Detect if we're running from a homebrew installation
    let current_exe = std::env::current_exe()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get current executable: {}", e)))?;
    let exe_path = current_exe.to_string_lossy();

    // Check if running from homebrew location
    let is_homebrew = exe_path.contains("/homebrew/") || exe_path.contains("/usr/local/bin/");

    if is_homebrew {
        // For homebrew installations, ONLY use homebrew MCP server
        let homebrew_locations = vec![
            "/opt/homebrew/bin/para-mcp-server",              // Apple Silicon
            "/usr/local/bin/para-mcp-server",                 // Intel Mac
            "/home/linuxbrew/.linuxbrew/bin/para-mcp-server", // Linux
        ];

        for location in homebrew_locations {
            let path = PathBuf::from(location);
            if path.exists() {
                // Check if it's a Node.js script
                if let Ok(first_line) = std::fs::read_to_string(&path)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                {
                    if first_line.starts_with("#!/usr/bin/env node")
                        || first_line.starts_with("#!") && first_line.contains("node")
                    {
                        return Ok(McpServerConfig {
                            command: "node".to_string(),
                            args: vec![path.to_string_lossy().to_string()],
                            description: "Homebrew Node.js MCP server".to_string(),
                        });
                    }
                }

                return Ok(McpServerConfig {
                    command: path.to_string_lossy().to_string(),
                    args: vec![],
                    description: "Homebrew MCP server".to_string(),
                });
            }
        }

        return Err(ParaError::invalid_args(
            "Para is installed via Homebrew but MCP server is missing.\n\
            Try reinstalling: brew reinstall para"
                .to_string(),
        ));
    }

    // For development/local installations, check in this order:

    // 1. Local development: TypeScript server in current directory
    let current_dir = std::env::current_dir()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get current directory: {}", e)))?;
    let local_ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

    if local_ts_server.exists() {
        return Ok(McpServerConfig {
            command: "node".to_string(),
            args: vec![local_ts_server.to_string_lossy().to_string()],
            description: "Local development TypeScript MCP server".to_string(),
        });
    }

    // 2. System installation: MCP server in ~/.local/bin
    let home_dir = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("~"));
    let local_server = home_dir.join(".local/bin/para-mcp-server");

    if local_server.exists() {
        // Check if it's a Node.js script by reading the first line
        if let Ok(first_line) = std::fs::read_to_string(&local_server)
            .map(|content| content.lines().next().unwrap_or("").to_string())
        {
            if first_line.starts_with("#!/usr/bin/env node")
                || first_line.starts_with("#!") && first_line.contains("node")
            {
                // It's a Node.js script - but we need to check if it can actually run
                // Try to find the original installation with dependencies
                let possible_source_locations = vec![
                    // Look for common para installation directories
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
                        // Check if node_modules exists in the parent directory
                        if let Some(parent) = source.parent() {
                            if parent.join("../node_modules").exists() {
                                return Ok(McpServerConfig {
                                    command: "node".to_string(),
                                    args: vec![source.to_string_lossy().to_string()],
                                    description: "Para TypeScript MCP server with dependencies"
                                        .to_string(),
                                });
                            }
                        }
                    }
                }

                // If we can't find the source with dependencies, use the installed one anyway
                // (it might fail, but we'll provide a clear error)
                return Ok(McpServerConfig {
                    command: "node".to_string(),
                    args: vec![local_server.to_string_lossy().to_string()],
                    description: "Local Node.js MCP server (may need dependencies)".to_string(),
                });
            }
        }

        // Otherwise treat it as a binary
        return Ok(McpServerConfig {
            command: local_server.to_string_lossy().to_string(),
            args: vec![],
            description: "Local MCP server".to_string(),
        });
    }

    // 3. System PATH as fallback
    if let Ok(output) = Command::new("which").arg("para-mcp-server").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

            // Check if the found script is a Node.js script
            if let Ok(first_line) = std::fs::read_to_string(&path)
                .map(|content| content.lines().next().unwrap_or("").to_string())
            {
                if first_line.starts_with("#!/usr/bin/env node")
                    || first_line.starts_with("#!") && first_line.contains("node")
                {
                    return Ok(McpServerConfig {
                        command: "node".to_string(),
                        args: vec![path],
                        description: "System PATH Node.js MCP server".to_string(),
                    });
                }
            }

            return Ok(McpServerConfig {
                command: path,
                args: vec![],
                description: "System PATH MCP server".to_string(),
            });
        }
    }

    // No MCP server found - provide detailed guidance
    Err(ParaError::invalid_args(
        "No para MCP server found. Claude Code won't be able to connect to Para tools.\n\n\
        📋 Install options (choose one):\n\n\
        🔧 For development in this repo:\n  \
        cd mcp-server-ts && npm install && npm run build\n\n\
        🏠 For production use:\n  \
        brew install 2mawi2/tap/para  # (includes MCP server)\n\n\
        🛠️  Manual installation:\n  \
        just install  # (builds and installs to ~/.local/bin)\n\n\
        ⚡ Quick check:\n  \
        Run 'which para-mcp-server' to see if it's in your PATH\n  \
        Check 'node mcp-server-ts/build/para-mcp-server.js --help' for TypeScript server\n\n\
        💡 After installing, run 'para mcp init --claude-code' again to update the configuration."
            .to_string(),
    ))
}

fn create_mcp_json() -> Result<bool> {
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
        return Ok(false);
    }

    fs::write(".mcp.json", mcp_config)?;
    Ok(true)
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
        Ok(output) => {
            let version_output = String::from_utf8_lossy(&output.stdout);
            println!("🔧 Configuring Claude Code user settings...");
            println!("   Found Claude Code: {}", version_output.trim());

            // Use the server discovery logic
            match find_mcp_server() {
                Ok(server_config) => {
                    println!("✅ Found MCP server: {}", server_config.description);
                    println!("✅ Claude Code will use project-scoped .mcp.json");
                    println!("💡 Verify with: claude mcp list");
                }
                Err(e) => {
                    println!("❌ MCP server setup incomplete:");
                    println!("   {}", e);
                    return Err(e);
                }
            }
        }
        Err(_) => {
            println!("⚠️  Claude Code not found in PATH");
            println!("   📥 Install Claude Code from: https://claude.ai/download");
            println!("   🔄 After installation, run 'para mcp init --claude-code' again");
            println!("   ✅ The .mcp.json configuration will work automatically once Claude Code is installed");
        }
    }

    Ok(())
}

fn add_to_gitignore(entry: &str) -> Result<bool> {
    let gitignore_manager = GitignoreManager::new(".");
    gitignore_manager
        .add_entry(entry)
        .map_err(|e| ParaError::file_operation(format!("Failed to update .gitignore: {}", e)))
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
}
