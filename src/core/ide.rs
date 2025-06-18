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
        self.launch_with_options(path, skip_permissions, false)
    }

    pub fn launch_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        // All IDEs require wrapper mode for cloud-based launching
        if !self.config.wrapper.enabled {
            return Err(ParaError::ide_error(
                "All IDEs require wrapper mode for cloud-based launching. Please run 'para config' to enable wrapper mode.\n   Available options: VS Code wrapper or Cursor wrapper".to_string()
            ));
        }

        println!(
            "▶ launching {} inside {} wrapper...",
            self.config.name, self.config.wrapper.name
        );
        self.launch_wrapper_with_options(path, skip_permissions, continue_conversation)
    }

    fn is_wrapper_test_mode(&self) -> bool {
        // Check if wrapper command is a test command
        let wrapper_cmd = &self.config.wrapper.command;
        wrapper_cmd == "true" || wrapper_cmd.starts_with("echo ")
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
            _ => Err(ParaError::ide_error(format!(
                "Unsupported wrapper IDE: '{}'. Please use 'cursor' or 'code' as wrapper.",
                self.config.wrapper.name
            ))),
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
            println!(
                "✅ Cursor wrapper (test stub) opened with {} auto-start",
                self.config.name
            );
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

        println!(
            "▶ launching Cursor wrapper with {} auto-start...",
            self.config.name
        );
        let mut cmd = Command::new(wrapper_cmd);
        cmd.arg(path.to_string_lossy().as_ref());

        // Launch in background like shell version ("&")
        cmd.spawn()
            .map_err(|e| ParaError::ide_error(format!("Failed to launch Cursor wrapper: {}", e)))?;
        println!(
            "✅ Cursor opened - {} will start automatically",
            self.config.name
        );

        Ok(())
    }

    fn launch_vscode_wrapper_with_options(
        &self,
        path: &Path,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> Result<()> {
        self.write_autorun_task_with_options(path, skip_permissions, continue_conversation)?;

        if self.is_wrapper_test_mode() {
            println!("▶ skipping VS Code wrapper launch (test stub)");
            println!(
                "✅ VS Code wrapper (test stub) opened with {} auto-start",
                self.config.name
            );
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
                "⚠️  VS Code wrapper CLI not found. Please install VS Code CLI or update your configuration.".to_string()
            ));
        }

        let mut cmd = Command::new(&self.config.wrapper.command);
        cmd.arg(path.to_string_lossy().as_ref());

        println!(
            "▶ launching VS Code wrapper with {} auto-start...",
            self.config.name
        );
        cmd.spawn().map_err(|e| {
            ParaError::ide_error(format!("Failed to launch VS Code wrapper: {}", e))
        })?;
        println!(
            "✅ VS Code opened - {} will start automatically",
            self.config.name
        );

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

        let ide_command =
            self.build_ide_wrapper_command_with_options(skip_permissions, continue_conversation);
        let task_label = format!("Start {}", self.config.name);
        let task_json = self.generate_ide_task_json(&task_label, &ide_command);

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, task_json)
            .map_err(|e| ParaError::ide_error(format!("Failed to write tasks.json: {}", e)))?;

        Ok(())
    }

    fn build_ide_wrapper_command_with_options(
        &self,
        skip_permissions: bool,
        continue_conversation: bool,
    ) -> String {
        let mut base_cmd = self.config.command.clone();

        // Add IDE-specific flags
        match self.config.name.as_str() {
            "claude" => {
                if skip_permissions {
                    base_cmd.push_str(" --dangerously-skip-permissions");
                }
                if continue_conversation {
                    base_cmd.push_str(" -c");
                }
            }
            _ => {
                // For non-Claude IDEs, we just use the base command
                // Additional flags can be added here in the future if needed
            }
        }

        base_cmd
    }

    fn generate_ide_task_json(&self, label: &str, command: &str) -> String {
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
        let result = manager.write_autorun_task_with_options(temp_dir.path(), false, false);
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
    fn test_build_ide_wrapper_command_options() {
        let config = create_test_config("claude", "claude");
        let manager = IdeManager::new(&config);

        // Test basic command
        let cmd = manager.build_ide_wrapper_command_with_options(false, false);
        assert_eq!(cmd, "claude");

        // Test with skip permissions
        let cmd = manager.build_ide_wrapper_command_with_options(true, false);
        assert_eq!(cmd, "claude --dangerously-skip-permissions");

        // Test with continue conversation
        let cmd = manager.build_ide_wrapper_command_with_options(false, true);
        assert_eq!(cmd, "claude -c");

        // Test with both options
        let cmd = manager.build_ide_wrapper_command_with_options(true, true);
        assert_eq!(cmd, "claude --dangerously-skip-permissions -c");
    }

    #[test]
    fn test_build_ide_wrapper_command_non_claude() {
        let config = create_test_config("cursor", "cursor");
        let manager = IdeManager::new(&config);

        // Test that non-Claude IDEs just use the base command
        let cmd = manager.build_ide_wrapper_command_with_options(false, false);
        assert_eq!(cmd, "cursor");

        // Test with options - should still be just the base command
        let cmd = manager.build_ide_wrapper_command_with_options(true, true);
        assert_eq!(cmd, "cursor");
    }
}
