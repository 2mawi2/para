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
