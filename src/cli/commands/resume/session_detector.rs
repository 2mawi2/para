use crate::cli::commands::common::create_claude_local_md;
use crate::config::Config;
use crate::core::git::{GitService, SessionEnvironment};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::{Path, PathBuf};

pub struct SessionDetector<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
}

impl<'a> SessionDetector<'a> {
    pub fn new(
        config: &'a Config,
        git_service: &'a GitService,
        session_manager: &'a SessionManager,
    ) -> Self {
        Self {
            config,
            git_service,
            session_manager,
        }
    }

    pub fn detect_and_resume_session(&self) -> Result<SessionResumeInfo> {
        let current_dir = env::current_dir()?;

        match self.git_service.validate_session_environment(&current_dir)? {
            SessionEnvironment::Worktree { branch, .. } => {
                println!("Current directory is a worktree for branch: {}", branch);

                // Try to find session name from current directory or branch
                let session_name = self
                    .session_manager
                    .list_sessions()?
                    .into_iter()
                    .find(|s| s.worktree_path == current_dir || s.branch == branch)
                    .map(|s| s.name);

                Ok(SessionResumeInfo {
                    session_name,
                    worktree_path: current_dir,
                    requires_selection: false,
                })
            }
            SessionEnvironment::MainRepository => {
                println!("Current directory is the main repository");
                self.select_session_interactively()
            }
            SessionEnvironment::Invalid => {
                println!("Current directory is not part of a para session");
                self.select_session_interactively()
            }
        }
    }

    pub fn find_specific_session(&self, session_name: &str) -> Result<SessionResumeInfo> {
        if self.session_manager.session_exists(session_name) {
            let mut session_state = self.session_manager.load_state(session_name)?;

            if !session_state.worktree_path.exists() {
                session_state.worktree_path = self.find_matching_worktree(session_name)?;
                self.session_manager.save_state(&session_state)?;
            }

            return Ok(SessionResumeInfo {
                session_name: Some(session_state.name),
                worktree_path: session_state.worktree_path.clone(),
                requires_selection: false,
            });
        }

        // Fallback: maybe the state file was timestamped (e.g. test4_20250611-XYZ)
        if let Some(candidate) = self
            .session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| s.name.starts_with(session_name))
        {
            return self.find_specific_session(&candidate.name);
        }

        // Original branch/path heuristic
        let worktrees = self.git_service.list_worktrees()?;
        let matching_worktree = worktrees
            .iter()
            .find(|wt| {
                wt.branch.contains(session_name)
                    || wt
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.contains(session_name))
                        .unwrap_or(false)
            })
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        // Try to find session name from matching worktree
        let resolved_session_name = self
            .session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| {
                s.worktree_path == matching_worktree.path || s.branch == matching_worktree.branch
            })
            .map(|s| s.name)
            .or_else(|| Some(session_name.to_string()));

        Ok(SessionResumeInfo {
            session_name: resolved_session_name,
            worktree_path: matching_worktree.path.clone(),
            requires_selection: false,
        })
    }

    fn find_matching_worktree(&self, session_name: &str) -> Result<PathBuf> {
        let session_state = self.session_manager.load_state(session_name)?;
        let branch_to_match = session_state.branch.clone();

        if let Some(wt) = self
            .git_service
            .list_worktrees()?
            .into_iter()
            .find(|w| w.branch == branch_to_match)
        {
            return Ok(wt.path.clone());
        }

        if let Some(wt) = self.git_service.list_worktrees()?.into_iter().find(|w| {
            w.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(session_name))
                .unwrap_or(false)
        }) {
            return Ok(wt.path.clone());
        }

        Err(ParaError::session_not_found(format!(
            "Session '{}' exists but worktree path '{}' not found",
            session_name,
            session_state.worktree_path.display()
        )))
    }

    fn select_session_interactively(&self) -> Result<SessionResumeInfo> {
        let sessions = self.session_manager.list_sessions()?;
        let active_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .collect();

        if active_sessions.is_empty() {
            println!("No active sessions found.");
            return Ok(SessionResumeInfo {
                session_name: None,
                worktree_path: PathBuf::new(),
                requires_selection: false,
            });
        }

        println!("Active sessions:");
        for (i, session) in active_sessions.iter().enumerate() {
            println!("  {}: {} ({})", i + 1, session.name, session.branch);
        }

        let selection = Select::new()
            .with_prompt("Select session to resume")
            .items(&active_sessions.iter().map(|s| &s.name).collect::<Vec<_>>())
            .interact();

        if let Ok(index) = selection {
            let session = &active_sessions[index];

            if !session.worktree_path.exists() {
                return Err(ParaError::session_not_found(format!(
                    "Session '{}' exists but worktree path '{}' not found",
                    session.name,
                    session.worktree_path.display()
                )));
            }

            Ok(SessionResumeInfo {
                session_name: Some(session.name.clone()),
                worktree_path: session.worktree_path.clone(),
                requires_selection: true,
            })
        } else {
            Ok(SessionResumeInfo {
                session_name: None,
                worktree_path: PathBuf::new(),
                requires_selection: false,
            })
        }
    }

    pub fn ensure_claude_local_md(&self, info: &SessionResumeInfo) -> Result<()> {
        if let Some(ref session_name) = info.session_name {
            create_claude_local_md(&info.worktree_path, session_name)?;
        } else if !info.worktree_path.as_os_str().is_empty() {
            // Fallback for cases where we have a path but no specific session name
            let fallback_name = info
                .worktree_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            create_claude_local_md(&info.worktree_path, fallback_name)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SessionResumeInfo {
    pub session_name: Option<String>,
    pub worktree_path: PathBuf,
    pub requires_selection: bool,
}