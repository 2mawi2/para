use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::Result;

mod execution;
mod session_detection;
mod task_transformation;
mod validation;

use execution::launch_ide_for_session;
use session_detection::{detect_and_resume_session, resume_specific_session};
use validation::validate_resume_args;

/// Main entry point for the resume command
pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => resume_specific_session(&config, &git_service, &session_name),
        None => detect_and_resume_session(&config, &git_service, &session_manager),
    }
}