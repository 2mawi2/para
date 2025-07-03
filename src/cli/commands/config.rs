use crate::cli::parser::{ConfigArgs, ConfigCommands};
use crate::config::{self, ConfigManager};
use crate::utils::{ParaError, Result};
use std::process::Command;

pub fn execute(args: ConfigArgs) -> Result<()> {
    match args.command {
        Some(ConfigCommands::Setup) => execute_setup(),
        Some(ConfigCommands::Auto) => execute_auto(),
        Some(ConfigCommands::Show) => execute_show(),
        Some(ConfigCommands::Edit) => execute_edit(),
        Some(ConfigCommands::Reset) => execute_reset(),
        Some(ConfigCommands::Set { path, value }) => execute_set(&path, &value),
        None => execute_default(),
    }
}

fn execute_setup() -> Result<()> {
    config::run_config_wizard()
        .map_err(|e| ParaError::config_error(format!("Configuration wizard failed: {e}")))?;
    println!("✅ Configuration wizard completed successfully");
    Ok(())
}

fn execute_auto() -> Result<()> {
    config::run_quick_setup()
        .map_err(|e| ParaError::config_error(format!("Auto-configuration failed: {e}")))?;
    println!("✅ Auto-configuration completed successfully");
    Ok(())
}

fn execute_show() -> Result<()> {
    match ConfigManager::load_or_create() {
        Ok(config) => {
            println!("{}", serde_json::to_string_pretty(&config).unwrap());
            Ok(())
        }
        Err(e) => Err(ParaError::config_error(format!(
            "Failed to load configuration: {e}"
        ))),
    }
}

fn execute_edit() -> Result<()> {
    let config_path = ConfigManager::get_config_path()
        .map_err(|e| ParaError::config_error(format!("Failed to get config path: {e}")))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| ParaError::config_error(format!("Failed to launch editor: {e}")))?;

    if !status.success() {
        return Err(ParaError::config_error(format!(
            "Editor exited with non-zero status: {}",
            status.code().unwrap_or(-1)
        )));
    }

    println!("✅ Configuration file edited successfully");
    Ok(())
}

fn is_non_interactive() -> bool {
    std::env::var("PARA_NON_INTERACTIVE").is_ok()
        || std::env::var("CI").is_ok()
        || !atty::is(atty::Stream::Stdin)
}

fn execute_reset() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm};

    if is_non_interactive() {
        return Err(ParaError::invalid_args(
            "Cannot reset configuration in non-interactive mode. Run interactively to confirm reset."
        ));
    }

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(
            "Are you sure you want to reset configuration to defaults? This cannot be undone.",
        )
        .default(false)
        .interact()
        .map_err(|e| ParaError::config_error(format!("Failed to read input: {e}")))?
    {
        println!("❌ Configuration reset cancelled");
        return Ok(());
    }

    let default_config = crate::config::defaults::default_config();
    ConfigManager::save(&default_config).map_err(|e| {
        ParaError::config_error(format!("Failed to save default configuration: {e}"))
    })?;

    println!("✅ Configuration reset to defaults successfully");
    Ok(())
}

fn execute_default() -> Result<()> {
    config::run_config_wizard()
        .map_err(|e| ParaError::config_error(format!("Configuration wizard failed: {e}")))?;
    println!("✅ Configuration wizard completed successfully");
    Ok(())
}

fn execute_set(path: &str, value: &str) -> Result<()> {
    let config_path = std::path::PathBuf::from(
        ConfigManager::get_config_path()
            .map_err(|e| ParaError::config_error(format!("Failed to get config path: {e}")))?,
    );

    // Load existing config as JSON value for manipulation
    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|e| ParaError::config_error(format!("Failed to read config file: {e}")))?;

    let mut json_value: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| ParaError::config_error(format!("Invalid JSON in config file: {e}")))?;

    // Parse the path and set the value
    set_json_value(&mut json_value, path, value)?;

    // Write back to file
    let updated_json = serde_json::to_string_pretty(&json_value)
        .map_err(|e| ParaError::config_error(format!("Failed to serialize config: {e}")))?;

    std::fs::write(&config_path, updated_json)
        .map_err(|e| ParaError::config_error(format!("Failed to write config file: {e}")))?;

    println!("✅ Configuration updated: {path} = {value}");
    Ok(())
}

fn set_json_value(json_value: &mut serde_json::Value, path: &str, value: &str) -> Result<()> {
    if path.trim().is_empty() {
        return Err(ParaError::config_error("Empty path provided".to_string()));
    }

    let path_parts: Vec<&str> = path.split('.').collect();

    // Navigate to the parent object
    let mut current = json_value;
    for part in &path_parts[..path_parts.len() - 1] {
        current = current
            .as_object_mut()
            .ok_or_else(|| {
                ParaError::config_error(format!("Path component '{part}' is not an object"))
            })?
            .get_mut(*part)
            .ok_or_else(|| ParaError::config_error(format!("Path component '{part}' not found")))?;
    }

    // Set the final value
    let final_key = path_parts[path_parts.len() - 1];
    let current_obj = current.as_object_mut().ok_or_else(|| {
        ParaError::config_error(format!("Parent of '{final_key}' is not an object"))
    })?;

    // Convert value based on simple heuristics
    let parsed_value = parse_config_value(value);
    current_obj.insert(final_key.to_string(), parsed_value);

    Ok(())
}

fn parse_config_value(value: &str) -> serde_json::Value {
    // Simple type heuristics
    match value {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        _ => {
            // Try to parse as number
            if let Ok(int_val) = value.parse::<i64>() {
                serde_json::Value::Number(serde_json::Number::from(int_val))
            } else if let Ok(float_val) = value.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(float_val) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::String(value.to_string())
                }
            } else {
                // Default to string
                serde_json::Value::String(value.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config_json() -> String {
        serde_json::json!({
            "ide": {
                "name": "test-ide",
                "command": "echo",
                "user_data_dir": null,
                "wrapper": {
                    "enabled": true,
                    "name": "test-wrapper",
                    "command": "echo"
                }
            },
            "directories": {
                "subtrees_dir": ".para/subtrees",
                "state_dir": ".para/state"
            },
            "git": {
                "branch_prefix": "para",
                "auto_stage": true,
                "auto_commit": false
            },
            "session": {
                "default_name_format": "%Y-%m-%d-%H%M%S",
                "preserve_on_finish": false,
                "auto_cleanup_days": 30
            }
        })
        .to_string()
    }

    #[test]
    fn test_parse_config_value_boolean() {
        assert_eq!(parse_config_value("true"), serde_json::Value::Bool(true));
        assert_eq!(parse_config_value("false"), serde_json::Value::Bool(false));
    }

    #[test]
    fn test_parse_config_value_number() {
        assert_eq!(
            parse_config_value("42"),
            serde_json::Value::Number(serde_json::Number::from(42))
        );
        assert_eq!(
            parse_config_value("2.5"),
            serde_json::Value::Number(serde_json::Number::from_f64(2.5).unwrap())
        );
    }

    #[test]
    fn test_parse_config_value_string() {
        assert_eq!(
            parse_config_value("hello"),
            serde_json::Value::String("hello".to_string())
        );
        assert_eq!(
            parse_config_value("cursor"),
            serde_json::Value::String("cursor".to_string())
        );
    }

    #[test]
    fn test_set_json_value_simple_path() {
        let mut json_value = serde_json::from_str(&create_test_config_json()).unwrap();

        // Test setting IDE name
        set_json_value(&mut json_value, "ide.name", "cursor").unwrap();
        assert_eq!(json_value["ide"]["name"], "cursor");

        // Test setting boolean
        set_json_value(&mut json_value, "git.auto_stage", "false").unwrap();
        assert_eq!(json_value["git"]["auto_stage"], false);

        // Test setting number
        set_json_value(&mut json_value, "session.auto_cleanup_days", "14").unwrap();
        assert_eq!(json_value["session"]["auto_cleanup_days"], 14);
    }

    #[test]
    fn test_set_json_value_nested_path() {
        let mut json_value = serde_json::from_str(&create_test_config_json()).unwrap();

        // Test setting wrapper command
        set_json_value(&mut json_value, "ide.wrapper.command", "cursor").unwrap();
        assert_eq!(json_value["ide"]["wrapper"]["command"], "cursor");

        // Test setting wrapper enabled
        set_json_value(&mut json_value, "ide.wrapper.enabled", "false").unwrap();
        assert_eq!(json_value["ide"]["wrapper"]["enabled"], false);
    }

    #[test]
    fn test_set_json_value_invalid_path() {
        let mut json_value = serde_json::from_str(&create_test_config_json()).unwrap();

        // Test invalid path component
        let result = set_json_value(&mut json_value, "nonexistent.field", "value");
        assert!(result.is_err());

        // Test empty path
        let result = set_json_value(&mut json_value, "", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_set_integration_isolated() {
        // Create isolated temporary config file
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        // Write initial config
        fs::write(&config_path, create_test_config_json()).unwrap();

        // Test the core logic without using ConfigManager::get_config_path()
        let config_content = fs::read_to_string(&config_path).unwrap();
        let mut json_value: serde_json::Value = serde_json::from_str(&config_content).unwrap();

        // Test setting various values
        set_json_value(&mut json_value, "ide.name", "cursor").unwrap();
        set_json_value(&mut json_value, "git.auto_stage", "false").unwrap();
        set_json_value(&mut json_value, "session.auto_cleanup_days", "7").unwrap();

        // Write back and verify
        let updated_json = serde_json::to_string_pretty(&json_value).unwrap();
        fs::write(&config_path, updated_json).unwrap();

        // Reload and verify changes
        let reloaded_content = fs::read_to_string(&config_path).unwrap();
        let reloaded_json: serde_json::Value = serde_json::from_str(&reloaded_content).unwrap();

        assert_eq!(reloaded_json["ide"]["name"], "cursor");
        assert_eq!(reloaded_json["git"]["auto_stage"], false);
        assert_eq!(reloaded_json["session"]["auto_cleanup_days"], 7);
    }

    #[test]
    fn test_common_claude_code_use_cases() {
        let mut json_value = serde_json::from_str(&create_test_config_json()).unwrap();

        // Test most common Claude Code configuration changes
        set_json_value(&mut json_value, "ide.name", "cursor").unwrap();
        assert_eq!(json_value["ide"]["name"], "cursor");

        set_json_value(&mut json_value, "ide.command", "cursor").unwrap();
        assert_eq!(json_value["ide"]["command"], "cursor");

        set_json_value(&mut json_value, "git.auto_stage", "true").unwrap();
        assert_eq!(json_value["git"]["auto_stage"], true);

        set_json_value(&mut json_value, "git.auto_commit", "false").unwrap();
        assert_eq!(json_value["git"]["auto_commit"], false);

        set_json_value(&mut json_value, "session.preserve_on_finish", "true").unwrap();
        assert_eq!(json_value["session"]["preserve_on_finish"], true);
    }
}
