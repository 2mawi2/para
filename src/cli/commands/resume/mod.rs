use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};

mod claude_session;
mod context;
mod repair;
mod session;
mod task_transform;

// Public API is exposed through the execute function only

/// Main entry point - orchestrates resume logic
pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    args.validate()?;
    validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match &args.session {
        Some(session_name) => {
            session::resume_specific_session(&config, &git_service, session_name, &args)
        }
        None => session::detect_and_resume_session(&config, &git_service, &session_manager, &args),
    }
}

fn validate_resume_args(args: &ResumeArgs) -> Result<()> {
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
    use crate::cli::parser::{ResumeArgs, SandboxArgs};
    use std::path::PathBuf;

    #[test]
    fn test_resume_args_validate() {
        // Test valid cases
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: None,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_ok());

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(PathBuf::from("test.md")),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_ok());

        // Test invalid case - both prompt and file
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: Some(PathBuf::from("test.md")),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_err());
        assert!(args
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both"));
    }
}
