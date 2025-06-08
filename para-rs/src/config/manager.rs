use super::{Config, ConfigError, Result};
use super::defaults::{default_config, get_config_file_path, get_legacy_config_path};
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct ConfigManager;

impl ConfigManager {
    pub fn load_or_create() -> Result<Config> {
        let config_path = get_config_file_path();
        
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let legacy_path = get_legacy_config_path();
            if legacy_path.exists() {
                Self::migrate_legacy_config(&legacy_path, &config_path)
            } else {
                let config = Self::apply_env_overrides(default_config());
                config.validate()?;
                Self::save(&config)?;
                Ok(config)
            }
        }
    }

    pub fn load_from_file(path: &Path) -> Result<Config> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_json::from_str(&content)?;
        config = Self::apply_env_overrides(config);
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

    fn apply_env_overrides(mut config: Config) -> Config {
        if let Ok(ide_name) = std::env::var("PARA_IDE_NAME") {
            config.ide.name = ide_name;
        }

        if let Ok(ide_command) = std::env::var("PARA_IDE_COMMAND") {
            config.ide.command = ide_command;
        }

        if let Ok(branch_prefix) = std::env::var("PARA_BRANCH_PREFIX") {
            config.git.branch_prefix = branch_prefix;
        }

        if let Ok(subtrees_dir) = std::env::var("PARA_SUBTREES_DIR") {
            config.directories.subtrees_dir = subtrees_dir;
        }

        if let Ok(state_dir) = std::env::var("PARA_STATE_DIR") {
            config.directories.state_dir = state_dir;
        }

        if let Ok(auto_stage) = std::env::var("PARA_AUTO_STAGE") {
            config.git.auto_stage = auto_stage.parse().unwrap_or(config.git.auto_stage);
        }

        if let Ok(auto_commit) = std::env::var("PARA_AUTO_COMMIT") {
            config.git.auto_commit = auto_commit.parse().unwrap_or(config.git.auto_commit);
        }

        if let Ok(preserve_on_finish) = std::env::var("PARA_PRESERVE_ON_FINISH") {
            config.session.preserve_on_finish = preserve_on_finish.parse().unwrap_or(config.session.preserve_on_finish);
        }

        config
    }

    fn migrate_legacy_config(legacy_path: &Path, new_path: &Path) -> Result<Config> {
        let legacy_content = fs::read_to_string(legacy_path)?;
        let config = Self::parse_legacy_config(&legacy_content)?;
        
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        Self::save(&config)?;
        
        match fs::rename(legacy_path, legacy_path.with_extension("backup")) {
            Ok(_) => eprintln!("Migrated legacy config from {} to {}", legacy_path.display(), new_path.display()),
            Err(_) => eprintln!("Migrated config but could not backup legacy file"),
        }
        
        Ok(config)
    }

    fn parse_legacy_config(content: &str) -> Result<Config> {
        let mut config = default_config();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                
                match key {
                    "IDE_NAME" => config.ide.name = value.to_string(),
                    "IDE_CMD" => config.ide.command = value.to_string(),
                    "SUBTREES_DIR" => config.directories.subtrees_dir = value.to_string(),
                    "STATE_DIR" => config.directories.state_dir = value.to_string(),
                    "BRANCH_PREFIX" => config.git.branch_prefix = value.to_string(),
                    _ => {}
                }
            }
        }
        
        Ok(config)
    }

    pub fn backup_config() -> Result<()> {
        let config_path = get_config_file_path();
        if !config_path.exists() {
            return Err(ConfigError::NotFound("No config file to backup".to_string()));
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

        if let Err(e) = config.validate() {
            return Err(e);
        }

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
        assert_eq!(original_config.git.branch_prefix, loaded_config.git.branch_prefix);
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("PARA_IDE_NAME", "test_override");
        std::env::set_var("PARA_BRANCH_PREFIX", "override_prefix");
        
        let config = ConfigManager::apply_env_overrides(create_test_config());
        
        assert_eq!(config.ide.name, "test_override");
        assert_eq!(config.git.branch_prefix, "override_prefix");
        
        std::env::remove_var("PARA_IDE_NAME");
        std::env::remove_var("PARA_BRANCH_PREFIX");
    }

    #[test]
    fn test_legacy_config_parsing() {
        let legacy_content = r#"
# Legacy para config
IDE_NAME="cursor"
IDE_CMD="cursor"
SUBTREES_DIR="legacy_subtrees"
BRANCH_PREFIX="legacy"
"#;
        
        let config = ConfigManager::parse_legacy_config(legacy_content).unwrap();
        
        assert_eq!(config.ide.name, "cursor");
        assert_eq!(config.ide.command, "cursor");
        assert_eq!(config.directories.subtrees_dir, "legacy_subtrees");
        assert_eq!(config.git.branch_prefix, "legacy");
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