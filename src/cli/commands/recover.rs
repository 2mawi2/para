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
    let archive_manager = ArchiveManager::new(config, git_service);

    // First check if this is an active session that needs recovery (dirty/missing)
    if session_manager.session_exists(session_name) {
        return recover_active_session(config, git_service, session_manager, session_name);
    }

    // If not an active session, try archived session recovery
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

            if is_non_interactive() {
                return Err(ParaError::invalid_args(format!(
                    "Cannot recover session '{}' due to conflicts in non-interactive mode. \
                     Conflicts: {}. Please resolve conflicts first or run interactively.",
                    session_name,
                    validation.conflicts.join(", ")
                )));
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

        if is_non_interactive() {
            // In non-interactive mode, proceed with recovery automatically
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
        } else if Confirm::new()
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

    if is_non_interactive() {
        return Err(ParaError::invalid_args(
            "Cannot interactively select session to recover in non-interactive mode. \
             Please specify a session name: para recover <session-name>",
        ));
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

                    if !is_non_interactive()
                        && Confirm::new()
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

fn recover_active_session(
    _config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<()> {
    let session_state = session_manager.load_state(session_name)?;

    // Check if worktree exists
    let worktree_exists = session_state.worktree_path.exists();
    let branch_exists = git_service
        .branch_manager()
        .branch_exists(&session_state.branch)?;

    if worktree_exists && branch_exists {
        println!(
            "✅ Session '{}' is already active and healthy",
            session_name
        );
        println!("  Branch: {}", session_state.branch);
        println!("  Worktree: {}", session_state.worktree_path.display());
        println!(
            "  💡 To open in your IDE, run: para resume {}",
            session_name
        );
        return Ok(());
    }

    // Need to recover missing worktree
    if !worktree_exists {
        println!(
            "🔧 Recovering missing worktree for session '{}'",
            session_name
        );

        // Ensure parent directory exists
        if let Some(parent) = session_state.worktree_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create worktree
        if branch_exists {
            git_service
                .worktree_manager()
                .create_worktree(&session_state.branch, &session_state.worktree_path)?;
        } else {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' has missing branch '{}' - cannot recover",
                session_name, session_state.branch
            )));
        }

        println!("✅ Session '{}' recovered successfully", session_name);
        println!("  Branch: {}", session_state.branch);
        println!("  Worktree: {}", session_state.worktree_path.display());
        println!(
            "  💡 To open in your IDE, run: para resume {}",
            session_name
        );
    } else {
        println!(
            "✅ Session '{}' worktree exists but may have issues",
            session_name
        );
        println!("  Branch: {}", session_state.branch);
        println!("  Worktree: {}", session_state.worktree_path.display());
        println!(
            "  💡 To open in your IDE, run: para resume {}",
            session_name
        );
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
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use crate::core::session::{SessionManager, SessionState};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir.join("subtrees").to_string_lossy().to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, GitService, Config, SessionManager) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo dir");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let git_service = GitService::discover_from(&repo_path).expect("Failed to discover repo");
        let config = create_test_config(temp_dir.path());
        let session_manager = SessionManager::new(&config);

        (temp_dir, git_service, config, session_manager)
    }

    #[test]
    fn test_recover_active_session_healthy() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();

        // Create a healthy session state
        let session_name = "test-active-session";
        let worktree_path = _temp_dir.path().join("worktree");
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

        // Test recovery - should recognize session as healthy
        let result = recover_active_session(&config, &git_service, &session_manager, session_name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recover_active_session_missing_worktree() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();

        // Create session state with missing worktree
        let session_name = "test-missing-worktree";
        let branch_name = "test-missing-worktree-branch";
        let worktree_path = _temp_dir.path().join("missing-worktree");
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

        // Test recovery - should recreate worktree
        let result = recover_active_session(&config, &git_service, &session_manager, session_name);
        assert!(result.is_ok(), "Recovery should succeed: {:?}", result);
        assert!(
            worktree_path.exists(),
            "Worktree should have been recreated"
        );
    }

    #[test]
    fn test_recover_active_session_missing_branch() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();

        // Create session state with missing branch
        let session_name = "test-missing-branch";
        let worktree_path = _temp_dir.path().join("worktree");

        let session_state = SessionState::new(
            session_name.to_string(),
            "missing-branch".to_string(),
            worktree_path.clone(),
        );

        session_manager.save_state(&session_state).unwrap();
        // Don't create the branch

        // Test recovery - should fail with missing branch error
        let result = recover_active_session(&config, &git_service, &session_manager, session_name);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("missing branch"));
        assert!(error_msg.contains("cannot recover"));
    }
}
