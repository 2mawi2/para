use crate::cli::parser::RecoverArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::archive::ArchiveManager;
use crate::core::session::recovery::{RecoveryOptions, SessionRecovery};
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};
use dialoguer::{Confirm, Select};
use std::env;

/// Check if we're running in non-interactive mode (e.g., from MCP server)
fn is_non_interactive() -> bool {
    env::var("PARA_NON_INTERACTIVE").is_ok()
        || env::var("CI").is_ok()
        || !atty::is(atty::Stream::Stdin)
}

pub fn execute(config: Config, args: RecoverArgs) -> Result<()> {
    validate_recover_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => {
            recover_specific_session(&config, &git_service, &session_manager, &session_name)
        }
        None => list_recoverable_sessions(&config, &git_service, &session_manager),
    }
}

fn recover_specific_session(
    config: &crate::config::Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<()> {
    let session_recovery = SessionRecovery::new(config, git_service, session_manager);

    // Try to recover the session, handling both active and archived sessions
    let recovery_options = if is_non_interactive() {
        RecoveryOptions {
            force_overwrite: true,
            preserve_original_name: true,
        }
    } else {
        // Check for conflicts first to provide user feedback
        if !session_recovery.is_active_session(session_name) {
            match session_recovery.validate_recovery(session_name) {
                Ok(validation) if !validation.can_recover => {
                    println!(
                        "❌ Cannot recover session '{}' due to conflicts:",
                        session_name
                    );
                    for conflict in &validation.conflicts {
                        println!("  • {}", conflict);
                    }
                    if !validation.warnings.is_empty() {
                        println!("Warnings:");
                        for warning in &validation.warnings {
                            println!("  ⚠ {}", warning);
                        }
                    }

                    if !Confirm::new()
                        .with_prompt("Force recovery anyway?")
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                    {
                        return Ok(());
                    }
                    
                    RecoveryOptions {
                        force_overwrite: true,
                        preserve_original_name: true,
                    }
                }
                Ok(validation) => {
                    if !validation.warnings.is_empty() {
                        println!("⚠ Warnings for session '{}':", session_name);
                        for warning in &validation.warnings {
                            println!("  • {}", warning);
                        }
                    }

                    if !Confirm::new()
                        .with_prompt(format!("Recover session '{}'?", session_name))
                        .default(true)
                        .interact()
                        .unwrap_or(false)
                    {
                        return Ok(());
                    }

                    RecoveryOptions {
                        force_overwrite: false,
                        preserve_original_name: true,
                    }
                }
                Err(_) => {
                    return Err(ParaError::session_not_found(format!(
                        "No session found for '{}'",
                        session_name
                    )));
                }
            }
        } else {
            RecoveryOptions {
                force_overwrite: false,
                preserve_original_name: true,
            }
        }
    };

    let result = session_recovery.recover_session_unified(session_name, recovery_options)?;
    display_recovery_result(&result);
    Ok(())
}

fn display_recovery_result(result: &crate::core::session::recovery::RecoveryResult) {
    println!(
        "✅ Session '{}' recovered successfully",
        result.session_name
    );
    println!("  Branch: {}", result.branch_name);
    println!("  Worktree: {}", result.worktree_path.display());
    println!(
        "  💡 To open in your IDE, run: para resume {}",
        result.session_name
    );
}

fn list_recoverable_sessions(
    config: &crate::config::Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let archive_manager = ArchiveManager::new(config, git_service);
    let session_recovery = SessionRecovery::new(config, git_service, session_manager);

    let archives = archive_manager.list_archives()?;

    if archives.is_empty() {
        println!("No recoverable sessions found.");
        return Ok(());
    }

    println!("Recoverable sessions:");
    for (i, archive) in archives.iter().enumerate() {
        println!(
            "  {}: {} (archived: {})",
            i + 1,
            archive.session_name,
            archive.archived_at
        );
    }

    if is_non_interactive() {
        return Err(ParaError::invalid_args(
            "Cannot interactively select session to recover in non-interactive mode. \
             Please specify a session name: para recover <session-name>",
        ));
    }

    if !Confirm::new()
        .with_prompt("Recover a session?")
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        return Ok(());
    }

    let session_names: Vec<&str> = archives.iter().map(|a| a.session_name.as_str()).collect();
    let selection = Select::new()
        .with_prompt("Select session to recover")
        .items(&session_names)
        .interact();

    if let Ok(index) = selection {
        let selected_session = &archives[index].session_name;

        let recovery_options = RecoveryOptions {
            force_overwrite: false,
            preserve_original_name: true,
        };

        match session_recovery.recover_session(selected_session, recovery_options) {
            Ok(result) => {
                display_recovery_result(&result);
            }
            Err(e) => {
                eprintln!("❌ Failed to recover session: {}", e);

                if Confirm::new()
                    .with_prompt("Try force recovery?")
                    .default(false)
                    .interact()
                    .unwrap_or(false)
                {
                    let force_options = RecoveryOptions {
                        force_overwrite: true,
                        preserve_original_name: true,
                    };

                    let result = session_recovery.recover_session(selected_session, force_options)?;
                    println!("✅ Session '{}' force recovered successfully", result.session_name);
                    display_recovery_result(&result);
                }
            }
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::{SessionManager, SessionState};
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    fn create_test_config_with_dir(temp_dir: &TempDir) -> Config {
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = temp_dir.path().join("subtrees").to_string_lossy().to_string();
        config
    }

    #[test]
    fn test_recover_active_session_healthy() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();
        
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Create a healthy session state
        let session_name = "test-active-session";
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session_state = SessionState::new(
            session_name.to_string(),
            "test-branch".to_string(),
            worktree_path.clone(),
        );

        session_manager.save_state(&session_state).unwrap();

        // Create the branch
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        git_service
            .branch_manager()
            .create_branch("test-branch", &initial_branch)
            .unwrap();

        // Test recovery using the core service
        let session_recovery = SessionRecovery::new(&config, &git_service, &session_manager);
        let result = session_recovery.recover_active_session(session_name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recover_active_session_missing_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();
        
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Create session state with missing worktree
        let session_name = "test-missing-worktree";
        let branch_name = "test-missing-worktree-branch";
        let worktree_path = temp_dir.path().join("missing-worktree");
        // Don't create the worktree directory

        let session_state = SessionState::new(
            session_name.to_string(),
            branch_name.to_string(),
            worktree_path.clone(),
        );

        session_manager.save_state(&session_state).unwrap();

        // Create the branch
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        git_service
            .branch_manager()
            .create_branch(branch_name, &initial_branch)
            .unwrap();

        // Switch back to main branch so the test branch isn't checked out
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        // Test recovery using the core service - should recreate worktree
        let session_recovery = SessionRecovery::new(&config, &git_service, &session_manager);
        let result = session_recovery.recover_active_session(session_name);
        assert!(result.is_ok(), "Recovery should succeed: {:?}", result);
        assert!(
            worktree_path.exists(),
            "Worktree should have been recreated"
        );
    }

    #[test]
    fn test_recover_active_session_missing_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();
        
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Create session state with missing branch
        let session_name = "test-missing-branch";
        let worktree_path = temp_dir.path().join("worktree");

        let session_state = SessionState::new(
            session_name.to_string(),
            "missing-branch".to_string(),
            worktree_path.clone(),
        );

        session_manager.save_state(&session_state).unwrap();
        // Don't create the branch

        // Test recovery using the core service - should fail with missing branch error
        let session_recovery = SessionRecovery::new(&config, &git_service, &session_manager);
        let result = session_recovery.recover_active_session(session_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("missing branch"));
        assert!(error_msg.contains("cannot recover"));
    }
}
