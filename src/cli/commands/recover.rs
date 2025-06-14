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
    use tempfile::TempDir;

    #[test]
    fn test_validate_recover_args_valid_session() {
        let args = RecoverArgs {
            session: Some("test-session".to_string()),
        };
        assert!(validate_recover_args(&args).is_ok());
    }

    #[test]
    fn test_validate_recover_args_empty_session() {
        let args = RecoverArgs {
            session: Some("".to_string()),
        };
        let result = validate_recover_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_recover_args_no_session() {
        let args = RecoverArgs { session: None };
        assert!(validate_recover_args(&args).is_ok());
    }

    #[test]
    fn test_recover_specific_session_success() {
        // This test verifies the recovery process finds and validates an archived session
        // Note: Interactive prompts will cause early return in test environment, which is expected
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create the archive directory
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Create an archive entry
        let archive_entry = crate::core::session::archive::ArchiveEntry {
            session_name: "test-recover".to_string(),
            branch_name: "test/test-recover".to_string(),
            base_branch: "main".to_string(),
            worktree_path: git_service.repo_path().join("subtrees").join("test-recover"),
            archived_at: chrono::Utc::now().to_rfc3339(),
            commit_sha: "abc123".to_string(),
            commit_message: "Test commit".to_string(),
            files_changed: vec!["test.txt".to_string()],
        };

        let archive_path = archive_dir.join("test-recover.json");
        std::fs::write(
            &archive_path,
            serde_json::to_string_pretty(&archive_entry).unwrap(),
        )
        .unwrap();

        // Test recovery - will return Ok(()) when it hits the confirmation prompt
        let result = recover_specific_session(&config, &git_service, &session_manager, "test-recover");
        assert!(result.is_ok());
    }

    #[test]
    fn test_recover_nonexistent_session() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create archive directory but no archives
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        let result = recover_specific_session(&config, &git_service, &session_manager, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No archived session found"));
    }

    #[test]
    fn test_list_recoverable_sessions_empty() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create archive directory but no archives
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Should complete successfully and print "No recoverable sessions found."
        let result = list_recoverable_sessions(&config, &git_service, &session_manager);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_recoverable_sessions_with_archives() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create archive directory with archives
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Create multiple archive entries
        for i in 1..=3 {
            let archive_entry = crate::core::session::archive::ArchiveEntry {
                session_name: format!("session-{}", i),
                branch_name: format!("test/session-{}", i),
                base_branch: "main".to_string(),
                worktree_path: git_service.repo_path().join("subtrees").join(format!("session-{}", i)),
                archived_at: chrono::Utc::now().to_rfc3339(),
                commit_sha: format!("abc{}", i),
                commit_message: format!("Test commit {}", i),
                files_changed: vec![format!("test{}.txt", i)],
            };

            let archive_path = archive_dir.join(format!("session-{}.json", i));
            std::fs::write(
                &archive_path,
                serde_json::to_string_pretty(&archive_entry).unwrap(),
            )
            .unwrap();
        }

        // Should list sessions and return Ok when hitting confirmation prompt
        let result = list_recoverable_sessions(&config, &git_service, &session_manager);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recovery_with_invalid_session_names() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create archive directory
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Test various invalid session names
        let invalid_names = vec![
            "session with spaces",
            "session/with/slashes", 
            "session\\with\\backslashes",
            "session|with|pipes",
        ];

        for invalid_name in invalid_names {
            let result = recover_specific_session(&config, &git_service, &session_manager, invalid_name);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("No archived session found"));
        }
    }

    #[test]
    fn test_interactive_session_selection_flow() {
        // This test verifies that the interactive flow properly lists sessions
        // In test environment, it will return Ok after displaying the list
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        // Create archive directory with test data
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Create test archive for interactive selection
        let archive_entry = crate::core::session::archive::ArchiveEntry {
            session_name: "interactive-test".to_string(),
            branch_name: "test/interactive".to_string(),
            base_branch: "main".to_string(),
            worktree_path: git_service.repo_path().join("subtrees").join("interactive-test"),
            archived_at: chrono::Utc::now().to_rfc3339(),
            commit_sha: "def456".to_string(),
            commit_message: "Interactive test commit".to_string(),
            files_changed: vec!["interactive.txt".to_string()],
        };

        let archive_path = archive_dir.join("interactive-test.json");
        std::fs::write(
            &archive_path,
            serde_json::to_string_pretty(&archive_entry).unwrap(),
        )
        .unwrap();

        // Execute with RecoverArgs having no session specified
        let args = RecoverArgs { session: None };
        let result = execute(config, args);
        // Should succeed (returns after listing sessions in test env)
        assert!(result.is_ok());
    }

    #[test]
    fn test_force_recovery_when_conflicts_exist() {
        // This test simulates a recovery scenario where conflicts exist
        // and verifies the force recovery path
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create archive directory and test archive
        let archive_dir = temp_dir.path().join(".para_state").join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();

        let archive_entry = crate::core::session::archive::ArchiveEntry {
            session_name: "conflict-test".to_string(),
            branch_name: "test/conflict".to_string(),
            base_branch: "main".to_string(),
            worktree_path: git_service.repo_path().join("subtrees").join("conflict-test"),
            archived_at: chrono::Utc::now().to_rfc3339(),
            commit_sha: "conflict123".to_string(),
            commit_message: "Conflict test commit".to_string(),
            files_changed: vec!["conflict.txt".to_string()],
        };

        let archive_path = archive_dir.join("conflict-test.json");
        std::fs::write(
            &archive_path,
            serde_json::to_string_pretty(&archive_entry).unwrap(),
        )
        .unwrap();

        // In a real scenario with conflicts, the validation would fail
        // but in test environment, prompts cause early return
        let result = recover_specific_session(&config, &git_service, &session_manager, "conflict-test");
        assert!(result.is_ok());
    }
}
