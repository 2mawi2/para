use crate::utils::{ParaError, Result};
use clap::{Args, Subcommand};
use dialoguer::Select;
use std::fs;
use std::process::Command;

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
    println!("📝 Created .mcp.json (commit this to share with team)");
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

fn create_mcp_json() -> Result<()> {
    let mcp_config = r#"{
  "mcpServers": {
    "para": {
      "type": "stdio",
      "command": "para-mcp-server"
    }
  }
}"#;

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

            // Add para-server to Claude Code user config
            match Command::new("claude")
                .args(["mcp", "add", "para-server", "para-mcp-server"])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ Claude Code configured");
                        println!("💡 Verify with: claude mcp list");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if stderr.contains("already exists")
                            || stderr.contains("already configured")
                        {
                            println!("✅ Claude Code already configured");
                        } else {
                            println!("⚠️  Warning: Failed to configure Claude Code user settings");
                            println!("   You can manually run: claude mcp add para-server para-mcp-server");
                        }
                    }
                }
                Err(_) => {
                    println!("⚠️  Warning: Failed to run claude mcp command");
                    println!("   You can manually run: claude mcp add para-server para-mcp-server");
                }
            }
        }
        Err(_) => {
            println!("ℹ️  Claude Code not found");
            println!("   Install Claude Code and run: claude mcp add para-server para-mcp-server");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_create_mcp_json() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().ok();

        // Change to temp directory
        env::set_current_dir(temp_dir.path()).unwrap();

        // Test creating .mcp.json
        assert!(create_mcp_json().is_ok());
        assert!(std::path::Path::new(".mcp.json").exists());

        // Test that it doesn't overwrite existing file
        let existing_content = "existing content";
        fs::write(".mcp.json", existing_content).unwrap();
        assert!(create_mcp_json().is_ok());
        let content = fs::read_to_string(".mcp.json").unwrap();
        assert_eq!(content.trim(), existing_content.trim());

        // Restore original directory if it exists
        if let Some(original) = original_dir {
            let _ = env::set_current_dir(original);
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
}
