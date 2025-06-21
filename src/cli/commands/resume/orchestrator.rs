use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::ide::IdeManager;
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};

use super::session_detector::{SessionDetector, SessionResumeInfo};
use super::task_transformer::TaskTransformer;

pub struct ResumeOrchestrator<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
    task_transformer: TaskTransformer,
}

impl<'a> ResumeOrchestrator<'a> {
    pub fn new(
        config: &'a Config,
        git_service: &'a GitService,
        session_manager: &'a SessionManager,
    ) -> Self {
        Self {
            config,
            git_service,
            session_manager,
            task_transformer: TaskTransformer::new(),
        }
    }

    pub fn execute(&self, args: &ResumeArgs) -> Result<()> {
        self.validate_args(args)?;

        match &args.session {
            Some(session_name) => self.resume_specific_session(session_name),
            None => self.resume_detected_session(),
        }
    }

    fn resume_specific_session(&self, session_name: &str) -> Result<()> {
        let session_detector = SessionDetector::new(self.config, self.git_service, self.session_manager);
        let resume_info = session_detector.find_specific_session(session_name)?;

        self.finalize_resume(&resume_info, Some(session_name))
    }

    fn resume_detected_session(&self) -> Result<()> {
        let session_detector = SessionDetector::new(self.config, self.git_service, self.session_manager);
        let resume_info = session_detector.detect_and_resume_session()?;

        if resume_info.session_name.is_none() && resume_info.worktree_path.as_os_str().is_empty() {
            // No session found and user didn't select one
            return Ok(());
        }

        let session_name = resume_info.session_name.as_deref();
        self.finalize_resume(&resume_info, session_name)
    }

    fn finalize_resume(&self, resume_info: &SessionResumeInfo, session_name: Option<&str>) -> Result<()> {
        let session_detector = SessionDetector::new(self.config, self.git_service, self.session_manager);
        
        // Ensure CLAUDE.local.md exists for the session
        session_detector.ensure_claude_local_md(resume_info)?;

        // Launch IDE for the session
        self.launch_ide_for_session(&resume_info.worktree_path)?;

        // Print success message
        match (session_name, resume_info.requires_selection) {
            (Some(name), true) => println!("✅ Resumed session '{}'", name),
            (Some(name), false) => println!("✅ Resumed session '{}'", name),
            (None, false) => println!("✅ Resumed current session"),
            (None, true) => println!("✅ Resumed session at '{}'", resume_info.worktree_path.display()),
        }

        Ok(())
    }

    fn launch_ide_for_session(&self, worktree_path: &std::path::Path) -> Result<()> {
        let ide_manager = IdeManager::new(self.config);

        // For Claude Code in wrapper mode, always use continuation flag when resuming
        if self.config.ide.name == "claude" && self.config.ide.wrapper.enabled {
            println!("▶ resuming Claude Code session with conversation continuation...");
            // Update existing tasks.json to include -c flag
            self.task_transformer.update_tasks_json_for_resume(worktree_path)?;
            ide_manager.launch_with_options(worktree_path, false, true)
        } else {
            ide_manager.launch(worktree_path, false)
        }
    }

    fn validate_args(&self, args: &ResumeArgs) -> Result<()> {
        if let Some(ref session) = args.session {
            if session.is_empty() {
                return Err(ParaError::invalid_args(
                    "Session identifier cannot be empty",
                ));
            }
        }
        Ok(())
    }
}