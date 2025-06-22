use crate::utils::ParaError;
use crate::utils::Result;
use std::process::Command;

/// Validates IDE configuration commands to ensure they exist and are executable
pub fn validate_ide_command(command: &str) -> Result<()> {
    if command.is_empty() {
        return Err(ParaError::invalid_args("IDE command cannot be empty"));
    }

    // Extract the command name from the command string
    // Handle commands with spaces in their paths by finding the first space that's not inside quotes
    let cmd = if command.starts_with('"') && command.contains('"') {
        // Handle quoted commands like: "C:\Program Files\My IDE\ide.exe" --args
        let end_quote = command[1..].find('"').unwrap_or(command.len() - 1) + 1;
        &command[1..end_quote]
    } else {
        // For unquoted commands, take everything up to the first space
        command.split_whitespace().next().unwrap_or(command)
    };

    // Check if command exists using 'which' on Unix or 'where' on Windows
    let check_cmd = if cfg!(windows) { "where" } else { "which" };

    let output = Command::new(check_cmd).arg(cmd).output();

    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                Err(ParaError::invalid_args(format!(
                    "IDE command '{}' not found in PATH",
                    cmd
                )))
            }
        }
        Err(_) => Err(ParaError::invalid_args("Failed to validate IDE command")),
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
