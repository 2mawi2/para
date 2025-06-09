use crate::config::{Config, IdeConfig};
use crate::utils::{ParaError, Result};
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
        let config = create_test_config("echo", "echo");
        let manager = IdeManager::new(&config);
        assert!(manager.is_available());

        let config = create_test_config("nonexistent", "nonexistent-command-12345");
        let manager = IdeManager::new(&config);
        assert!(!manager.is_available());
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
    fn test_validate_ide_availability_function() {
        let config = create_test_config("echo", "echo");
        assert!(validate_ide_availability(&config).is_ok());

        let config = create_test_config("nonexistent", "nonexistent-command-12345");
        assert!(validate_ide_availability(&config).is_err());
    }
}
