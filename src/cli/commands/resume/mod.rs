use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::Result;

pub mod orchestrator;
pub mod session_detector;
pub mod task_transformer;

pub use orchestrator::ResumeOrchestrator;
pub use session_detector::{SessionDetector, SessionResumeInfo};
pub use task_transformer::{TaskConfiguration, TaskTransformation, TaskTransformer};

pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    
    let orchestrator = ResumeOrchestrator::new(&config, &git_service, &session_manager);
    orchestrator.execute(&args)
}