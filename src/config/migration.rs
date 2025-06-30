use crate::config::Config;
use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Migrate configuration to latest format
pub fn migrate_config(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(config_path)?;
    let mut config_value: Value = serde_json::from_str(&content)?;

    let mut migrated = false;

    // Migration 1: Add sandbox field if missing
    if config_value.get("sandbox").is_none() {
        config_value["sandbox"] = serde_json::json!({
            "enabled": false,
            "profile": "permissive-open"
        });
        migrated = true;
        eprintln!("Migrating config: Adding sandbox configuration (disabled by default)");
    }

    // Future migrations can be added here
    // if config_value.get("new_field").is_none() { ... }

    if migrated {
        let pretty_json = serde_json::to_string_pretty(&config_value)?;
        fs::write(config_path, pretty_json)?;
        eprintln!("Configuration migration completed successfully");
    }

    Ok(())
}

/// Load config with migration
pub fn load_config_with_migration(config_path: &Path) -> Result<Config> {
    // Run migrations first
    migrate_config(config_path)?;

    // Then load normally
    let content = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migrate_config_adds_sandbox_field() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create old config without sandbox field
        let old_config = r#"{
            "ide": {
                "name": "claude",
                "command": "claude",
                "wrapper": {
                    "enabled": true,
                    "name": "cursor",
                    "command": "cursor"
                }
            },
            "git": {
                "branch_prefix": "para"
            },
            "directories": {
                "subtrees_dir": ".para/worktrees",
                "state_dir": ".para/state"
            }
        }"#;

        fs::write(&config_path, old_config).unwrap();

        // Run migration
        migrate_config(&config_path).unwrap();

        // Load and verify
        let content = fs::read_to_string(&config_path).unwrap();
        let config_value: Value = serde_json::from_str(&content).unwrap();

        assert!(config_value.get("sandbox").is_some());
        assert_eq!(config_value["sandbox"]["enabled"], false);
        assert_eq!(config_value["sandbox"]["profile"], "permissive-open");
    }

    #[test]
    fn test_migrate_config_preserves_existing_sandbox() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create config with existing sandbox field
        let existing_config = r#"{
            "ide": {
                "name": "claude",
                "command": "claude",
                "wrapper": {
                    "enabled": true,
                    "name": "cursor",
                    "command": "cursor"
                }
            },
            "sandbox": {
                "enabled": true,
                "profile": "restrictive-closed"
            }
        }"#;

        fs::write(&config_path, existing_config).unwrap();

        // Run migration
        migrate_config(&config_path).unwrap();

        // Load and verify sandbox wasn't changed
        let content = fs::read_to_string(&config_path).unwrap();
        let config_value: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(config_value["sandbox"]["enabled"], true);
        assert_eq!(config_value["sandbox"]["profile"], "restrictive-closed");
    }

    #[test]
    fn test_load_config_with_migration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create minimal old config with required fields
        let old_config = r#"{
            "ide": {
                "name": "claude",
                "command": "claude",
                "wrapper": {
                    "enabled": false,
                    "name": "code",
                    "command": "code"
                }
            },
            "directories": {
                "subtrees_dir": ".para/worktrees",
                "state_dir": ".para/state"
            },
            "git": {
                "branch_prefix": "para",
                "auto_stage": true,
                "auto_commit": false
            },
            "session": {
                "default_name_format": "%Y%m%d-%H%M%S",
                "preserve_on_finish": false
            }
        }"#;

        fs::write(&config_path, old_config).unwrap();

        // Load with migration
        let config = load_config_with_migration(&config_path).unwrap();

        // Verify migration happened
        assert!(config.sandbox.is_some());
        assert!(!config.sandbox.as_ref().unwrap().enabled);
        assert_eq!(config.sandbox.as_ref().unwrap().profile, "permissive-open");
    }
}
