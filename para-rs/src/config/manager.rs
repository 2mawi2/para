use super::defaults::{default_config, get_config_file_path};
use super::{Config, ConfigError, Result};
use crate::cli::parser::IntegrationStrategy;
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

    pub fn backup_config() -> Result<()> {
        let config_path = get_config_file_path();
        if !config_path.exists() {
            return Err(ConfigError::NotFound(
                "No config file to backup".to_string(),
            ));
        }

        let backup_path = config_path.with_extension("json.backup");
        fs::copy(&config_path, &backup_path)?;

        Ok(())
    }

    pub fn restore_backup() -> Result<Config> {
        let backup_path = get_config_file_path().with_extension("json.backup");
        if !backup_path.exists() {
            return Err(ConfigError::NotFound("No backup file found".to_string()));
        }

        let config_path = get_config_file_path();
        fs::copy(&backup_path, &config_path)?;

        Self::load_from_file(&config_path)
    }

    pub fn reset_to_defaults() -> Result<Config> {
        Self::backup_config().ok(); // Backup if possible, but don't fail if it doesn't exist

        let config = default_config();
        Self::save(&config)?;
        Ok(config)
    }

    pub fn validate_and_fix(config: &mut Config) -> Result<Vec<String>> {
        let mut fixes = Vec::new();

        if !super::defaults::is_command_available(&config.ide.command) {
            let (detected_name, detected_command) = super::defaults::detect_ide();
            if super::defaults::is_command_available(&detected_command) {
                config.ide.name = detected_name;
                config.ide.command = detected_command.clone();
                fixes.push(format!("Fixed IDE command to '{}'", detected_command));
            }
        }

        config.validate()?;

        Ok(fixes)
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
    fn test_validate_and_fix() {
        let mut config = create_test_config();
        config.ide.command = "nonexistent_command".to_string();

        let fixes = ConfigManager::validate_and_fix(&mut config).unwrap();
        assert!(!fixes.is_empty());
        assert_ne!(config.ide.command, "nonexistent_command");
    }
}
