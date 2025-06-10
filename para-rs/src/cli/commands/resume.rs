use crate::cli::parser::ResumeArgs;
use crate::config::{Config, ConfigManager};
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::Path;

pub fn execute(args: ResumeArgs) -> Result<()> {
    validate_resume_args(&args)?;

    let config = ConfigManager::load_or_create()?;
    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => resume_specific_session(&config, &git_service, &session_name),
        None => detect_and_resume_session(&config, &git_service, &session_manager),
    }
}

fn resume_specific_session(
    config: &Config,
    git_service: &GitService,
    session_name: &str,
) -> Result<()> {
    let session_manager = SessionManager::new(&config);

    if session_manager.session_exists(session_name) {
        let session_state = session_manager.load_state(session_name)?;

        if !session_state.worktree_path.exists() {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session_name,
                session_state.worktree_path.display()
            )));
        }

        launch_ide_for_session(config, &session_state.worktree_path)?;
        println!("✅ Resumed session '{}'", session_name);
    } else {
        let worktrees = git_service.list_worktrees()?;
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

        launch_ide_for_session(config, &matching_worktree.path)?;
        println!(
            "✅ Resumed session at '{}'",
            matching_worktree.path.display()
        );
    }

    Ok(())
}

fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            println!("Current directory is a worktree for branch: {}", branch);
            launch_ide_for_session(config, &current_dir)?;
            println!("✅ Resumed current session");
            Ok(())
        }
        SessionEnvironment::MainRepository => {
            println!("Current directory is the main repository");
            list_and_select_session(config, git_service, session_manager)
        }
        SessionEnvironment::Invalid => {
            println!("Current directory is not part of a para session");
            list_and_select_session(config, git_service, session_manager)
        }
    }
}

fn list_and_select_session(
    config: &Config,
    _git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let sessions = session_manager.list_sessions()?;
    let active_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| matches!(s.status, SessionStatus::Active))
        .collect();

    if active_sessions.is_empty() {
        println!("No active sessions found.");
        return Ok(());
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

        launch_ide_for_session(config, &session.worktree_path)?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

fn launch_ide_for_session(config: &Config, path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);
    ide_manager.launch(path, false)
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
