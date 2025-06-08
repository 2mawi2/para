use crate::cli::parser::{ConfigArgs, ConfigCommands};
use crate::config::{self, Config, ConfigManager};
use crate::utils::{ParaError, Result};

pub fn execute(args: ConfigArgs) -> Result<()> {
    match args.command {
        Some(ConfigCommands::Setup) => execute_setup(),
        Some(ConfigCommands::Auto) => execute_auto(),
        Some(ConfigCommands::Show) => execute_show(),
        Some(ConfigCommands::Edit) => execute_edit(),
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
    Err(ParaError::not_implemented("config edit command"))
}

fn execute_default() -> Result<()> {
    config::run_config_wizard()
        .map_err(|e| ParaError::config_error(format!("Configuration wizard failed: {}", e)))?;
    println!("✅ Configuration wizard completed successfully");
    Ok(())
}
