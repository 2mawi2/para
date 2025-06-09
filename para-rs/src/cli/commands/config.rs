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
        None => execute_default(),
    }
}

fn execute_setup() -> Result<()> {
    config::run_config_wizard()
        .map_err(|e| ParaError::config_error(format!("Configuration wizard failed: {}", e)))?;
    println!("✅ Configuration wizard completed successfully");
    Ok(())
}

fn execute_auto() -> Result<()> {
    config::run_quick_setup()
        .map_err(|e| ParaError::config_error(format!("Auto-configuration failed: {}", e)))?;
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
            "Failed to load configuration: {}",
            e
        ))),
    }
}

fn execute_edit() -> Result<()> {
    let config_path = ConfigManager::get_config_path()
        .map_err(|e| ParaError::config_error(format!("Failed to get config path: {}", e)))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    
    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| ParaError::config_error(format!("Failed to launch editor: {}", e)))?;

    if !status.success() {
        return Err(ParaError::config_error(format!(
            "Editor exited with non-zero status: {}",
            status.code().unwrap_or(-1)
        )));
    }

    println!("✅ Configuration file edited successfully");
    Ok(())
}

fn execute_reset() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm};
    
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure you want to reset configuration to defaults? This cannot be undone.")
        .default(false)
        .interact()
        .map_err(|e| ParaError::config_error(format!("Failed to read input: {}", e)))?
    {
        println!("❌ Configuration reset cancelled");
        return Ok(());
    }

    let default_config = crate::config::defaults::default_config();
    ConfigManager::save(&default_config)
        .map_err(|e| ParaError::config_error(format!("Failed to save default configuration: {}", e)))?;
    
    println!("✅ Configuration reset to defaults successfully");
    Ok(())
}

fn execute_default() -> Result<()> {
    config::run_config_wizard()
        .map_err(|e| ParaError::config_error(format!("Configuration wizard failed: {}", e)))?;
    println!("✅ Configuration wizard completed successfully");
    Ok(())
}
