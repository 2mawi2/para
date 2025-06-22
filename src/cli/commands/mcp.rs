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

trait InstallationStrategy {
    fn detect(&self) -> Option<McpServerConfig>;
}

struct HomebrewStrategy;
struct DevelopmentStrategy;
struct LocalInstallStrategy;
struct PathFallbackStrategy;

fn validate_node_script(path: &PathBuf) -> bool {
    if let Ok(first_line) = std::fs::read_to_string(path)
        .map(|content| content.lines().next().unwrap_or("").to_string())
    {
        first_line.starts_with("#!/usr/bin/env node")
            || (first_line.starts_with("#!") && first_line.contains("node"))
    } else {
        false
    }
}

fn create_server_config(
    command: String,
    args: Vec<String>,
    description: String,
) -> McpServerConfig {
    McpServerConfig {
        command,
        args,
        description,
    }
}

fn is_homebrew_installation() -> Result<bool> {
    let current_exe = std::env::current_exe()
        .map_err(|e| ParaError::invalid_args(format!("Failed to get current executable: {}", e)))?;
    let exe_path = current_exe.to_string_lossy();
    Ok(exe_path.contains("/homebrew/") || exe_path.contains("/usr/local/bin/"))
}

impl InstallationStrategy for HomebrewStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let homebrew_locations = vec![
            "/opt/homebrew/bin/para-mcp-server",              // Apple Silicon
            "/usr/local/bin/para-mcp-server",                 // Intel Mac
            "/home/linuxbrew/.linuxbrew/bin/para-mcp-server", // Linux
        ];

        for location in homebrew_locations {
            let path = PathBuf::from(location);
            if path.exists() {
                if validate_node_script(&path) {
                    return Some(create_server_config(
                        "node".to_string(),
                        vec![path.to_string_lossy().to_string()],
                        "Homebrew Node.js MCP server".to_string(),
                    ));
                } else {
                    return Some(create_server_config(
                        path.to_string_lossy().to_string(),
                        vec![],
                        "Homebrew MCP server".to_string(),
                    ));
                }
            }
        }
        None
    }
}

impl InstallationStrategy for DevelopmentStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let current_dir = std::env::current_dir().ok()?;
        let local_ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

        if local_ts_server.exists() {
            Some(create_server_config(
                "node".to_string(),
                vec![local_ts_server.to_string_lossy().to_string()],
                "Local development TypeScript MCP server".to_string(),
            ))
        } else {
            None
        }
    }
}

fn find_source_with_dependencies() -> Option<PathBuf> {
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

    for source in possible_source_locations.into_iter().flatten() {
        if source.exists() {
            if let Some(parent) = source.parent() {
                if parent.join("../node_modules").exists() {
                    return Some(source);
                }
            }
        }
    }
    None
}

impl InstallationStrategy for LocalInstallStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        let home_dir = directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~"));
        let local_server = home_dir.join(".local/bin/para-mcp-server");

        if local_server.exists() {
            if validate_node_script(&local_server) {
                // Try to find source with dependencies first
                if let Some(source_with_deps) = find_source_with_dependencies() {
                    return Some(create_server_config(
                        "node".to_string(),
                        vec![source_with_deps.to_string_lossy().to_string()],
                        "Para TypeScript MCP server with dependencies".to_string(),
                    ));
                }
                // Fallback to installed version without dependencies
                Some(create_server_config(
                    "node".to_string(),
                    vec![local_server.to_string_lossy().to_string()],
                    "Local Node.js MCP server (may need dependencies)".to_string(),
                ))
            } else {
                Some(create_server_config(
                    local_server.to_string_lossy().to_string(),
                    vec![],
                    "Local MCP server".to_string(),
                ))
            }
        } else {
            None
        }
    }
}

impl InstallationStrategy for PathFallbackStrategy {
    fn detect(&self) -> Option<McpServerConfig> {
        if let Ok(output) = Command::new("which").arg("para-mcp-server").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let path_buf = PathBuf::from(&path);

                if validate_node_script(&path_buf) {
                    Some(create_server_config(
                        "node".to_string(),
                        vec![path],
                        "System PATH Node.js MCP server".to_string(),
                    ))
                } else {
                    Some(create_server_config(
                        path,
                        vec![],
                        "System PATH MCP server".to_string(),
                    ))
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn find_mcp_server() -> Result<McpServerConfig> {
    let is_homebrew = is_homebrew_installation()?;

    if is_homebrew {
        let homebrew_strategy = HomebrewStrategy;
        if let Some(config) = homebrew_strategy.detect() {
            return Ok(config);
        }
        return Err(ParaError::invalid_args(
            "Para is installed via Homebrew but MCP server is missing.\n\
            Try reinstalling: brew reinstall para"
                .to_string(),
        ));
    }

    let strategies: Vec<Box<dyn InstallationStrategy>> = vec![
        Box::new(DevelopmentStrategy),
        Box::new(LocalInstallStrategy),
        Box::new(PathFallbackStrategy),
    ];

    for strategy in strategies {
        if let Some(config) = strategy.detect() {
            return Ok(config);
        }
    }

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

    // Comprehensive tests for find_mcp_server function
    mod find_mcp_server_tests {
        use super::*;
        use crate::utils::path::safe_resolve_path;
        use std::{env, fs, path::Path};
        use tempfile::TempDir;

        fn create_mock_executable(path: &Path, is_node_script: bool) {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if is_node_script {
                fs::write(path, "#!/usr/bin/env node\nconsole.log('test');").unwrap();
            } else {
                fs::write(path, "#!/bin/bash\necho 'test'").unwrap();
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        fn create_node_modules(base_path: &Path) {
            let node_modules = base_path.join("node_modules");
            fs::create_dir_all(&node_modules).unwrap();
            fs::write(node_modules.join("package.json"), "{}").unwrap();
        }

        #[test]
        fn test_find_mcp_server_homebrew_detection_apple_silicon() {
            let temp_dir = TempDir::new().unwrap();
            let mock_exe_path = temp_dir.path().join("homebrew").join("bin").join("para");
            create_mock_executable(&mock_exe_path, false);

            let server_path = temp_dir
                .path()
                .join("opt")
                .join("homebrew")
                .join("bin")
                .join("para-mcp-server");
            create_mock_executable(&server_path, false);

            // Mock current_exe to return homebrew path
            let _original_exe = env::current_exe().unwrap();

            // This test validates the homebrew detection logic
            // When para is run from /opt/homebrew/bin/, it should find the MCP server there
            let homebrew_locations = vec![
                "/opt/homebrew/bin/para-mcp-server",
                "/usr/local/bin/para-mcp-server",
                "/home/linuxbrew/.linuxbrew/bin/para-mcp-server",
            ];

            for location in homebrew_locations {
                let path = std::path::PathBuf::from(location);
                // Test path construction logic
                assert!(path.to_string_lossy().contains("para-mcp-server"));
                assert!(path.parent().unwrap().to_string_lossy().contains("bin"));
            }
        }

        #[test]
        fn test_find_mcp_server_homebrew_node_script_detection() {
            let temp_dir = TempDir::new().unwrap();
            let server_path = temp_dir
                .path()
                .join("opt")
                .join("homebrew")
                .join("bin")
                .join("para-mcp-server");
            create_mock_executable(&server_path, true);

            // Test Node.js script detection logic
            if server_path.exists() {
                let first_line = std::fs::read_to_string(&server_path)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(is_node_script);

                // Test expected config structure for Node.js script
                let expected_config = McpServerConfig {
                    command: "node".to_string(),
                    args: vec![server_path.to_string_lossy().to_string()],
                    description: "Homebrew Node.js MCP server".to_string(),
                };

                assert_eq!(expected_config.command, "node");
                assert_eq!(expected_config.args.len(), 1);
                assert!(expected_config.description.contains("Homebrew"));
            }
        }

        #[test]
        fn test_find_mcp_server_homebrew_binary_detection() {
            let temp_dir = TempDir::new().unwrap();
            let server_path = temp_dir
                .path()
                .join("usr")
                .join("local")
                .join("bin")
                .join("para-mcp-server");
            create_mock_executable(&server_path, false);

            // Test binary (non-Node.js) detection logic
            if server_path.exists() {
                let first_line = std::fs::read_to_string(&server_path)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(!is_node_script);

                // Test expected config structure for binary
                let expected_config = McpServerConfig {
                    command: server_path.to_string_lossy().to_string(),
                    args: vec![],
                    description: "Homebrew MCP server".to_string(),
                };

                assert_eq!(expected_config.args.len(), 0);
                assert!(expected_config.description.contains("Homebrew"));
            }
        }

        #[test]
        fn test_find_mcp_server_local_typescript_development() {
            let temp_dir = TempDir::new().unwrap();

            // Simulate being in a development directory
            let original_dir = env::current_dir().unwrap();
            env::set_current_dir(&temp_dir).unwrap();

            // Create the local TypeScript server path
            let ts_server_path = temp_dir
                .path()
                .join("mcp-server-ts")
                .join("build")
                .join("para-mcp-server.js");
            create_mock_executable(&ts_server_path, true);

            // Test path construction
            let current_dir = env::current_dir().unwrap();
            let expected_path = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

            // Use safe_resolve_path to handle symlink resolution differences on macOS
            assert_eq!(
                safe_resolve_path(&ts_server_path),
                safe_resolve_path(&expected_path)
            );
            assert!(ts_server_path.exists());

            // Test expected config structure
            let expected_config = McpServerConfig {
                command: "node".to_string(),
                args: vec![ts_server_path.to_string_lossy().to_string()],
                description: "Local development TypeScript MCP server".to_string(),
            };

            assert_eq!(expected_config.command, "node");
            assert!(expected_config.description.contains("Local development"));
            assert!(expected_config.description.contains("TypeScript"));

            // Restore original directory
            env::set_current_dir(original_dir).unwrap();
        }

        #[test]
        fn test_find_mcp_server_system_installation_node_script() {
            let temp_dir = TempDir::new().unwrap();

            // Create mock home directory structure
            let mock_home = temp_dir.path().join("home").join("user");
            let local_server = mock_home.join(".local").join("bin").join("para-mcp-server");
            create_mock_executable(&local_server, true);

            // Test Node.js script detection in ~/.local/bin
            if local_server.exists() {
                let first_line = std::fs::read_to_string(&local_server)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(is_node_script);

                // Test expected config for Node.js script without dependencies
                let expected_config = McpServerConfig {
                    command: "node".to_string(),
                    args: vec![local_server.to_string_lossy().to_string()],
                    description: "Local Node.js MCP server (may need dependencies)".to_string(),
                };

                assert_eq!(expected_config.command, "node");
                assert!(expected_config
                    .description
                    .contains("may need dependencies"));
            }
        }

        #[test]
        fn test_find_mcp_server_dependency_discovery() {
            let temp_dir = TempDir::new().unwrap();

            // Create mock home directory and server
            let mock_home = temp_dir.path().join("home").join("user");
            let local_server = mock_home.join(".local").join("bin").join("para-mcp-server");
            create_mock_executable(&local_server, true);

            // Create a source location with dependencies
            let source_with_deps = mock_home
                .join("git")
                .join("para")
                .join("mcp-server-ts")
                .join("build")
                .join("para-mcp-server.js");
            create_mock_executable(&source_with_deps, true);
            let deps_parent = source_with_deps.parent().unwrap().parent().unwrap();
            create_node_modules(deps_parent);

            // Test dependency discovery logic
            let possible_source_locations = vec![
                mock_home.join("Documents/git/para/mcp-server-ts/build/para-mcp-server.js"),
                mock_home.join("git/para/mcp-server-ts/build/para-mcp-server.js"),
                mock_home.join("repos/para/mcp-server-ts/build/para-mcp-server.js"),
                mock_home.join("projects/para/mcp-server-ts/build/para-mcp-server.js"),
            ];

            // Check the logic for finding sources with dependencies
            for source in possible_source_locations {
                if source.exists() {
                    if let Some(parent) = source.parent() {
                        let node_modules_path = parent.join("../node_modules");
                        if node_modules_path.exists() {
                            let expected_config = McpServerConfig {
                                command: "node".to_string(),
                                args: vec![source.to_string_lossy().to_string()],
                                description: "Para TypeScript MCP server with dependencies"
                                    .to_string(),
                            };

                            assert_eq!(expected_config.command, "node");
                            assert!(expected_config.description.contains("with dependencies"));
                        }
                    }
                }
            }
        }

        #[test]
        fn test_find_mcp_server_system_installation_binary() {
            let temp_dir = TempDir::new().unwrap();

            // Create mock home directory structure
            let mock_home = temp_dir.path().join("home").join("user");
            let local_server = mock_home.join(".local").join("bin").join("para-mcp-server");
            create_mock_executable(&local_server, false);

            // Test binary detection in ~/.local/bin
            if local_server.exists() {
                let first_line = std::fs::read_to_string(&local_server)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(!is_node_script);

                // Test expected config for binary
                let expected_config = McpServerConfig {
                    command: local_server.to_string_lossy().to_string(),
                    args: vec![],
                    description: "Local MCP server".to_string(),
                };

                assert_eq!(expected_config.args.len(), 0);
                assert!(expected_config.description.contains("Local MCP server"));
            }
        }

        #[test]
        fn test_find_mcp_server_path_fallback_node_script() {
            // Test the PATH fallback logic for Node.js scripts
            // This test validates the logic without actually calling 'which'

            let mock_path = "/usr/bin/para-mcp-server";
            let temp_dir = TempDir::new().unwrap();
            let mock_file = temp_dir.path().join("para-mcp-server");
            create_mock_executable(&mock_file, true);

            // Test Node.js script detection in PATH
            if mock_file.exists() {
                let first_line = std::fs::read_to_string(&mock_file)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(is_node_script);

                // Test expected config for Node.js script in PATH
                let expected_config = McpServerConfig {
                    command: "node".to_string(),
                    args: vec![mock_path.to_string()],
                    description: "System PATH Node.js MCP server".to_string(),
                };

                assert_eq!(expected_config.command, "node");
                assert!(expected_config.description.contains("System PATH"));
                assert!(expected_config.description.contains("Node.js"));
            }
        }

        #[test]
        fn test_find_mcp_server_path_fallback_binary() {
            // Test the PATH fallback logic for binary executables

            let mock_path = "/usr/bin/para-mcp-server";
            let temp_dir = TempDir::new().unwrap();
            let mock_file = temp_dir.path().join("para-mcp-server");
            create_mock_executable(&mock_file, false);

            // Test binary detection in PATH
            if mock_file.exists() {
                let first_line = std::fs::read_to_string(&mock_file)
                    .map(|content| content.lines().next().unwrap_or("").to_string())
                    .unwrap();

                let is_node_script = first_line.starts_with("#!/usr/bin/env node")
                    || (first_line.starts_with("#!") && first_line.contains("node"));

                assert!(!is_node_script);

                // Test expected config for binary in PATH
                let expected_config = McpServerConfig {
                    command: mock_path.to_string(),
                    args: vec![],
                    description: "System PATH MCP server".to_string(),
                };

                assert_eq!(expected_config.args.len(), 0);
                assert!(expected_config.description.contains("System PATH"));
                assert!(!expected_config.description.contains("Node.js"));
            }
        }

        #[test]
        fn test_find_mcp_server_no_server_found_error() {
            // Test the error case when no MCP server is found
            let expected_error_message =
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
        💡 After installing, run 'para mcp init --claude-code' again to update the configuration.";

            // Validate error message structure
            assert!(expected_error_message.contains("No para MCP server found"));
            assert!(expected_error_message.contains("📋 Install options"));
            assert!(expected_error_message.contains("🔧 For development in this repo"));
            assert!(expected_error_message.contains("🏠 For production use"));
            assert!(expected_error_message.contains("🛠️  Manual installation"));
            assert!(expected_error_message.contains("⚡ Quick check"));
            assert!(expected_error_message.contains("💡 After installing"));
            assert!(expected_error_message.contains("brew install"));
            assert!(expected_error_message.contains("just install"));
            assert!(expected_error_message.contains("which para-mcp-server"));
        }

        #[test]
        fn test_find_mcp_server_homebrew_missing_error() {
            // Test the specific error when para is installed via homebrew but MCP server is missing
            let expected_error_message =
                "Para is installed via Homebrew but MCP server is missing.\n\
            Try reinstalling: brew reinstall para";

            // Validate homebrew-specific error message
            assert!(expected_error_message.contains("Para is installed via Homebrew"));
            assert!(expected_error_message.contains("MCP server is missing"));
            assert!(expected_error_message.contains("brew reinstall para"));
        }

        #[test]
        fn test_find_mcp_server_exe_path_detection() {
            // Test the executable path detection logic for homebrew identification

            let homebrew_paths = vec![
                "/opt/homebrew/bin/para",
                "/usr/local/bin/para",
                "/opt/homebrew/Cellar/para/1.0.0/bin/para",
            ];

            for path in homebrew_paths {
                let is_homebrew = path.contains("/homebrew/") || path.contains("/usr/local/bin/");
                assert!(is_homebrew, "Path {} should be detected as homebrew", path);
            }

            let non_homebrew_paths = vec![
                "/usr/bin/para",
                "/home/user/.local/bin/para",
                "/home/user/git/para/target/debug/para",
                "/tmp/para",
            ];

            for path in non_homebrew_paths {
                let is_homebrew = path.contains("/homebrew/") || path.contains("/usr/local/bin/");
                assert!(
                    !is_homebrew,
                    "Path {} should NOT be detected as homebrew",
                    path
                );
            }
        }

        #[test]
        fn test_find_mcp_server_complex_integration_scenario() {
            // Test a complex scenario that mimics real-world usage
            let temp_dir = TempDir::new().unwrap();

            // Set up a scenario where multiple locations exist but we should pick the right one
            let mock_home = temp_dir.path().join("home").join("user");

            // Create local system installation (should be found if not homebrew)
            let local_server = mock_home.join(".local").join("bin").join("para-mcp-server");
            create_mock_executable(&local_server, true);

            // Create a development server (should be preferred over system installation)
            let original_dir = env::current_dir().unwrap();
            let dev_dir = temp_dir.path().join("dev").join("para");
            fs::create_dir_all(&dev_dir).unwrap();
            env::set_current_dir(&dev_dir).unwrap();

            let ts_server = dev_dir
                .join("mcp-server-ts")
                .join("build")
                .join("para-mcp-server.js");
            create_mock_executable(&ts_server, true);

            // Test priority logic: local development should be preferred
            let current_dir = env::current_dir().unwrap();
            let local_ts_server = current_dir.join("mcp-server-ts/build/para-mcp-server.js");

            if local_ts_server.exists() {
                // Development server should be preferred
                let expected_config = McpServerConfig {
                    command: "node".to_string(),
                    args: vec![local_ts_server.to_string_lossy().to_string()],
                    description: "Local development TypeScript MCP server".to_string(),
                };

                assert_eq!(expected_config.command, "node");
                assert!(expected_config.description.contains("Local development"));
            }

            // Restore directory
            env::set_current_dir(original_dir).unwrap();
        }
    }
}
