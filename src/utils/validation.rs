use crate::utils::ParaError;
use crate::utils::Result;

/// Validates that a session name is valid for use as a git branch name
///
/// Session names must:
/// - Not be empty
/// - Not contain spaces or special characters
/// - Be less than 50 characters
pub fn validate_session_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_args("Session name cannot be empty"));
    }

    if name.len() > 50 {
        return Err(ParaError::invalid_args(
            "Session name must be less than 50 characters",
        ));
    }

    // Check for spaces and special characters that are invalid in git branch names
    if name.contains(' ') {
        return Err(ParaError::invalid_args(
            "Session name cannot contain spaces",
        ));
    }

    // Check for other characters that are problematic in git branch names
    let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', '.', '@', '{'];
    if name.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(ParaError::invalid_args(
            "Session name contains invalid characters for git branch names",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_session_name_valid() {
        assert!(validate_session_name("my-feature").is_ok());
        assert!(validate_session_name("feature123").is_ok());
    }

    #[test]
    fn test_validate_session_name_empty() {
        assert!(validate_session_name("").is_err());
    }

    #[test]
    fn test_validate_session_name_too_long() {
        let long_name = "a".repeat(51);
        assert!(validate_session_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_session_name_with_spaces() {
        // This test will FAIL because our implementation doesn't check for spaces
        let result = validate_session_name("my feature");
        assert!(result.is_err(), "Should reject names with spaces");
    }
}
