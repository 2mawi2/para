use crate::config::{Config, IdeConfig};
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct IdeManager {
    config: IdeConfig,
}

impl IdeManager {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.ide.clone(),
        }
    }

    pub fn launch(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        // Check if IDE wrapper is enabled and we're launching Claude Code
        // Like shell version, check environment variable first, then config
        let wrapper_enabled = std::env::var("IDE_WRAPPER_ENABLED")
            .map(|v| v == "true")
            .unwrap_or(self.config.wrapper.enabled);

        if self.config.name == "claude" && wrapper_enabled {
            let wrapper_name = std::env::var("IDE_WRAPPER_NAME")
                .unwrap_or_else(|_| self.config.wrapper.name.clone());
            println!("▶ launching Claude Code inside {} wrapper...", wrapper_name);
            return self.launch_wrapper(path, skip_permissions);
        }

        // Claude Code requires wrapper mode when not in test mode
        if self.config.name == "claude" && !self.config.wrapper.enabled {
            return Err(ParaError::ide_error(
                "Claude Code requires IDE wrapper mode. Please run 'para config' to enable wrapper mode.\n   Available options: VS Code wrapper or Cursor wrapper".to_string()
            ));
        }

        if self.is_test_mode() {
            return self.handle_test_mode(path);
        }

        if !skip_permissions {
            self.check_permissions()?;
        }

        self.validate_ide_availability()?;
        self.validate_path(path)?;

        let path_str = path.to_string_lossy();

        let mut cmd = Command::new(&self.config.command);
        cmd.arg(&*path_str);

        if self.config.name == "claude" {
            cmd.arg("--no-confirm");
        }

        let output = cmd.output().map_err(|e| {
            ParaError::ide_error(format!(
                "Failed to launch IDE '{}': {}",
                self.config.command, e
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ParaError::ide_error(format!(
                "IDE '{}' failed to start: {}",
                self.config.command, stderr
            )));
        }

        Ok(())
    }

    pub fn is_available(&self) -> bool {
        if self.is_test_mode() {
            return true;
        }
        crate::config::defaults::is_command_available(&self.config.command)
    }

    fn validate_ide_availability(&self) -> Result<()> {
        if !self.is_available() {
            return Err(ParaError::ide_not_available(format!(
                "IDE command '{}' is not available in PATH. Please install {} or update your configuration.",
                self.config.command, self.config.name
            )));
        }
        Ok(())
    }

    fn validate_path(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(ParaError::directory_not_found(
                path.to_string_lossy().to_string(),
            ));
        }

        if !path.is_dir() {
            return Err(ParaError::invalid_args(format!(
                "IDE can only be launched on directories, not files: {}",
                path.display()
            )));
        }

        Ok(())
    }

    fn check_permissions(&self) -> Result<()> {
        if self.config.name == "claude" && self.is_in_wrapper_context() {
            println!("⚠️  Claude Code detected running inside another IDE");
            println!("   This may cause permission issues or conflicts");
            println!("   Use --dangerously-skip-permissions to bypass this check");
            return Err(ParaError::permission_denied(
                "Claude Code should not be launched from within another IDE without explicit permission"
            ));
        }

        Ok(())
    }

    fn is_in_wrapper_context(&self) -> bool {
        std::env::var("TERM_PROGRAM").is_ok()
            || std::env::var("VSCODE_INJECTION").is_ok()
            || std::env::var("CURSOR").is_ok()
    }

    pub fn get_config(&self) -> &IdeConfig {
        &self.config
    }

    fn is_test_mode(&self) -> bool {
        // Check environment variable like shell version does
        if let Ok(ide_cmd) = std::env::var("IDE_CMD") {
            return ide_cmd == "true" || ide_cmd.starts_with("echo ");
        }

        // Fall back to config
        self.config.command == "true" || self.config.command.starts_with("echo ")
    }

    fn is_wrapper_test_mode(&self) -> bool {
        // Check wrapper-specific environment variable like shell version does
        if let Ok(wrapper_cmd) = std::env::var("IDE_WRAPPER_CMD") {
            return wrapper_cmd == "true" || wrapper_cmd.starts_with("echo ");
        }

        // Fall back to normal test mode
        self.is_test_mode()
    }

    fn handle_test_mode(&self, path: &Path) -> Result<()> {
        let test_command = std::env::var("IDE_CMD").unwrap_or_else(|_| self.config.command.clone());

        if test_command == "true" {
            println!("▶ skipping {} launch (test stub)", self.config.name);
            println!("✅ {} (test stub) opened", self.config.name);
            return Ok(());
        }

        if test_command.starts_with("echo ") {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(format!("{} \"{}\"", test_command, path.display()));
            cmd.output()
                .map_err(|e| ParaError::ide_error(format!("Failed to run test stub: {}", e)))?;
            return Ok(());
        }

        unreachable!("is_test_mode should only return true for 'true' or 'echo ' commands")
    }

    fn launch_wrapper(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        // Use environment variable first, like shell version
        let wrapper_name =
            std::env::var("IDE_WRAPPER_NAME").unwrap_or_else(|_| self.config.wrapper.name.clone());

        match wrapper_name.as_str() {
            "cursor" => self.launch_cursor_wrapper(path, skip_permissions),
            "code" => self.launch_vscode_wrapper(path, skip_permissions),
            _ => {
                println!("⚠️  Unsupported wrapper IDE: {}", wrapper_name);
                println!("   Falling back to regular Claude Code launch...");
                // Fallback to regular launch like shell version
                self.launch_claude_fallback(path, skip_permissions)
            }
        }
    }

    fn launch_cursor_wrapper(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        self.write_autorun_task(path, skip_permissions)?;

        // Check wrapper-specific test mode like shell version
        if self.is_wrapper_test_mode() {
            println!("▶ skipping Cursor wrapper launch (test stub)");
            println!("✅ Cursor wrapper (test stub) opened with Claude Code auto-start");
            return Ok(());
        }

        // Get wrapper command from environment or config, like shell version
        let wrapper_cmd = std::env::var("IDE_WRAPPER_CMD")
            .unwrap_or_else(|_| self.config.wrapper.command.clone());

        // Handle echo commands like shell version
        if wrapper_cmd.starts_with("echo ") {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(format!("{} \"{}\"", wrapper_cmd, path.display()));
            cmd.output().map_err(|e| {
                ParaError::ide_error(format!("Failed to run wrapper test stub: {}", e))
            })?;
            return Ok(());
        }

        // Check if command exists, like shell version
        if !crate::config::defaults::is_command_available(&wrapper_cmd) {
            return Err(ParaError::ide_error(
                "⚠️  Cursor wrapper CLI not found. Please install Cursor CLI or set IDE_WRAPPER_CMD environment variable.\n   Falling back to regular Claude Code launch...".to_string()
            ));
        }

        println!("▶ launching Cursor wrapper with Claude Code auto-start...");
        let mut cmd = Command::new(&wrapper_cmd);
        cmd.arg(path.to_string_lossy().as_ref());

        // Launch in background like shell version ("&")
        cmd.spawn()
            .map_err(|e| ParaError::ide_error(format!("Failed to launch Cursor wrapper: {}", e)))?;
        println!("✅ Cursor opened - Claude Code will start automatically");

        Ok(())
    }

    fn launch_vscode_wrapper(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        self.write_autorun_task(path, skip_permissions)?;

        if self.is_test_mode() {
            println!("▶ skipping VS Code wrapper launch (test stub)");
            println!("✅ VS Code wrapper (test stub) opened with Claude Code auto-start");
            return Ok(());
        }

        let mut cmd = Command::new(&self.config.wrapper.command);
        cmd.arg(path.to_string_lossy().as_ref());

        println!("▶ launching VS Code wrapper with Claude Code auto-start...");
        cmd.spawn().map_err(|e| {
            ParaError::ide_error(format!("Failed to launch VS Code wrapper: {}", e))
        })?;
        println!("✅ VS Code opened - Claude Code will start automatically");

        Ok(())
    }

    fn write_autorun_task(&self, path: &Path, skip_permissions: bool) -> Result<()> {
        let vscode_dir = path.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| {
            ParaError::ide_error(format!("Failed to create .vscode directory: {}", e))
        })?;

        let claude_command = self.build_claude_wrapper_command(skip_permissions);
        let task_json = self.generate_claude_task_json("Start Claude Code", &claude_command);

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, task_json)
            .map_err(|e| ParaError::ide_error(format!("Failed to write tasks.json: {}", e)))?;

        Ok(())
    }

    fn build_claude_wrapper_command(&self, skip_permissions: bool) -> String {
        let mut base_cmd = "claude".to_string();

        if skip_permissions {
            base_cmd.push_str(" --dangerously-skip-permissions");
        }

        base_cmd
    }

    fn generate_claude_task_json(&self, label: &str, command: &str) -> String {
        format!(
            r#"{{
    "version": "2.0.0",
    "tasks": [
        {{
            "label": "{}",
            "type": "shell",
            "command": "{}",
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
}}"#,
            label, command
        )
    }

    fn launch_claude_fallback(&self, _path: &Path, _skip_permissions: bool) -> Result<()> {
        // This is a fallback when wrapper is unsupported
        // For now, just return an error since we require wrapper mode
        Err(ParaError::ide_error(
            "Claude Code requires supported wrapper mode (cursor or code)".to_string(),
        ))
    }
}

pub fn launch_ide(config: &Config, path: &Path, skip_permissions: bool) -> Result<()> {
    let manager = IdeManager::new(config);
    manager.launch(path, skip_permissions)
}

pub fn validate_ide_availability(config: &Config) -> Result<()> {
    let manager = IdeManager::new(config);
    manager.validate_ide_availability()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::IntegrationStrategy;
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
                default_integration_strategy: IntegrationStrategy::Squash,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_ide_manager_creation() {
        let config = create_test_config("test-ide", "echo");
        let manager = IdeManager::new(&config);

        assert_eq!(manager.config.name, "test-ide");
        assert_eq!(manager.config.command, "echo");
    }

    #[test]
    fn test_validate_path() {
        let config = create_test_config("test-ide", "echo");
        let manager = IdeManager::new(&config);

        let temp_dir = TempDir::new().unwrap();
        assert!(manager.validate_path(temp_dir.path()).is_ok());

        let nonexistent = temp_dir.path().join("nonexistent");
        assert!(manager.validate_path(&nonexistent).is_err());

        let temp_file = temp_dir.path().join("test.txt");
        std::fs::write(&temp_file, "test").unwrap();
        assert!(manager.validate_path(&temp_file).is_err());
    }

    #[test]
    fn test_ide_availability() {
        let config = create_test_config("echo", "echo");
        let manager = IdeManager::new(&config);
        assert!(manager.is_available());

        // In test mode, all commands are considered available
        // This is expected behavior to avoid test failures
        let config = create_test_config("nonexistent", "nonexistent-command-12345");
        let manager = IdeManager::new(&config);

        // Clear test mode temporarily to test actual availability
        let old_ide_cmd = std::env::var("IDE_CMD").ok();
        std::env::remove_var("IDE_CMD");

        assert!(!manager.is_available());

        // Restore test mode if it was set
        if let Some(cmd) = old_ide_cmd {
            std::env::set_var("IDE_CMD", cmd);
        }
    }

    #[test]
    fn test_launch_ide_function() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("echo", "echo");

        // This should succeed because echo is available and temp_dir exists
        // echo will just print the path and exit successfully
        let result = launch_ide(&config, temp_dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_claude_standalone_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Temporarily disable test mode to test actual Claude standalone prevention
        let old_ide_cmd = std::env::var("IDE_CMD").ok();
        std::env::remove_var("IDE_CMD");

        // Claude without wrapper enabled should fail
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Claude Code requires IDE wrapper mode"));
        assert!(error_msg.contains("para config"));

        // Restore test mode if it was set
        if let Some(cmd) = old_ide_cmd {
            std::env::set_var("IDE_CMD", cmd);
        }
    }

    #[test]
    fn test_claude_wrapper_mode_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("claude", "claude");
        config.ide.wrapper.enabled = true;
        config.ide.wrapper.name = "cursor".to_string();
        config.ide.wrapper.command = "echo".to_string(); // Use echo for testing

        let manager = IdeManager::new(&config);

        // Set test mode to avoid actual IDE launch
        std::env::set_var("IDE_CMD", "true");

        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_ok());

        // Cleanup
        std::env::remove_var("IDE_CMD");
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
        let result = manager.write_autorun_task(temp_dir.path(), false);
        assert!(result.is_ok());

        // Check that .vscode/tasks.json was created
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        // Check content
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(content.contains("Start Claude Code"));
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

        // Test the wrapper launching logic directly without environment manipulation
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Claude Code requires supported wrapper mode (cursor or code)"));
    }

    #[test]
    fn test_validate_ide_availability_function() {
        let config = create_test_config("echo", "echo");
        assert!(validate_ide_availability(&config).is_ok());

        let config = create_test_config("nonexistent", "nonexistent-command-12345");
        assert!(validate_ide_availability(&config).is_err());
    }

    #[test]
    fn test_environment_variable_wrapper_override() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Set environment variables to enable wrapper mode
        std::env::set_var("IDE_WRAPPER_ENABLED", "true");
        std::env::set_var("IDE_WRAPPER_NAME", "cursor");
        std::env::set_var("IDE_WRAPPER_CMD", "echo cursor test");

        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_ok());

        // Check that .vscode/tasks.json was created
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        // Check content
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(content.contains("Start Claude Code"));
        assert!(content.contains("claude"));
        assert!(content.contains("runOn"));
        assert!(content.contains("folderOpen"));

        // Cleanup
        std::env::remove_var("IDE_WRAPPER_ENABLED");
        std::env::remove_var("IDE_WRAPPER_NAME");
        std::env::remove_var("IDE_WRAPPER_CMD");
    }
}
