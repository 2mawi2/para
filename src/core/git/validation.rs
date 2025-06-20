use crate::utils::error::{ParaError, Result};
use regex::Regex;
use std::path::Path;

/// Centralized Git-related validation utilities
pub struct GitValidator;

impl GitValidator {
    /// Validate a Git branch name according to Git's naming rules
    pub fn validate_branch_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(ParaError::git_operation(
                "Branch name cannot be empty".to_string(),
            ));
        }

        if name.len() > 250 {
            return Err(ParaError::git_operation("Branch name too long".to_string()));
        }

        let invalid_patterns = vec![
            r"\.\.+",              // Contains ..
            r"^-",                 // Starts with -
            r"/$",                 // Ends with /
            r"\x00",               // Contains null byte
            r"[ \t]",              // Contains whitespace
            r"[\x00-\x1f\x7f]",    // Contains control characters
            r"~|\^|:|\\|\*|\?|\[", // Contains special Git characters
            r"^@$",                // Exactly "@"
            r"/\.",                // Contains "/.
            r"\.\.",               // Contains ".."
            r"@\{",                // Contains "@{"
        ];

        for pattern in invalid_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| ParaError::git_operation(format!("Regex error: {}", e)))?;
            if regex.is_match(name) {
                return Err(ParaError::git_operation(format!(
                    "Invalid branch name '{}': contains invalid characters or patterns",
                    name
                )));
            }
        }

        if name.starts_with("refs/") {
            return Err(ParaError::git_operation(
                "Branch name cannot start with 'refs/'".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a worktree path to ensure it's suitable for creating a worktree
    pub fn validate_worktree_path(path: &Path, repo_root: &Path) -> Result<()> {
        if path == repo_root {
            return Err(ParaError::git_operation(
                "Cannot create worktree at repository root".to_string(),
            ));
        }

        if let Ok(canonical_path) = path.canonicalize() {
            if canonical_path == repo_root {
                return Err(ParaError::git_operation(
                    "Cannot create worktree at repository root (canonical path)".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_branch_name_valid() {
        assert!(GitValidator::validate_branch_name("valid-branch").is_ok());
        assert!(GitValidator::validate_branch_name("feature/test").is_ok());
        assert!(GitValidator::validate_branch_name("v1.0.0").is_ok());
        assert!(GitValidator::validate_branch_name("main").is_ok());
    }

    #[test]
    fn test_validate_branch_name_invalid() {
        let invalid_names = vec![
            "",
            "branch..name",
            "-invalid",
            "invalid/",
            "branch name",
            "@",
            "branch@{",
            "branch~1",
            "refs/heads/test",
        ];

        for invalid_name in invalid_names {
            assert!(
                GitValidator::validate_branch_name(invalid_name).is_err(),
                "Should reject invalid branch name: {}",
                invalid_name
            );
        }
    }

    #[test]
    fn test_validate_branch_name_too_long() {
        let long_name = "a".repeat(251);
        assert!(GitValidator::validate_branch_name(&long_name).is_err());
    }
}
