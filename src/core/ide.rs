use crate::config::{Config, IdeConfig};
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};
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
        self.launch_with_options(path, skip_permissions, false)
    }

    pub fn launch_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        // Check if IDE wrapper is enabled and we're launching Claude Code
        if self.config.name == "claude" && self.config.wrapper.enabled {
            println!(
                "▶ launching Claude Code inside {} wrapper...",
                self.config.wrapper.name
            );
            return self.launch_wrapper_with_options(path, skip_permissions, continue_conversation);
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
            if continue_conversation {
                cmd.arg("-c");
            }
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

    fn is_test_mode(&self) -> bool {
        // Check if IDE command is a test command
        self.config.command == "true" || self.config.command.starts_with("echo ")
    }

    fn is_wrapper_test_mode(&self) -> bool {
        // Check if wrapper command is a test command
        let wrapper_cmd = &self.config.wrapper.command;
        wrapper_cmd == "true" || wrapper_cmd.starts_with("echo ")
    }

    fn handle_test_mode(&self, path: &Path) -> Result<()> {
        let test_command = &self.config.command;

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

    fn launch_wrapper_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        match self.config.wrapper.name.as_str() {
            "cursor" => self.launch_cursor_wrapper_with_options(
                path,
                skip_permissions,
                continue_conversation,
            ),
            "code" => self.launch_vscode_wrapper_with_options(
                path,
                skip_permissions,
                continue_conversation,
            ),
            _ => {
                println!("⚠️  Unsupported wrapper IDE: {}", self.config.wrapper.name);
                println!("   Falling back to regular Claude Code launch...");
                // Fallback to regular launch like shell version
                self.launch_claude_fallback(path, skip_permissions)
            }
        }
    }

    fn launch_cursor_wrapper_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        self.write_autorun_task_with_options(path, skip_permissions, continue_conversation)?;

        // Check wrapper-specific test mode like shell version
        if self.is_wrapper_test_mode() {
            println!("▶ skipping Cursor wrapper launch (test stub)");
            println!("✅ Cursor wrapper (test stub) opened with Claude Code auto-start");
            return Ok(());
        }

        // Get wrapper command from config
        let wrapper_cmd = &self.config.wrapper.command;

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

        // Check if command exists
        if !crate::config::defaults::is_command_available(wrapper_cmd) {
            return Err(ParaError::ide_error(
                "⚠️  Cursor wrapper CLI not found. Please install Cursor CLI or update your configuration.\n   Falling back to regular Claude Code launch...".to_string()
            ));
        }

        println!("▶ launching Cursor wrapper with Claude Code auto-start...");
        let mut cmd = Command::new(wrapper_cmd);
        cmd.arg(path.to_string_lossy().as_ref());

        // Launch in background like shell version ("&")
        cmd.spawn()
            .map_err(|e| ParaError::ide_error(format!("Failed to launch Cursor wrapper: {}", e)))?;
        println!("✅ Cursor opened - Claude Code will start automatically");

        Ok(())
    }

    fn launch_vscode_wrapper_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        self.write_autorun_task_with_options(path, skip_permissions, continue_conversation)?;

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

    fn write_autorun_task_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        let vscode_dir = path.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| {
            ParaError::ide_error(format!("Failed to create .vscode directory: {}", e))
        })?;

        let claude_command =
            self.build_claude_wrapper_command_with_options(skip_permissions, continue_conversation);
        let task_json = self.generate_claude_task_json("Start Claude Code", &claude_command);

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, task_json)
            .map_err(|e| ParaError::ide_error(format!("Failed to write tasks.json: {}", e)))?;

        Ok(())
    }

    fn build_claude_wrapper_command_with_options(
        &self,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> String {
        let mut base_cmd = "claude".to_string();

        if skip_permissions {
            base_cmd.push_str(" --dangerously-skip-permissions");
        }

        if continue_conversation {
            base_cmd.push_str(" -c");
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

    /// Check if an IDE is already running for a given session
    pub fn is_ide_running_for_session(&self, session_name: &str) -> bool {
        // In test mode, always return false
        if self.is_test_mode() || self.is_wrapper_test_mode() {
            return false;
        }

        // Try to read the launch file to understand how the IDE was launched
        let state_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".para_state");

        let launch_file = state_dir.join(format!("{}.launch", session_name));

        if !launch_file.exists() {
            // No launch file means IDE was not launched via dispatch, check anyway
            return self.check_ide_process_running(None);
        }

        // Parse launch file to determine which IDE to check
        if let Ok(content) = fs::read_to_string(&launch_file) {
            if content.contains("LAUNCH_METHOD=wrapper") {
                // Extract wrapper IDE name
                for line in content.lines() {
                    if line.starts_with("WRAPPER_IDE=") {
                        let wrapper_ide = line.trim_start_matches("WRAPPER_IDE=").trim();
                        return self.check_ide_process_running(Some(wrapper_ide));
                    }
                }
            } else if content.contains("LAUNCH_METHOD=ide") {
                // Direct IDE launch - check configured IDE
                return self.check_ide_process_running(Some(&self.config.name));
            }
        }

        // Default: check configured IDE
        self.check_ide_process_running(None)
    }

    /// Platform-specific check if an IDE process is running
    fn check_ide_process_running(&self, ide_name: Option<&str>) -> bool {
        // Determine which IDE to check
        let ide_to_check = if let Some(name) = ide_name {
            name
        } else if self.config.wrapper.enabled {
            &self.config.wrapper.name
        } else {
            &self.config.name
        };

        // Platform-specific process detection
        #[cfg(target_os = "macos")]
        {
            self.check_macos_ide_running(ide_to_check)
        }

        #[cfg(not(target_os = "macos"))]
        {
            // For non-macOS platforms, we can't reliably detect running processes
            // Return false to allow the continuation flag to be added
            false
        }
    }

    #[cfg(target_os = "macos")]
    fn check_macos_ide_running(&self, ide_name: &str) -> bool {
        // Map IDE names to macOS application names
        let app_name = match ide_name {
            "cursor" => "Cursor",
            "code" => "Code",
            "vscode" => "Code",
            _ => return false, // Unknown IDE, assume not running
        };

        // Use pgrep to check if the application is running
        let output = Command::new("pgrep").arg("-x").arg(app_name).output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
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
        // Test with echo command (test mode)
        let config = create_test_config("echo", "echo");
        let manager = IdeManager::new(&config);
        assert!(manager.is_available());

        // Test with actual nonexistent command (non-test mode)
        let config = create_test_config("nonexistent", "nonexistent-command-12345");
        let manager = IdeManager::new(&config);
        assert!(!manager.is_available());
    }

    #[test]
    fn test_claude_standalone_prevention() {
        let temp_dir = TempDir::new().unwrap();
        // Use a non-test command to test actual Claude standalone prevention
        let config = create_test_config("claude", "real-claude-command");
        let manager = IdeManager::new(&config);

        // Claude without wrapper enabled should fail
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Claude Code requires IDE wrapper mode"));
        assert!(error_msg.contains("para config"));
    }

    #[test]
    fn test_claude_wrapper_mode_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("claude", "true"); // Use test command
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
        let result = manager.write_autorun_task_with_options(temp_dir.path(), false, false);
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

        // Test the wrapper launching logic
        let result = manager.launch(temp_dir.path(), true);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Claude Code requires supported wrapper mode (cursor or code)"));
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
        let result = manager.launch_with_options(temp_dir.path(), true, false);
        assert!(result.is_ok());

        // Check that tasks.json doesn't contain -c flag
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");
        assert!(tasks_file.exists());
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(!content.contains(" -c"));

        // Test with continue_conversation = true
        let result = manager.launch_with_options(temp_dir.path(), true, true);
        assert!(result.is_ok());

        // Check that tasks.json contains -c flag
        let content = std::fs::read_to_string(&tasks_file).unwrap();
        assert!(content.contains(" -c"));
    }

    #[test]
    fn test_ide_process_detection() {
        let config = create_test_config("claude", "echo");
        let manager = IdeManager::new(&config);

        // In test mode, should always return false
        assert!(!manager.is_ide_running_for_session("test-session"));
    }

    #[test]
    fn test_build_claude_wrapper_command_options() {
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Test basic command
        let cmd = manager.build_claude_wrapper_command_with_options(false, false);
        assert_eq!(cmd, "claude");

        // Test with skip permissions
        let cmd = manager.build_claude_wrapper_command_with_options(true, false);
        assert_eq!(cmd, "claude --dangerously-skip-permissions");

        // Test with continue conversation
        let cmd = manager.build_claude_wrapper_command_with_options(false, true);
        assert_eq!(cmd, "claude -c");

        // Test with both options
        let cmd = manager.build_claude_wrapper_command_with_options(true, true);
        assert_eq!(cmd, "claude --dangerously-skip-permissions -c");
    }
}
