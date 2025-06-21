// This file was moved to resume_modules/validation.rs
use crate::utils::{ParaError, Result};

/// Validates resume command arguments
pub fn validate_resume_args(args: &ResumeArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_resume_args_valid() {
        let args = ResumeArgs {
            session: Some("valid_session".to_string()),
        };
        assert!(validate_resume_args(&args).is_ok());

        let args = ResumeArgs { session: None };
        assert!(validate_resume_args(&args).is_ok());
    }

    #[test]
    fn test_validate_resume_args_invalid() {
        let args = ResumeArgs {
            session: Some("".to_string()),
        };
        assert!(validate_resume_args(&args).is_err());
    }
}