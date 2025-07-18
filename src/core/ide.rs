use crate::config::{Config, IdeConfig};
use crate::core::sandbox::launcher::{
    generate_network_sandbox_wrapper, is_sandbox_available, wrap_command_with_sandbox,
    SandboxOptions,
};
use crate::core::sandbox::proxy::DEFAULT_PROXY_PORT;
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Options for launching IDE with specific features
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub skip_permissions: bool,
    pub continue_conversation: bool,
    pub claude_session_id: Option<String>,
    pub prompt: Option<String>,
    pub sandbox_override: Option<bool>,  // CLI flag override
    pub sandbox_profile: Option<String>, // CLI profile override
    pub network_sandbox: bool,           // Enable network sandboxing
    pub allowed_domains: Vec<String>,    // Additional allowed domains for proxy
}

pub struct IdeManager {
    ide_config: IdeConfig,
    sandbox_config: Option<crate::core::sandbox::SandboxConfig>,
}

impl IdeManager {
    pub fn new(config: &Config) -> Self {
        Self {
            ide_config: config.ide.clone(),
            sandbox_config: config.sandbox.clone(),
        }
    }

    pub fn launch(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        let options = LaunchOptions {
            skip_permissions,
            ..Default::default()
        };
        self.launch_with_options(path, options)
    }

    pub fn launch_with_options(&self, path: &Path, options: LaunchOptions) -> Result<()> {
        // All IDEs require wrapper mode for cloud-based launching
        if !self.ide_config.wrapper.enabled {
            return Err(ParaError::ide_error(
                "All IDEs require wrapper mode for cloud-based launching. Please run 'para config' to enable wrapper mode.\n   Available options: VS Code wrapper or Cursor wrapper".to_string()
            ));
        }

        println!(
            "▶ launching {} inside {} wrapper...",
            self.ide_config.name, self.ide_config.wrapper.name
        );
        self.launch_wrapper_with_options(path, options)
    }

    fn is_wrapper_test_mode(&self) -> bool {
        let wrapper_cmd = &self.ide_config.wrapper.command;
        wrapper_cmd == "true" || wrapper_cmd.starts_with("echo ")
    }

    fn launch_wrapper_with_options(&self, path: &Path, options: LaunchOptions) -> Result<()> {
        match self.ide_config.wrapper.name.as_str() {
            "cursor" => self.launch_cursor_wrapper_with_options(path, options),
            "code" => self.launch_vscode_wrapper_with_options(path, options),
            _ => Err(ParaError::ide_error(format!(
                "Unsupported wrapper IDE: '{}'. Please use 'cursor' or 'code' as wrapper.",
                self.ide_config.wrapper.name
            ))),
        }
    }

    fn launch_cursor_wrapper_with_options(
        &self,
        path: &Path,
        options: LaunchOptions,
    ) -> Result<()> {
        self.write_autorun_task_with_options(path, &options)?;

        if self.is_wrapper_test_mode() {
            println!("▶ skipping Cursor wrapper launch (test stub)");
            println!(
                "✅ Cursor wrapper (test stub) opened with {} auto-start",
                self.ide_config.name
            );
            return Ok(());
        }

        let wrapper_cmd = &self.ide_config.wrapper.command;

        if wrapper_cmd.starts_with("echo ") {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(format!("{} \"{}\"", wrapper_cmd, path.display()));
            cmd.output().map_err(|e| {
                ParaError::ide_error(format!("Failed to run wrapper test stub: {e}"))
            })?;
            return Ok(());
        }

        if !crate::config::defaults::is_command_available(wrapper_cmd) {
            return Err(ParaError::ide_error(
                "⚠️  Cursor wrapper CLI not found. Please install Cursor CLI or update your configuration.\n   Falling back to regular Claude Code launch...".to_string()
            ));
        }

        println!(
            "▶ launching Cursor wrapper with {} auto-start...",
            self.ide_config.name
        );
        let mut cmd = Command::new(wrapper_cmd);
        cmd.arg(path.to_string_lossy().as_ref());

        // Detach the IDE process from the parent by redirecting stdio
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        cmd.spawn()
            .map_err(|e| ParaError::ide_error(format!("Failed to launch Cursor wrapper: {e}")))?;
        println!(
            "✅ Cursor opened - {} will start automatically",
            self.ide_config.name
        );

        Ok(())
    }

    fn launch_vscode_wrapper_with_options(
        &self,
        path: &Path,
        options: LaunchOptions,
    ) -> Result<()> {
        self.write_autorun_task_with_options(path, &options)?;

        if self.is_wrapper_test_mode() {
            println!("▶ skipping VS Code wrapper launch (test stub)");
            println!(
                "✅ VS Code wrapper (test stub) opened with {} auto-start",
                self.ide_config.name
            );
            return Ok(());
        }

        let wrapper_cmd = &self.ide_config.wrapper.command;

        if wrapper_cmd.starts_with("echo ") {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(format!("{} \"{}\"", wrapper_cmd, path.display()));
            cmd.output().map_err(|e| {
                ParaError::ide_error(format!("Failed to run wrapper test stub: {e}"))
            })?;
            return Ok(());
        }

        if !crate::config::defaults::is_command_available(wrapper_cmd) {
            return Err(ParaError::ide_error(
                "⚠️  VS Code wrapper CLI not found. Please install VS Code CLI or update your configuration.".to_string()
            ));
        }

        let mut cmd = Command::new(&self.ide_config.wrapper.command);
        cmd.arg(path.to_string_lossy().as_ref());

        // Detach the IDE process from the parent by redirecting stdio
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        println!(
            "▶ launching VS Code wrapper with {} auto-start...",
            self.ide_config.name
        );
        cmd.spawn()
            .map_err(|e| ParaError::ide_error(format!("Failed to launch VS Code wrapper: {e}")))?;
        println!(
            "✅ VS Code opened - {} will start automatically",
            self.ide_config.name
        );

        Ok(())
    }

    fn write_autorun_task_with_options(&self, path: &Path, options: &LaunchOptions) -> Result<()> {
        let vscode_dir = path.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| {
            ParaError::ide_error(format!("Failed to create .vscode directory: {e}"))
        })?;

        let mut ide_command = self.build_ide_wrapper_command_with_options(options);

        // Apply sandboxing if enabled (with CLI override support)
        let temp_config = crate::config::defaults::default_config();
        let resolver = crate::core::sandbox::config::SandboxResolver::new(&crate::config::Config {
            ide: temp_config.ide,
            directories: temp_config.directories,
            git: temp_config.git,
            session: temp_config.session,
            docker: temp_config.docker,
            setup_script: temp_config.setup_script,
            sandbox: self.sandbox_config.clone(),
        });

        let settings = resolver.resolve_with_network(
            options.sandbox_override.unwrap_or(false),
            options.sandbox_override.map(|v| !v).unwrap_or(false),
            options.sandbox_profile.clone(),
            options.network_sandbox,
            options.allowed_domains.clone(),
        );

        let should_sandbox = settings.enabled && cfg!(target_os = "macos");

        if should_sandbox && !is_sandbox_available() {
            eprintln!(
                "⚠️  Warning: Sandbox is enabled but sandbox-exec is not available on this system"
            );
        }

        let mut needs_wrapper_script = false;
        let mut proxy_port = DEFAULT_PROXY_PORT;
        let mut sandbox_allowed_domains = vec![];

        if should_sandbox && is_sandbox_available() {
            // Determine profile and proxy settings
            let (profile, proxy_address) = if settings.network_sandbox {
                // For network sandboxing, use the proxied profile
                (
                    "standard-proxied",
                    Some(format!("127.0.0.1:{DEFAULT_PROXY_PORT}")),
                )
            } else {
                (settings.profile.as_str(), None)
            };

            let sandbox_options = SandboxOptions {
                profile: profile.to_string(),
                proxy_address,
                allowed_domains: settings.allowed_domains.clone(),
            };

            match wrap_command_with_sandbox(&ide_command, path, &sandbox_options) {
                Ok(sandboxed_cmd) => {
                    if settings.network_sandbox {
                        println!("🔒 Network-isolated sandboxing enabled for Claude CLI");
                        if !settings.allowed_domains.is_empty() {
                            println!(
                                "   Additional allowed domains: {}",
                                settings.allowed_domains.join(", ")
                            );
                        }
                    } else {
                        println!("🔒 Sandboxing enabled for Claude CLI");
                    }

                    ide_command = sandboxed_cmd.command;
                    needs_wrapper_script = sandboxed_cmd.needs_wrapper_script;
                    if let Some(port) = sandboxed_cmd.proxy_port {
                        proxy_port = port;
                    }
                    sandbox_allowed_domains = sandboxed_cmd.allowed_domains;
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to apply sandbox: {e}");
                    eprintln!("   Continuing without sandboxing");
                }
            }
        }

        let task_label = format!("Start {}", self.ide_config.name);
        let task_json = self.generate_ide_task_json(&task_label, &ide_command);

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, task_json)
            .map_err(|e| ParaError::ide_error(format!("Failed to write tasks.json: {e}")))?;

        // For network sandboxing, create a temporary script that the task will execute
        if needs_wrapper_script {
            // Create the sandboxed command script
            let script_path = vscode_dir.join("para-sandbox-launcher.sh");

            // Generate the wrapper script
            let script_content = generate_network_sandbox_wrapper(
                &ide_command,
                proxy_port,
                &sandbox_allowed_domains,
            );

            fs::write(&script_path, script_content).map_err(|e| {
                ParaError::ide_error(format!("Failed to write launcher script: {e}"))
            })?;

            // Make it executable on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms)?;
            }

            // Update the task to run the script instead of the complex command
            let script_command = "./.vscode/para-sandbox-launcher.sh";
            let updated_task_json = self.generate_ide_task_json(&task_label, script_command);
            fs::write(&tasks_file, updated_task_json)
                .map_err(|e| ParaError::ide_error(format!("Failed to update tasks.json: {e}")))?;

            println!("\n📝 Network sandboxing configured");
            println!("   VS Code will run the sandboxed Claude when you allow the task");
            println!("   The launcher script will self-delete after execution");
        }

        Ok(())
    }

    fn build_ide_wrapper_command_with_options(&self, options: &LaunchOptions) -> String {
        let mut base_cmd = self.ide_config.command.clone();

        if self.ide_config.name.as_str() == "claude" {
            if options.skip_permissions {
                base_cmd.push_str(" --dangerously-skip-permissions");
            }

            // Handle session continuation with proper quoting
            if let Some(ref session_id) = options.claude_session_id {
                if !session_id.is_empty() {
                    base_cmd.push_str(&format!(" -r \"{session_id}\""));

                    // Add prompt if provided
                    if let Some(ref prompt) = options.prompt {
                        // Escape quotes in prompt and add it
                        let escaped_prompt = prompt.replace('"', "\\\"");
                        base_cmd.push_str(&format!(" \"{escaped_prompt}\""));
                    }
                } else {
                    // Empty session ID, fall back to -c
                    base_cmd.push_str(" -c");
                }
            } else if options.continue_conversation {
                // Fallback to -c flag if no session ID
                base_cmd.push_str(" -c");
            }
        }

        base_cmd
    }

    fn generate_ide_task_json(&self, label: &str, command: &str) -> String {
        // Escape quotes in the command for JSON
        let escaped_command = command.replace('"', "\\\"");
        format!(
            r#"{{
    "version": "2.0.0",
    "tasks": [
        {{
            "label": "{label}",
            "type": "shell",
            "command": "{escaped_command}",
            "group": "build",
            "presentation": {{
                "echo": true,
                "reveal": "always",
                "focus": true,
                "panel": "new",
                "showReuseMessage": false,
                "clear": false
            }},
            "runOptions": {{
                "runOn": "folderOpen"
            }}
        }}
    ]
}}"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(ide_name: &str, ide_command: &str) -> Config {
        Config {
            ide: IdeConfig {
                name: ide_name.to_string(),
                command: ide_command.to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: ".para_state".to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
            docker: None,
            setup_script: None,
            sandbox: None,
        }
    }

    #[test]
    fn test_ide_manager_creation() {
        let config = create_test_config("test-ide", "echo");
        let manager = IdeManager::new(&config);

        assert_eq!(manager.ide_config.name, "test-ide");
        assert_eq!(manager.ide_config.command, "echo");
    }

    #[test]
    fn test_wrapper_mode_requirement() {
        let temp_dir = TempDir::new().unwrap();
        // Test that all IDEs require wrapper mode
        let config = create_test_config("cursor", "cursor");
        let manager = IdeManager::new(&config);

        // Any IDE without wrapper enabled should fail
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("All IDEs require wrapper mode"));
        assert!(error_msg.contains("para config"));
    }

    #[test]
    fn test_wrapper_mode_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("cursor", "true"); // Use test command
        config.ide.wrapper.enabled = true;
        config.ide.wrapper.name = "cursor".to_string();
        config.ide.wrapper.command = "echo".to_string(); // Use echo for testing

        let manager = IdeManager::new(&config);

        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wrapper_task_generation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("claude", "claude");
        config.ide.wrapper.enabled = true;
        config.ide.wrapper.name = "cursor".to_string();
        config.ide.wrapper.command = "echo".to_string();

        let manager = IdeManager::new(&config);

        // Test task generation
        let options = LaunchOptions::default();
        let result = manager.write_autorun_task_with_options(temp_dir.path(), &options);
        assert!(result.is_ok());

        // Check that .vscode/tasks.json was created
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        // Check content
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(content.contains("Start claude"));
        assert!(content.contains("claude"));
        assert!(content.contains("runOn"));
        assert!(content.contains("folderOpen"));
    }

    #[test]
    fn test_unsupported_wrapper() {
        let temp_dir = TempDir::new().unwrap();

        // Use a non-echo command to disable test mode, but keep it deterministic
        let mut config = create_test_config("claude", "para-test-mode-disabled");
        config.ide.wrapper.enabled = true;
        config.ide.wrapper.name = "unsupported-ide".to_string();
        config.ide.wrapper.command = "unsupported-cmd".to_string();

        let manager = IdeManager::new(&config);

        // Test the wrapper launching logic
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported wrapper IDE"));
    }

    #[test]
    fn test_continue_conversation_flag() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("claude", "echo");
        config.ide.wrapper.enabled = true;
        config.ide.wrapper.name = "cursor".to_string();
        config.ide.wrapper.command = "echo".to_string();

        let manager = IdeManager::new(&config);

        // Test with continue_conversation = false
        let options = LaunchOptions {
            skip_permissions: true,
            ..Default::default()
        };
        let result = manager.launch_with_options(temp_dir.path(), options);
        assert!(result.is_ok());

        // Check that tasks.json doesn't contain -c flag
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(!content.contains(" -c"));

        // Test with continue_conversation = true
        let options = LaunchOptions {
            skip_permissions: true,
            continue_conversation: true,
            ..Default::default()
        };
        let result = manager.launch_with_options(temp_dir.path(), options);
        assert!(result.is_ok());

        // Check that tasks.json contains -c flag
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(content.contains(" -c"));
    }

    #[test]
    fn test_build_ide_wrapper_command_options() {
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Test basic command
        let options = LaunchOptions::default();
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude");

        // Test with skip permissions
        let options = LaunchOptions {
            skip_permissions: true,
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude --dangerously-skip-permissions");

        // Test with continue conversation
        let options = LaunchOptions {
            continue_conversation: true,
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude -c");

        // Test with both options
        let options = LaunchOptions {
            skip_permissions: true,
            continue_conversation: true,
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude --dangerously-skip-permissions -c");
    }

    #[test]
    fn test_build_ide_wrapper_command_non_claude() {
        let config = create_test_config("cursor", "cursor");
        let manager = IdeManager::new(&config);

        // Test that non-Claude IDEs just use the base command
        let options = LaunchOptions::default();
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "cursor");

        // Test with options - should still be just the base command
        let options = LaunchOptions {
            skip_permissions: true,
            continue_conversation: true,
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "cursor");
    }

    #[test]
    fn test_build_ide_wrapper_command_with_session_id() {
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Test with session ID only
        let options = LaunchOptions {
            claude_session_id: Some("12345678-1234-1234-1234-123456789012".to_string()),
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude -r \"12345678-1234-1234-1234-123456789012\"");

        // Test with session ID and prompt
        let options = LaunchOptions {
            claude_session_id: Some("12345678-1234-1234-1234-123456789012".to_string()),
            prompt: Some("continue implementing the feature".to_string()),
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude -r \"12345678-1234-1234-1234-123456789012\" \"continue implementing the feature\"");

        // Test with session ID, prompt with quotes
        let options = LaunchOptions {
            claude_session_id: Some("12345678-1234-1234-1234-123456789012".to_string()),
            prompt: Some("add \"test\" functionality".to_string()),
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(
            cmd,
            "claude -r \"12345678-1234-1234-1234-123456789012\" \"add \\\"test\\\" functionality\""
        );

        // Test that session ID takes precedence over continue_conversation
        let options = LaunchOptions {
            claude_session_id: Some("12345678-1234-1234-1234-123456789012".to_string()),
            continue_conversation: true,
            ..Default::default()
        };
        let cmd = manager.build_ide_wrapper_command_with_options(&options);
        assert_eq!(cmd, "claude -r \"12345678-1234-1234-1234-123456789012\"");
        assert!(!cmd.contains(" -c")); // Should not contain -c flag
    }
}
