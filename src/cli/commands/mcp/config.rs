use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::fs;
use std::process::Command;

use super::strategies::{
    get_detection_strategies, HomebrewDetectionStrategy, McpServerConfig,
    McpServerDetectionStrategy,
};

/// Simplified MCP server detection using strategy pattern
pub fn find_mcp_server() -> Result<McpServerConfig> {
    // Check if we're running from homebrew - if so, only use homebrew strategy
    let current_exe = std::env::current_exe()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get current executable: {}", e)))?;
    let exe_path = current_exe.to_string_lossy();
    let is_homebrew = exe_path.contains("/homebrew/") || exe_path.contains("/usr/local/bin/");

    let strategies: Vec<Box<dyn McpServerDetectionStrategy>> = if is_homebrew {
        // For homebrew installations, only use homebrew strategy
        vec![Box::new(HomebrewDetectionStrategy)]
    } else {
        // For other installations, try all strategies in order
        get_detection_strategies()
    };

    let mut tried_strategies = Vec::new();
    for strategy in strategies {
        tried_strategies.push(strategy.description().to_string());
        if let Some(config) = strategy.detect() {
            return Ok(config);
        }
    }

    // Handle homebrew case specifically
    if is_homebrew {
        return Err(ParaError::invalid_args(
            "Para is installed via Homebrew but MCP server is missing.\n\
            Try reinstalling: brew reinstall para"
                .to_string(),
        ));
    }

    // No MCP server found - provide detailed guidance with strategies tried
    let strategies_tried = tried_strategies.join(", ");
    Err(ParaError::invalid_args(
        format!(
            "No para MCP server found. Claude Code won't be able to connect to Para tools.\n\
            Tried strategies: {}\n\n\
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
            💡 After installing, run 'para mcp init --claude-code' again to update the configuration.",
            strategies_tried
        )
    ))
}

/// Create .mcp.json configuration file
pub fn create_mcp_json() -> Result<bool> {
    // Try to find MCP server in multiple locations
    let mcp_server_path = find_mcp_server()?;

    let para_config = serde_json::json!({
        "type": "stdio",
        "command": mcp_server_path.command,
        "args": mcp_server_path.args
    });

    let mcp_path = std::path::Path::new(".mcp.json");

    // Load existing .mcp.json or create new one
    let mut mcp_config = if mcp_path.exists() {
        let content = fs::read_to_string(mcp_path)
            .map_err(|e| ParaError::fs_error(format!("Failed to read .mcp.json: {}", e)))?;

        if content.trim().is_empty() {
            // File exists but is empty, create new config
            serde_json::json!({
                "mcpServers": {}
            })
        } else {
            serde_json::from_str(&content).map_err(|e| {
                ParaError::invalid_config(format!("Invalid .mcp.json format: {}", e))
            })?
        }
    } else {
        // File doesn't exist, create new config
        serde_json::json!({
            "mcpServers": {}
        })
    };

    // Check if para server is already configured
    if let Some(servers) = mcp_config.get_mut("mcpServers") {
        if servers.get("para").is_some() {
            return Ok(false); // Already configured
        }
        servers["para"] = para_config;
    } else {
        // Add mcpServers section if it doesn't exist
        mcp_config["mcpServers"] = serde_json::json!({
            "para": para_config
        });
    }

    // Write the updated config with proper formatting
    let formatted_config = serde_json::to_string_pretty(&mcp_config)
        .map_err(|e| ParaError::fs_error(format!("Failed to serialize .mcp.json: {}", e)))?;

    fs::write(mcp_path, formatted_config)
        .map_err(|e| ParaError::fs_error(format!("Failed to write .mcp.json: {}", e)))?;
    Ok(true)
}

/// Prompt user to select IDE
pub fn prompt_for_ide() -> Result<&'static str> {
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

/// Configure Claude Code IDE integration
pub fn configure_claude_code() -> Result<()> {
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
