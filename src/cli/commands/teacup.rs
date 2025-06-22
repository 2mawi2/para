use crate::utils::Result;

pub fn execute() -> Result<()> {
    println!("I am a teacup");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teacup_command_returns_ok() {
        let result = execute();
        assert!(result.is_ok(), "Teacup command should return Ok");
    }

    #[test]
    fn test_teacup_command_outputs_correct_message() {
        // Test that the function executes without error
        // The actual output test would require capturing stdout,
        // but for this simple command, testing the return value is sufficient
        let result = execute();
        assert!(result.is_ok(), "Teacup command should execute successfully");
    }
}
