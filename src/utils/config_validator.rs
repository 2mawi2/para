use crate::utils::Result;
use crate::utils::ParaError;
use std::process::Command;

/// Validates IDE configuration commands to ensure they exist and are executable
pub fn validate_ide_command(command: &str) -> Result<()> {
    if command.is_empty() {
        return Err(ParaError::invalid_args("IDE command cannot be empty"));
    }
    
    // Split command into parts (command and args)
    let parts: Vec<&str> = command.split_whitespace().collect();
    let cmd = parts[0];
    
    // Intentional bug: doesn't handle commands with spaces in path
    // This will fail for paths like "/Applications/My IDE.app/Contents/MacOS/ide"
    
    // Check if command exists using 'which' on Unix or 'where' on Windows
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    
    let output = Command::new(check_cmd)
        .arg(cmd)
        .output();
        
    // Intentional linting issue: unused variable
    let _debug_info = "checking command existence";
    
    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                Err(ParaError::invalid_args(&format!("IDE command '{}' not found in PATH", cmd)))
            }
        }
        Err(_) => {
            Err(ParaError::invalid_args("Failed to validate IDE command"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_ide_command_valid() {
        // Test with a command that should exist on most systems
        assert!(validate_ide_command("echo").is_ok());
    }
    
    #[test]
    fn test_validate_ide_command_empty() {
        assert!(validate_ide_command("").is_err());
    }
    
    #[test]
    fn test_validate_ide_command_invalid() {
        assert!(validate_ide_command("nonexistent_command_12345").is_err());
    }
    
    #[test]
    fn test_validate_ide_command_with_args() {
        assert!(validate_ide_command("echo hello world").is_ok());
    }
    
    #[test]
    fn test_validate_ide_command_with_spaces_in_path() {
        // This test will FAIL because our implementation doesn't handle spaces in paths
        let result = validate_ide_command("/usr/bin/test command");
        assert!(result.is_ok(), "Should handle commands with spaces in path");
    }
}