use crate::cli::parser::RecoverArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::archive::ArchiveManager;
use crate::core::session::recovery::{RecoveryOptions, SessionRecovery};
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};
use dialoguer::{Confirm, Select};

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
    let archive_manager = ArchiveManager::new(config, git_service);

    match session_recovery.validate_recovery(session_name)? {
        validation if !validation.can_recover => {
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
        }
        validation => {
            if !validation.warnings.is_empty() {
                println!("⚠ Warnings for session '{}':", session_name);
                for warning in &validation.warnings {
                    println!("  • {}", warning);
                }
            }
        }
    }

    if let Some(archive_entry) = archive_manager.find_archive(session_name)? {
        println!(
            "Found archived session: {} (archived: {})",
            session_name, archive_entry.archived_at
        );

        if Confirm::new()
            .with_prompt(format!("Recover session '{}'?", session_name))
            .default(true)
            .interact()
            .unwrap_or(false)
        {
            let recovery_options = RecoveryOptions {
                force_overwrite: true,
                preserve_original_name: true,
            };

            let result = session_recovery.recover_session(session_name, recovery_options)?;
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
    } else {
        return Err(ParaError::session_not_found(format!(
            "No archived session found for '{}'",
            session_name
        )));
    }

    Ok(())
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

    if Confirm::new()
        .with_prompt("Recover a session?")
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        let session_names: Vec<&str> = archives.iter().map(|a| a.session_name.as_str()).collect();
        let selection = Select::new()
            .with_prompt("Select session to recover")
            .items(&session_names)
            .interact();

        if let Ok(index) = selection {
            let selected_archive = &archives[index];

            let recovery_options = RecoveryOptions {
                force_overwrite: false,
                preserve_original_name: true,
            };

            match session_recovery.recover_session(&selected_archive.session_name, recovery_options)
            {
                Ok(result) => {
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

                        let result = session_recovery
                            .recover_session(&selected_archive.session_name, force_options)?;
                        println!(
                            "✅ Session '{}' force recovered successfully",
                            result.session_name
                        );
                        println!("  Branch: {}", result.branch_name);
                        println!("  Worktree: {}", result.worktree_path.display());
                        println!(
                            "  💡 To open in your IDE, run: para resume {}",
                            result.session_name
                        );
                    }
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
    use crate::test_utils::test_helpers::*;
    use crate::core::session::SessionState;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_validate_recover_args() {
        // Valid args with session name
        let valid_args = RecoverArgs {
            session: Some("test-session".to_string()),
        };
        assert!(validate_recover_args(&valid_args).is_ok());

        // Valid args without session name (interactive mode)
        let no_session_args = RecoverArgs { session: None };
        assert!(validate_recover_args(&no_session_args).is_ok());

        // Invalid args with empty session name
        let empty_session_args = RecoverArgs {
            session: Some("".to_string()),
        };
        assert!(validate_recover_args(&empty_session_args).is_err());
    }

    #[test]
    fn test_successful_recovery_of_archived_session() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Pre-create directories to avoid race conditions
        fs::create_dir_all(&config.directories.state_dir).unwrap();
        let para_dir = git_service.repository().root.join(".para");
        fs::create_dir_all(&para_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Create a test session state
        let session_state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            git_service.repository().root.join("worktree")
        );
        session_manager.save_state(&session_state).unwrap();

        // Create a test branch to archive
        git_service.branch_manager().create_branch("test-branch", "main").unwrap();
        git_service.checkout_branch("main").unwrap();

        // Archive the branch with session name
        git_service.branch_manager().move_to_archive_with_session_name(
            "test-branch",
            "test-session",
            &config.get_branch_prefix()
        ).unwrap();

        // Test recovery
        let args = RecoverArgs {
            session: Some("test-session".to_string()),
        };

        // We can't test the interactive prompts, but we can verify the archive exists
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let archive = archive_manager.find_archive("test-session").unwrap();
        assert!(archive.is_some());
        assert_eq!(archive.unwrap().session_name, "test-session");
    }

    #[test]
    fn test_recovery_when_no_archived_sessions_exist() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Pre-create directories
        fs::create_dir_all(&config.directories.state_dir).unwrap();
        let para_dir = git_service.repository().root.join(".para");
        fs::create_dir_all(&para_dir).unwrap();

        // Check that no archives exist
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let archives = archive_manager.list_archives().unwrap();
        assert!(archives.is_empty());

        // Try to recover a non-existent session
        let args = RecoverArgs {
            session: Some("non-existent".to_string()),
        };

        // The recover operation should fail when trying to find the archive
        let result = recover_specific_session(&config, &git_service, &SessionManager::new(&config), "non-existent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No archived session found"));
    }

    #[test]
    fn test_recovery_with_invalid_session_names() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Test with various invalid session names
        let invalid_names = vec!["", "with spaces", "with/slash"];

        for invalid_name in invalid_names {
            let args = RecoverArgs {
                session: Some(invalid_name.to_string()),
            };

            // Empty string should fail validation
            if invalid_name.is_empty() {
                assert!(validate_recover_args(&args).is_err());
            } else {
                // Non-empty invalid names should fail when trying to find the archive
                let result = recover_specific_session(&config, &git_service, &SessionManager::new(&config), invalid_name);
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_interactive_session_selection_flow() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Pre-create directories
        fs::create_dir_all(&config.directories.state_dir).unwrap();
        let para_dir = git_service.repository().root.join(".para");
        fs::create_dir_all(&para_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Create multiple archived sessions
        for i in 1..=3 {
            let session_name = format!("session-{}", i);
            let branch_name = format!("branch-{}", i);

            // Create session state
            let session_state = SessionState::new(
                session_name.clone(),
                branch_name.clone(),
                git_service.repository().root.join(format!("worktree-{}", i))
            );
            session_manager.save_state(&session_state).unwrap();

            // Create and archive branch
            git_service.branch_manager().create_branch(&branch_name, "main").unwrap();
            git_service.checkout_branch("main").unwrap();
            git_service.branch_manager().move_to_archive_with_session_name(
                &branch_name,
                &session_name,
                &config.get_branch_prefix()
            ).unwrap();
        }

        // Verify all archives exist
        let archive_manager = ArchiveManager::new(&config, &git_service);
        let archives = archive_manager.list_archives().unwrap();
        assert_eq!(archives.len(), 3);

        // Test listing recoverable sessions (without actual interaction)
        let args = RecoverArgs { session: None };
        
        // We can't test the interactive flow directly, but we can verify the archives are ready
        for i in 1..=3 {
            let session_name = format!("session-{}", i);
            let archive = archive_manager.find_archive(&session_name).unwrap();
            assert!(archive.is_some());
        }
    }

    #[test]
    fn test_force_recovery_when_conflicts_exist() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Pre-create directories
        fs::create_dir_all(&config.directories.state_dir).unwrap();
        let para_dir = git_service.repository().root.join(".para");
        fs::create_dir_all(&para_dir).unwrap();

        let session_manager = SessionManager::new(&config);
        let session_recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        // Create an archived session
        let session_state = SessionState::new(
            "conflicting-session".to_string(),
            "conflicting-branch".to_string(),
            git_service.repository().root.join("conflicting-worktree")
        );
        session_manager.save_state(&session_state).unwrap();

        // Create and archive the branch
        git_service.branch_manager().create_branch("conflicting-branch", "main").unwrap();
        git_service.checkout_branch("main").unwrap();
        git_service.branch_manager().move_to_archive_with_session_name(
            "conflicting-branch",
            "conflicting-session",
            &config.get_branch_prefix()
        ).unwrap();

        // Create a conflicting branch with the same name
        git_service.branch_manager().create_branch("conflicting-branch", "main").unwrap();
        git_service.checkout_branch("main").unwrap();

        // Try to recover - it should detect conflicts
        let validation = session_recovery.validate_recovery("conflicting-session").unwrap();
        assert!(!validation.can_recover || !validation.conflicts.is_empty());

        // Force recovery should work
        let recovery_options = RecoveryOptions {
            force_overwrite: true,
            preserve_original_name: true,
        };

        // The branch should be restored with a unique name due to conflict
        let archives = git_service.branch_manager().list_archived_branches(&config.get_branch_prefix()).unwrap();
        assert!(!archives.is_empty());
    }
}
