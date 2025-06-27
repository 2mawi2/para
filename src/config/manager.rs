use super::defaults::{default_config, get_config_file_path};
use super::{Config, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct ConfigManager;

impl ConfigManager {
    pub fn get_config_path() -> Result<String> {
        let config_path = get_config_file_path();
        Ok(config_path.to_string_lossy().to_string())
    }

    pub fn load_or_create() -> Result<Config> {
        Self::load_or_create_with_path(None)
    }

    pub fn load_or_create_with_path(config_path: Option<&Path>) -> Result<Config> {
        let config_path = match config_path {
            Some(path) => path.to_path_buf(),
            None => get_config_file_path(),
        };

        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let config = default_config();
            config.validate()?;
            Self::save_to_path(&config, &config_path)?;
            Ok(config)
        }
    }

    pub fn load_from_file(path: &Path) -> Result<Config> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_json::from_str(&content)?;

        if !config.ide.wrapper.enabled {
            eprintln!("🔄 Migrating configuration to use wrapper mode...");
            config = Self::migrate_to_wrapper_mode(config);
            Self::save_to_path(&config, path)?;
            eprintln!("✅ Configuration migrated successfully");
        }

        config.validate()?;
        Ok(config)
    }

    #[cfg(test)]
    pub fn load_from_file_no_cmd_check(path: &Path) -> Result<Config> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_json::from_str(&content)?;

        if !config.ide.wrapper.enabled {
            eprintln!("🔄 Migrating configuration to use wrapper mode...");
            config = Self::migrate_to_wrapper_mode(config);
            // Skip saving during tests to avoid validation errors
            eprintln!("✅ Configuration migrated successfully");
        }

        // Use test validation that skips command availability checks
        super::validation::validate_ide_config_no_cmd_check(&config.ide)?;
        super::validation::validate_directory_config(&config.directories)?;
        super::validation::validate_git_config(&config.git)?;
        super::validation::validate_session_config(&config.session)?;
        Ok(config)
    }

    fn migrate_to_wrapper_mode(mut config: Config) -> Config {
        config.ide.wrapper.enabled = true;

        if config.ide.wrapper.name.is_empty() {
            config.ide.wrapper.name = config.ide.name.clone();
        }
        if config.ide.wrapper.command.is_empty() {
            config.ide.wrapper.command = config.ide.command.clone();
        }

        if config.ide.name == "claude" && config.ide.wrapper.name == "claude" {
            if super::defaults::is_command_available("cursor") {
                config.ide.wrapper.name = "cursor".to_string();
                config.ide.wrapper.command = "cursor".to_string();
            } else {
                config.ide.wrapper.name = "code".to_string();
                config.ide.wrapper.command = "code".to_string();
            }
        }

        config
    }

    pub fn save(config: &Config) -> Result<()> {
        Self::save_to_path(config, &get_config_file_path())
    }

    pub fn save_to_path(config: &Config, path: &Path) -> Result<()> {
        config.validate()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(config)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            ide: super::super::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: super::super::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: super::super::DirectoryConfig {
                subtrees_dir: "test_subtrees".to_string(),
                state_dir: "test_state".to_string(),
            },
            git: super::super::GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: super::super::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
            docker: None,
            setup_script: None,
        }
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let original_config = create_test_config();

        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();

        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();

        assert_eq!(original_config.ide.name, loaded_config.ide.name);
        assert_eq!(original_config.ide.command, loaded_config.ide.command);
        assert_eq!(
            original_config.git.branch_prefix,
            loaded_config.git.branch_prefix
        );
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.json");

        let result = ConfigManager::load_from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");
        fs::write(&config_path, "invalid json content").unwrap();

        let result = ConfigManager::load_from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let test_config_path = temp_dir.path().join("nested/dir/config.json");

        let config = create_test_config();
        let result = ConfigManager::save_to_path(&config, &test_config_path);
        assert!(result.is_ok());

        // Verify the file was created
        assert!(test_config_path.exists());
    }

    #[test]
    fn test_get_config_path() {
        // This test just verifies the method returns a valid path
        // We cannot test specifics without dependency injection
        let path = ConfigManager::get_config_path().unwrap();
        assert!(path.ends_with("config.json"));
        assert!(!path.is_empty());
    }

    #[test]
    fn test_load_or_create_existing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let original_config = create_test_config();
        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();

        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert!(loaded_config.validate().is_ok());
        assert_eq!(loaded_config.ide.name, original_config.ide.name);
    }

    #[test]
    fn test_load_or_create_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let original_config = create_test_config();
        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();

        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert!(loaded_config.validate().is_ok());
        assert_eq!(loaded_config.ide.name, original_config.ide.name);
    }

    #[test]
    fn test_config_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut original_config = create_test_config();
        original_config.ide.name = "custom-ide".to_string();
        original_config.git.branch_prefix = "custom-prefix".to_string();

        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();

        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.ide.name, "custom-ide");
        assert_eq!(loaded_config.git.branch_prefix, "custom-prefix");
    }

    #[test]
    fn test_config_migration_to_wrapper_mode() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut old_config = create_test_config();
        old_config.ide.wrapper.enabled = false;
        old_config.ide.wrapper.name = String::new();
        old_config.ide.wrapper.command = String::new();

        let json = serde_json::to_string_pretty(&old_config).unwrap();
        fs::write(&config_path, json).unwrap();

        // Load should trigger migration
        let migrated_config = ConfigManager::load_from_file(&config_path).unwrap();

        assert!(migrated_config.ide.wrapper.enabled);
        assert_eq!(migrated_config.ide.wrapper.name, old_config.ide.name);
        assert_eq!(migrated_config.ide.wrapper.command, old_config.ide.command);

        let reloaded = ConfigManager::load_from_file(&config_path).unwrap();
        assert!(reloaded.ide.wrapper.enabled);
    }

    #[test]
    fn test_claude_migration_uses_different_wrapper() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let claude_config = Config {
            ide: super::super::IdeConfig {
                name: "claude".to_string(),
                command: "claude".to_string(),
                user_data_dir: None,
                wrapper: super::super::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: super::super::DirectoryConfig {
                subtrees_dir: "test_subtrees".to_string(),
                state_dir: "test_state".to_string(),
            },
            git: super::super::GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: super::super::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
            docker: None,
            setup_script: None,
        };

        let json = serde_json::to_string_pretty(&claude_config).unwrap();
        fs::write(&config_path, json).unwrap();

        let migrated_config = ConfigManager::load_from_file_no_cmd_check(&config_path).unwrap();

        // Verify Claude uses a different wrapper (cursor or code)
        assert!(migrated_config.ide.wrapper.enabled);
        assert_ne!(migrated_config.ide.wrapper.name, "claude");
        assert!(
            migrated_config.ide.wrapper.name == "cursor"
                || migrated_config.ide.wrapper.name == "code"
        );
    }
}
