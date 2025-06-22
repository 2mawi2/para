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

    // Check for spaces
    if name.contains(' ') {
        return Err(ParaError::invalid_args(
            "Session name cannot contain spaces",
        ));
    }

    // Check for invalid characters that would cause issues with git branch names
    let invalid_chars = [
        '~', '^', ':', '?', '*', '[', '\\', ' ', '\t', '\n', '\r', '\x7f', // DEL character
    ];

    for ch in name.chars() {
        if invalid_chars.contains(&ch) || ch.is_control() {
            return Err(ParaError::invalid_args(format!(
                "Session name cannot contain invalid character: '{}'",
                ch
            )));
        }
    }

    // Check for sequences that are problematic in git
    if name.starts_with('.') || name.ends_with('.') {
        return Err(ParaError::invalid_args(
            "Session name cannot start or end with '.'",
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(ParaError::invalid_args(
            "Session name cannot start or end with '-'",
        ));
    }

    if name.contains("..") {
        return Err(ParaError::invalid_args("Session name cannot contain '..'"));
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
        let result = validate_session_name("my feature");
        assert!(result.is_err(), "Should reject names with spaces");
    }

    #[test]
    fn test_validate_session_name_with_invalid_chars() {
        let invalid_names = vec![
            "name~with~tilde",
            "name^with^caret",
            "name:with:colon",
            "name?with?question",
            "name*with*asterisk",
            "name[with[bracket",
            "name\\with\\backslash",
            "name\twith\ttab",
            "name\nwith\nnewline",
        ];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Should reject name with invalid chars: {}",
                name
            );
        }
    }

    #[test]
    fn test_validate_session_name_with_problematic_sequences() {
        let invalid_names = vec![
            ".starts-with-dot",
            "ends-with-dot.",
            "-starts-with-dash",
            "ends-with-dash-",
            "has..double..dots",
        ];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(result.is_err(), "Should reject problematic name: {}", name);
        }
    }

    #[test]
    fn test_validate_session_name_valid_edge_cases() {
        let valid_names = vec![
            "a", // minimal valid name
            "feature-123",
            "my_feature",
            "feature.v1", // dots in middle are ok
            "UPPER_case",
            "mixed-Case_123",
        ];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(result.is_ok(), "Should accept valid name: {}", name);
        }
    }
}
