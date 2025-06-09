use crate::cli::parser::RecoverArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{ParaError, Result};
use dialoguer::{Confirm, Select};
use std::path::PathBuf;

pub fn execute(args: RecoverArgs) -> Result<()> {
    validate_recover_args(&args)?;

    let config = Config::load_or_create()?;
    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => recover_specific_session(&config, &git_service, &session_manager, &session_name),
        None => list_recoverable_sessions(&config, &git_service, &session_manager),
    }
}

fn recover_specific_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<()> {
    let archived_branches = git_service.branch_manager().list_archived_branches(config.get_branch_prefix())?;
    
    let matching_archive = archived_branches
        .iter()
        .find(|branch| extract_session_name_from_archive(branch, config.get_branch_prefix()) == Some(session_name))
        .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

    println!("Found archived session: {}", matching_archive);
    
    if Confirm::new()
        .with_prompt(format!("Recover session '{}'?", session_name))
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        recover_session_from_archive(config, git_service, session_manager, matching_archive, session_name)?;
        println!("✅ Session '{}' recovered successfully", session_name);
    }

    Ok(())
}

fn list_recoverable_sessions(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let archived_branches = git_service.branch_manager().list_archived_branches(config.get_branch_prefix())?;
    
    if archived_branches.is_empty() {
        println!("No recoverable sessions found.");
        return Ok(());
    }

    println!("Recoverable sessions:");
    let mut session_options = Vec::new();
    
    for (i, archived_branch) in archived_branches.iter().enumerate() {
        if let Some(session_name) = extract_session_name_from_archive(archived_branch, config.get_branch_prefix()) {
            let timestamp = extract_timestamp_from_archive(archived_branch).unwrap_or_default();
            println!("  {}: {} (archived: {})", i + 1, session_name, timestamp);
            session_options.push((session_name, archived_branch));
        }
    }

    if session_options.is_empty() {
        println!("No valid recoverable sessions found.");
        return Ok(());
    }

    if Confirm::new()
        .with_prompt("Recover a session?")
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        let selection = Select::new()
            .with_prompt("Select session to recover")
            .items(&session_options.iter().map(|(name, _)| name).collect::<Vec<_>>())
            .interact();

        if let Ok(index) = selection {
            let (session_name, archived_branch) = &session_options[index];
            recover_session_from_archive(config, git_service, session_manager, archived_branch, session_name)?;
            println!("✅ Session '{}' recovered successfully", session_name);
        }
    }

    Ok(())
}

fn recover_session_from_archive(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    archived_branch: &str,
    session_name: &str,
) -> Result<()> {
    let branch_manager = git_service.branch_manager();
    let worktree_manager = git_service.worktree_manager();

    let restored_branch = branch_manager.restore_from_archive(archived_branch, config.get_branch_prefix())?;
    println!("Restored branch: {}", restored_branch);

    let subtrees_dir = PathBuf::from(config.get_subtrees_dir());
    let worktree_path = subtrees_dir.join(config.get_branch_prefix()).join(&restored_branch);

    if worktree_path.exists() {
        if !Confirm::new()
            .with_prompt(format!(
                "Worktree directory '{}' already exists. Overwrite?",
                worktree_path.display()
            ))
            .default(false)
            .interact()
            .unwrap_or(false)
        {
            return Err(ParaError::worktree_operation(
                "Recovery cancelled due to existing worktree".to_string(),
            ));
        }

        std::fs::remove_dir_all(&worktree_path)?;
    }

    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    worktree_manager.create_worktree(&restored_branch, &worktree_path)?;
    println!("Created worktree at: {}", worktree_path.display());

    let session_state = SessionState::new(
        session_name.to_string(),
        restored_branch,
        worktree_path,
    );
    session_manager.save_state(&session_state)?;
    println!("Restored session state");

    Ok(())
}

fn extract_session_name_from_archive<'a>(archived_branch: &'a str, prefix: &str) -> Option<&'a str> {
    let archive_prefix = format!("{}/archived/", prefix);
    archived_branch
        .strip_prefix(&archive_prefix)?
        .split('/')
        .next_back()
}

fn extract_timestamp_from_archive(archived_branch: &str) -> Option<&str> {
    archived_branch
        .split('/')
        .nth(2)
}

fn validate_recover_args(args: &RecoverArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}
