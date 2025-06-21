use crate::cli::commands::common::create_claude_local_md;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;

use super::execution::launch_ide_for_session;

/// Resumes a specific session by name
pub fn resume_specific_session(
    config: &Config,
    git_service: &GitService,
    session_name: &str,
) -> Result<()> {
    let session_manager = SessionManager::new(config);

    if session_manager.session_exists(session_name) {
        let mut session_state = session_manager.load_state(session_name)?;

        if !session_state.worktree_path.exists() {
            let branch_to_match = session_state.branch.clone();
            if let Some(wt) = git_service
                .list_worktrees()?
                .into_iter()
                .find(|w| w.branch == branch_to_match)
            {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
                w.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(session_name))
                    .unwrap_or(false)
            }) {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else {
                return Err(ParaError::session_not_found(format!(
                    "Session '{}' exists but worktree path '{}' not found",
                    session_name,
                    session_state.worktree_path.display()
                )));
            }
        }

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

        launch_ide_for_session(config, &session_state.worktree_path)?;
        println!("✅ Resumed session '{}'", session_name);
    } else {
        // Fallback: maybe the state file was timestamped (e.g. test4_20250611-XYZ)
        if let Some(candidate) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| s.name.starts_with(session_name))
        {
            // recurse with the full name
            return resume_specific_session(config, git_service, &candidate.name);
        }
        // original branch/path heuristic
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

        // Try to find session name from matching worktree
        if let Some(session_name) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| {
                s.worktree_path == matching_worktree.path || s.branch == matching_worktree.branch
            })
            .map(|s| s.name)
        {
            create_claude_local_md(&matching_worktree.path, &session_name)?;
        } else {
            // Fallback: use session name from search
            create_claude_local_md(&matching_worktree.path, session_name)?;
        }

        launch_ide_for_session(config, &matching_worktree.path)?;
        println!(
            "✅ Resumed session at '{}'",
            matching_worktree.path.display()
        );
    }

    Ok(())
}

/// Detects current environment and resumes appropriate session
pub fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            println!("Current directory is a worktree for branch: {}", branch);

            // Try to find session name from current directory or branch
            if let Some(session_name) = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch)
                .map(|s| s.name)
            {
                create_claude_local_md(&current_dir, &session_name)?;
            }

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

/// Lists active sessions and allows user to select one
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

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session.worktree_path, &session.name)?;

        launch_ide_for_session(config, &session.worktree_path)?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

