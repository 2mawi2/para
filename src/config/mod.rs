use serde::{Deserialize, Serialize};

pub mod defaults;
pub mod manager;
pub mod path;
pub mod validation;
pub mod wizard;

pub use manager::ConfigManager;
pub use wizard::{run_config_wizard, run_quick_setup};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub ide: IdeConfig,
    pub directories: DirectoryConfig,
    pub git: GitConfig,
    pub session: SessionConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IdeConfig {
    pub name: String,
    pub command: String,
    pub user_data_dir: Option<String>,
    pub wrapper: WrapperConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WrapperConfig {
    pub enabled: bool,
    pub name: String,
    pub command: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DirectoryConfig {
    pub subtrees_dir: String,
    pub state_dir: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitConfig {
    pub branch_prefix: String,
    pub auto_stage: bool,
    pub auto_commit: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionConfig {
    pub default_name_format: String,
    pub preserve_on_finish: bool,
    pub auto_cleanup_days: Option<u32>,
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Json(e) => write!(f, "JSON error: {}", e),
            ConfigError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        ConfigError::Io(error)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        ConfigError::Json(error)
    }
}

impl Config {
    pub fn load_or_create() -> Result<Self> {
        ConfigManager::load_or_create()
    }

    pub fn validate(&self) -> Result<()> {
        validation::validate_config(self)
    }

    #[cfg(test)]
    pub fn validate_no_cmd_check(&self) -> Result<()> {
        validation::validate_ide_config_no_cmd_check(&self.ide)?;
        validation::validate_directory_config(&self.directories)?;
        validation::validate_git_config(&self.git)?;
        validation::validate_session_config(&self.session)?;
        Ok(())
    }

    pub fn get_branch_prefix(&self) -> &str {
        &self.git.branch_prefix
    }

    pub fn is_wrapper_enabled(&self) -> bool {
        self.ide.wrapper.enabled
    }

    pub fn get_state_dir(&self) -> &str {
        &self.directories.state_dir
    }

    pub fn should_auto_stage(&self) -> bool {
        self.git.auto_stage
    }

    pub fn should_preserve_on_finish(&self) -> bool {
        self.session.preserve_on_finish
    }

    pub fn is_real_ide_environment(&self) -> bool {
        !cfg!(test) && self.ide.command != "echo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_real_ide_environment() {
        let mut config = defaults::default_config();

        config.ide.command = "echo".to_string();
        assert!(!config.is_real_ide_environment());

        config.ide.command = "cursor".to_string();
        assert!(!config.is_real_ide_environment());
    }

    #[test]
    fn test_config_getter_methods() {
        let config = Config {
            ide: IdeConfig {
                name: "test-ide".to_string(),
                command: "echo".to_string(),
                user_data_dir: Some("/test/data".to_string()),
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "test-wrapper".to_string(),
                    command: "echo".to_string(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "custom/subtrees".to_string(),
                state_dir: "custom/state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "feature".to_string(),
                auto_stage: false,
                auto_commit: true,
            },
            session: SessionConfig {
                default_name_format: "%Y-%m-%d".to_string(),
                preserve_on_finish: true,
                auto_cleanup_days: Some(14),
            },
        };

        assert_eq!(config.get_branch_prefix(), "feature");
        assert!(config.is_wrapper_enabled());
        assert_eq!(config.get_state_dir(), "custom/state");
        assert!(!config.should_auto_stage());
        assert!(config.should_preserve_on_finish());
    }

    #[test]
    fn test_config_error_display() {
        use std::io;

        let io_error = ConfigError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert_eq!(io_error.to_string(), "IO error: file not found");

        let json_str = r#"{"invalid": json}"#;
        let json_error: Result<Config> =
            Err(serde_json::from_str::<Config>(json_str).unwrap_err().into());
        if let Err(ConfigError::Json(e)) = json_error {
            assert!(e.to_string().contains("expected value"));
        } else {
            panic!("Expected JSON error");
        }

        let validation_error = ConfigError::Validation("Invalid configuration".to_string());
        assert_eq!(
            validation_error.to_string(),
            "Validation error: Invalid configuration"
        );
    }

    #[test]
    fn test_config_validation_integration() {
        let valid_config = Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: "state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        };
        assert!(valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.ide.name = String::new();
        let result = invalid_config.validate();
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert_eq!(msg, "IDE name cannot be empty");
        }
    }

    #[test]
    fn test_wrapper_config_validation() {
        let config_wrapper_disabled = Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: "state".to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        };
        assert!(config_wrapper_disabled.validate().is_ok());

        let mut config_wrapper_enabled = config_wrapper_disabled.clone();
        config_wrapper_enabled.ide.wrapper = WrapperConfig {
            enabled: true,
            name: "claude".to_string(),
            command: "echo".to_string(),
        };
        assert!(config_wrapper_enabled.validate().is_ok());

        let mut config_invalid_wrapper = config_wrapper_enabled.clone();
        config_invalid_wrapper.ide.wrapper.name = String::new();
        let result = config_invalid_wrapper.validate();
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert_eq!(msg, "Wrapper name cannot be empty when wrapper is enabled");
        }
    }

    #[test]
    fn test_config_environment_override() {
        // This test verifies that config loading respects paths, but without
        // modifying global environment variables which breaks parallel tests

        let temp_dir = TempDir::new().unwrap();
        let custom_config_path = temp_dir.path().join("custom_config.json");

        // Create a test config with mock IDE for CI compatibility
        let test_config = Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: defaults::default_directory_config(),
            git: defaults::default_git_config(),
            session: defaults::default_session_config(),
        };
        let config_json = serde_json::to_string_pretty(&test_config).unwrap();
        std::fs::write(&custom_config_path, config_json).unwrap();

        // Test that we can load from a specific path
        let loaded = ConfigManager::load_from_file(&custom_config_path).unwrap();
        assert_eq!(loaded.git.branch_prefix, test_config.git.branch_prefix);

        // Note: We cannot safely test environment variable override in parallel tests
        // That functionality is tested in integration tests or with serial execution
    }

    #[test]
    fn test_config_load_or_create_isolated() {
        // This test verifies config persistence and loading functionality
        // We pre-create a config to avoid IDE detection issues in CI environments
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        // Create a mock config for testing
        let test_config = Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: defaults::default_directory_config(),
            git: defaults::default_git_config(),
            session: defaults::default_session_config(),
        };

        // Test 1: Manually save config and verify it can be loaded
        let config_json = serde_json::to_string_pretty(&test_config).unwrap();
        std::fs::write(&config_path, config_json).unwrap();
        assert!(config_path.exists());

        // Load the config and verify it matches what we saved
        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.ide.name, "test");
        assert_eq!(loaded_config.ide.command, "echo");
        assert!(!loaded_config.git.branch_prefix.is_empty());

        // Test 2: Verify load_or_create loads existing config without modifying it
        let config = ConfigManager::load_or_create_with_path(Some(&config_path)).unwrap();
        assert_eq!(config.ide.name, "test");
        assert_eq!(config.ide.command, "echo");

        // Verify the file content is still valid JSON
        let file_content = std::fs::read_to_string(&config_path).unwrap();
        assert!(!file_content.is_empty());
        let parsed: serde_json::Value = serde_json::from_str(&file_content).unwrap();
        assert!(parsed.is_object());

        // Test 3: Loading again should return the same config
        let loaded_again = ConfigManager::load_or_create_with_path(Some(&config_path)).unwrap();
        assert_eq!(loaded_again.git.branch_prefix, config.git.branch_prefix);
        assert_eq!(loaded_again.ide.name, config.ide.name);
        assert_eq!(loaded_again.ide.command, "echo");

        // Note: We don't test the "create" path here because it would try to detect
        // the system IDE, which fails in CI. The create functionality is tested
        // separately in the defaults module tests.
    }
}
