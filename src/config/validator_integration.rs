// Integration file to demonstrate usage of the new config validator
use crate::utils::config_validator::validate_ide_command;
use crate::utils::Result;

pub fn validate_config_ide_settings(ide_command: &str) -> Result<()> {
    validate_ide_command(ide_command)?;
    println!("IDE command '{}' validated successfully", ide_command);
    Ok(())
}