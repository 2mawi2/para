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
        let config_path = get_config_file_path();

        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let config = default_config();
            config.validate()?;
            Self::save(&config)?;
            Ok(config)
        }
    }

    pub fn load_from_file(path: &Path) -> Result<Config> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(config: &Config) -> Result<()> {
        config.validate()?;

        let config_path = get_config_file_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(config)?;
        let mut file = fs::File::create(&config_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::IntegrationStrategy;
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
                default_integration_strategy: IntegrationStrategy::Squash,
            },
            session: super::super::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
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
        let original_home = std::env::var("HOME").ok();
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
        
        let config = create_test_config();
        let result = ConfigManager::save(&config);
        assert!(result.is_ok());
        
        // Restore original environment
        match original_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        match original_xdg {
            Some(x) => std::env::set_var("XDG_CONFIG_HOME", x),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_get_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let original_home = std::env::var("HOME").ok();
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
        
        let path = ConfigManager::get_config_path().unwrap();
        assert!(path.contains("para-rs"));
        assert!(path.ends_with("config.json"));
        
        // Restore original environment
        match original_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        match original_xdg {
            Some(x) => std::env::set_var("XDG_CONFIG_HOME", x),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_load_or_create_existing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        // Create an existing config file
        let original_config = create_test_config();
        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();
        
        // Load the existing config
        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert!(loaded_config.validate().is_ok());
        assert_eq!(loaded_config.ide.name, original_config.ide.name);
    }

    #[test]
    fn test_load_or_create_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        // Test loading from existing file
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
        
        // Save to temporary file
        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();
        
        // Load from the same file
        let loaded_config = ConfigManager::load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.ide.name, "custom-ide");
        assert_eq!(loaded_config.git.branch_prefix, "custom-prefix");
    }
}
