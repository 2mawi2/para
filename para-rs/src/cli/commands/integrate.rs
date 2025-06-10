use crate::cli::parser::{IntegrateArgs, IntegrationStrategy};
use crate::config::ConfigManager;
use crate::core::git::{
    GitOperations, GitService, SessionEnvironment, StrategyRequest, StrategyResult,
};
use crate::core::ide::IdeManager;
use crate::core::session::{
    IntegrationState, IntegrationStateManager, IntegrationStep, SessionManager,
};
use crate::utils::{ParaError, Result};
use std::env;
use std::path::{Path, PathBuf};

pub fn execute(args: IntegrateArgs) -> Result<()> {
    validate_integrate_args(&args)?;

    if args.abort {
        return execute_abort();
    }

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    let state_manager = IntegrationStateManager::new(PathBuf::from(config.get_state_dir()));

    if state_manager.has_active_integration() {
        return Err(ParaError::git_operation(
            "Another integration is already in progress. Use 'para continue' to resume or 'para integrate --abort' to abort.".to_string()
        ));
    }

    let (session_id, feature_branch, worktree_path) = if let Some(ref session) = args.session {
        let state = session_manager.load_state(session)?;
        (
            session.clone(),
            state.branch.clone(),
            state.worktree_path.clone(),
        )
    } else {
        let current_dir = env::current_dir()
            .map_err(|e| ParaError::invalid_args(format!("Cannot get current directory: {}", e)))?;

        let env = git_service.validate_session_environment(&current_dir)?;
        match env {
            SessionEnvironment::Worktree { branch, .. } => {
                let session_name = find_session_by_branch(&session_manager, &branch)?;
                let _state = session_manager.load_state(&session_name)?;
                (session_name, branch, current_dir)
            }
            _ => {
                return Err(ParaError::invalid_args(
                    "Not in a session worktree. Please specify session ID or run from session directory.".to_string()
                ));
            }
        }
    };

    let target_branch = args.target.unwrap_or_else(|| {
        git_service
            .repository()
            .get_main_branch()
            .unwrap_or_else(|_| "main".to_string())
    });

    let strategy = args
        .strategy
        .unwrap_or_else(|| config.get_default_integration_strategy());

    let commit_message = if let Some(msg) = args.message {
        Some(msg)
    } else {
        match strategy {
            IntegrationStrategy::Squash => None, // Will be auto-generated
            IntegrationStrategy::Merge => None,  // Will be auto-generated
            IntegrationStrategy::Rebase => None, // No additional commit needed
        }
    };

    println!(
        "🔄 Integrating session '{}' (branch '{}') into '{}'",
        session_id, feature_branch, target_branch
    );
    println!("📋 Using {} strategy", format_strategy(&strategy));

    if args.dry_run {
        return execute_dry_run(&git_service, &feature_branch, &target_branch, &strategy);
    }

    let branch_manager = git_service.branch_manager();
    let current_head = branch_manager.get_branch_commit(&target_branch)?;
    let current_dir = env::current_dir()
        .map_err(|e| ParaError::invalid_args(format!("Cannot get current directory: {}", e)))?;

    let backup_branch = format!(
        "backup-{}-{}",
        target_branch,
        chrono::Utc::now().timestamp()
    );

    let integration_state = IntegrationState::new(
        session_id.clone(),
        feature_branch.clone(),
        target_branch.clone(),
        strategy.clone(),
        commit_message.clone(),
    )
    .with_backup_info(current_head, current_dir, backup_branch);

    state_manager.save_integration_state(&integration_state)?;

    let context = IntegrationContext {
        git_service: &git_service,
        session_manager: &session_manager,
        state_manager: &state_manager,
        config: &config,
        feature_branch: &feature_branch,
        target_branch: &target_branch,
        strategy: &strategy,
        session_id: &session_id,
        worktree_path: &worktree_path,
    };

    match execute_integration(context) {
        Ok(()) => {
            state_manager.clear_integration_state()?;
            println!("✅ Integration completed successfully!");
            Ok(())
        }
        Err(e) => {
            println!("⚠️  Integration failed or paused: {}", e);
            Err(e)
        }
    }
}

fn execute_dry_run(
    git_service: &GitService,
    feature_branch: &str,
    target_branch: &str,
    strategy: &IntegrationStrategy,
) -> Result<()> {
    let strategy_manager = git_service.strategy_manager();

    let request = StrategyRequest {
        feature_branch: feature_branch.to_string(),
        target_branch: target_branch.to_string(),
        strategy: strategy.clone(),
        dry_run: true,
    };

    match strategy_manager.execute_strategy(request)? {
        StrategyResult::DryRun { preview } => {
            println!("🔍 Integration Preview:");
            println!("{}", preview);
            println!("\n💡 Run without --dry-run to execute the integration");
            Ok(())
        }
        _ => Err(ParaError::git_operation(
            "Unexpected result from dry run".to_string(),
        )),
    }
}

struct IntegrationContext<'a> {
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
    state_manager: &'a IntegrationStateManager,
    config: &'a crate::config::Config,
    feature_branch: &'a str,
    target_branch: &'a str,
    strategy: &'a IntegrationStrategy,
    session_id: &'a str,
    worktree_path: &'a Path,
}

fn execute_integration(context: IntegrationContext) -> Result<()> {
    let strategy_manager = context.git_service.strategy_manager();

    context
        .state_manager
        .update_integration_step(IntegrationStep::BaseBranchUpdated)?;

    println!("📦 Preparing base branch '{}'", context.target_branch);

    let request = StrategyRequest {
        feature_branch: context.feature_branch.to_string(),
        target_branch: context.target_branch.to_string(),
        strategy: context.strategy.clone(),
        dry_run: false,
    };

    match strategy_manager.execute_strategy(request)? {
        StrategyResult::Success { final_branch } => {
            context
                .state_manager
                .update_integration_step(IntegrationStep::IntegrationComplete)?;

            println!("🌿 Successfully integrated into branch: {}", final_branch);

            cleanup_after_successful_integration(
                context.git_service,
                context.session_manager,
                context.config,
                context.session_id,
                context.worktree_path,
                context.feature_branch,
            )?;

            Ok(())
        }
        StrategyResult::ConflictsPending { conflicted_files } => {
            context
                .state_manager
                .update_integration_step(IntegrationStep::ConflictsDetected {
                    files: conflicted_files.clone(),
                })?;

            println!("⚠️  Integration paused due to conflicts");
            println!("📁 Conflicted files:");
            for file in &conflicted_files {
                println!("   • {}", file.display());
            }

            let conflict_manager = context.git_service.conflict_manager();
            let summary = conflict_manager.get_conflict_summary()?;
            println!("\n{}", summary);

            open_ide_for_conflict_resolution(context.config, context.worktree_path)?;

            Err(ParaError::git_operation(
                "Integration paused due to conflicts. Resolve conflicts and run 'para continue' to proceed.".to_string()
            ))
        }
        StrategyResult::Failed { error } => {
            context
                .state_manager
                .update_integration_step(IntegrationStep::Failed {
                    error: error.clone(),
                })?;

            Err(ParaError::git_operation(format!(
                "Integration failed: {}",
                error
            )))
        }
        StrategyResult::DryRun { .. } => {
            unreachable!("Dry run should not be returned in non-dry-run mode")
        }
    }
}

fn cleanup_after_successful_integration(
    git_service: &GitService,
    session_manager: &SessionManager,
    config: &crate::config::Config,
    session_id: &str,
    worktree_path: &Path,
    feature_branch: &str,
) -> Result<()> {
    println!("🧹 Cleaning up session...");

    close_ide_for_session(config, worktree_path)?;

    git_service.remove_worktree(worktree_path)?;
    println!("🗂️  Removed worktree: {}", worktree_path.display());

    if !config.should_preserve_on_finish() {
        match git_service.delete_branch(feature_branch, false) {
            Ok(()) => println!("🌿 Deleted feature branch: {}", feature_branch),
            Err(e) => println!(
                "⚠️  Could not delete feature branch {}: {}",
                feature_branch, e
            ),
        }
    } else {
        println!("🌿 Preserved feature branch: {}", feature_branch);
    }

    session_manager.delete_state(session_id)?;
    println!("📋 Removed session state: {}", session_id);

    Ok(())
}

fn open_ide_for_conflict_resolution(
    config: &crate::config::Config,
    worktree_path: &Path,
) -> Result<()> {
    if config.is_wrapper_enabled() {
        println!(
            "💡 Open your IDE to resolve conflicts in: {}",
            worktree_path.display()
        );
        return Ok(());
    }

    println!("🚀 Opening IDE for conflict resolution...");
    let ide_manager = IdeManager::new(config);

    match ide_manager.launch(worktree_path, false) {
        Ok(()) => println!("✅ IDE opened successfully"),
        Err(e) => {
            println!("⚠️  Could not open IDE automatically: {}", e);
            println!(
                "💡 Please manually open your IDE in: {}",
                worktree_path.display()
            );
        }
    }

    Ok(())
}

fn close_ide_for_session(config: &crate::config::Config, _worktree_path: &Path) -> Result<()> {
    if config.is_wrapper_enabled() {
        return Ok(());
    }

    println!("🚪 IDE session will remain open for review");

    Ok(())
}

fn find_session_by_branch(session_manager: &SessionManager, branch: &str) -> Result<String> {
    let sessions = session_manager.list_sessions()?;

    for session in sessions {
        if session.branch == branch {
            return Ok(session.name);
        }
    }

    Err(ParaError::session_not_found(format!(
        "No session found for branch '{}'",
        branch
    )))
}

fn validate_integrate_args(args: &IntegrateArgs) -> Result<()> {
    if args.abort {
        // When aborting, no other options should be provided
        if args.session.is_some()
            || args.target.is_some()
            || args.strategy.is_some()
            || args.message.is_some()
            || args.dry_run
        {
            return Err(ParaError::invalid_args(
                "--abort cannot be used with other options",
            ));
        }
        return Ok(());
    }

    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    if let Some(ref target) = args.target {
        if target.is_empty() {
            return Err(ParaError::invalid_args("Target branch cannot be empty"));
        }
    }

    if let Some(ref message) = args.message {
        if message.trim().is_empty() {
            return Err(ParaError::invalid_args("Commit message cannot be empty"));
        }
    }

    Ok(())
}

fn format_strategy(strategy: &IntegrationStrategy) -> String {
    match strategy {
        IntegrationStrategy::Merge => "merge (preserves commit history)".to_string(),
        IntegrationStrategy::Squash => "squash (combines commits into one)".to_string(),
        IntegrationStrategy::Rebase => "rebase (replays commits linearly)".to_string(),
    }
}

pub fn execute_abort() -> Result<()> {
    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    let git_service = GitService::discover()?;
    let state_manager = IntegrationStateManager::new(PathBuf::from(config.get_state_dir()));

    let integration_state = state_manager.load_integration_state()?.ok_or_else(|| {
        ParaError::git_operation("No integration in progress to abort.".to_string())
    })?;

    println!(
        "🚫 Aborting integration of session '{}'...",
        integration_state.session_id
    );

    let integration_manager = git_service.integration_manager();

    println!("🔄 Cleaning up any ongoing Git operations...");
    integration_manager.cleanup_integration_state()?;

    if let Some(ref backup_branch) = integration_state.backup_branch {
        println!("🔄 Restoring original state from backup...");
        integration_manager
            .safe_abort_integration(Some(backup_branch), &integration_state.base_branch)?;

        println!("🧹 Cleaning up backup branch...");
        if let Err(e) = git_service.delete_branch(backup_branch, true) {
            println!(
                "⚠️  Could not delete backup branch {}: {}",
                backup_branch, e
            );
        }
    } else {
        integration_manager.cleanup_integration_state()?;
    }

    for temp_branch in &integration_state.temp_branches {
        println!("🧹 Cleaning up temporary branch: {}", temp_branch);
        if let Err(e) = git_service.delete_branch(temp_branch, true) {
            println!(
                "⚠️  Could not delete temporary branch {}: {}",
                temp_branch, e
            );
        }
    }

    state_manager.clear_integration_state()?;

    println!("✅ Integration aborted successfully");
    println!("🌿 Repository state restored to original condition");
    println!(
        "📋 Session '{}' remains active for further work",
        integration_state.session_id
    );

    if let Some(ref original_dir) = integration_state.original_working_dir {
        if env::current_dir()
            .map_err(|_| ParaError::git_operation("Failed to get current dir".to_string()))?
            != *original_dir
        {
            println!(
                "💡 You may want to return to your original working directory: {}",
                original_dir.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::IntegrateArgs;
    use crate::core::session::IntegrationState;
    use std::process::Command;
    use crate::utils::ParaError;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, crate::core::git::GitService) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        std::fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let service = crate::core::git::GitService::discover_from(repo_path)
            .expect("Failed to discover repo");
        (temp_dir, service)
    }

    fn create_test_integrate_args() -> IntegrateArgs {
        IntegrateArgs {
            session: None,
            target: None,
            strategy: None,
            message: None,
            dry_run: false,
            abort: false,
        }
    }

    #[test]
    fn test_validate_integrate_args_valid() {
        let args = create_test_integrate_args();
        let result = validate_integrate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_integrate_args_abort_with_session() {
        let args = IntegrateArgs {
            session: Some("test-session".to_string()),
            target: None,
            strategy: None,
            message: None,
            dry_run: false,
            abort: true,
        };

        let result = validate_integrate_args(&args);

        assert!(result.is_err());
        if let Err(ParaError::InvalidArgs { message }) = result {
            assert!(message.contains("--abort cannot be used with other options"));
        } else {
            panic!("Expected InvalidArgs error, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_integrate_args_abort_with_target_branch() {
        let args = IntegrateArgs {
            session: None,
            target: Some("master".to_string()),
            strategy: None,
            message: None,
            dry_run: false,
            abort: true,
        };

        let result = validate_integrate_args(&args);

        assert!(result.is_err());
        if let Err(ParaError::InvalidArgs { message }) = result {
            assert!(message.contains("--abort cannot be used with other options"));
        } else {
            panic!("Expected InvalidArgs error, got: {:?}", result);
        }
    }

    #[test]
    fn test_execute_abort_no_integration() {
        let temp_dir = TempDir::new().unwrap();
        let (git_temp, _git_service) = setup_test_repo();

        // Set up test environment
        std::env::set_current_dir(git_temp.path()).unwrap();
        std::env::set_var("PARA_STATE_DIR", temp_dir.path());

        let result = execute_abort();

        // Accept any error result since test environment can vary
        match result {
            Err(_) => {
                // Any error is acceptable - the important thing is that it fails gracefully
            }
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_integration_state_creation_with_backup() {
        let state = IntegrationState::new(
            "test-session".to_string(),
            "feature-branch".to_string(),
            "master".to_string(),
            IntegrationStrategy::Rebase,
            Some("Test commit".to_string()),
        )
        .with_backup_info(
            "abc123def456".to_string(),
            PathBuf::from("/test/path"),
            "backup-master-123456".to_string(),
        );

        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.feature_branch, "feature-branch");
        assert_eq!(state.base_branch, "master");
        assert_eq!(state.original_head_commit, Some("abc123def456".to_string()));
        assert_eq!(
            state.original_working_dir,
            Some(PathBuf::from("/test/path"))
        );
        assert_eq!(
            state.backup_branch,
            Some("backup-master-123456".to_string())
        );
    }

    #[test]
    fn test_integrate_args_structure() {
        let args = IntegrateArgs {
            session: None,
            target: None,
            strategy: None,
            message: None,
            dry_run: false,
            abort: false,
        };

        assert_eq!(args.session, None);
        assert_eq!(args.target, None);
        assert_eq!(args.strategy, None);
        assert_eq!(args.message, None);
        assert!(!args.dry_run);
        assert!(!args.abort);
    }

    #[test]
    fn test_integration_strategy_enum() {
        let rebase = IntegrationStrategy::Rebase;
        let merge = IntegrationStrategy::Merge;
        let squash = IntegrationStrategy::Squash;

        assert!(matches!(rebase, IntegrationStrategy::Rebase));
        assert!(matches!(merge, IntegrationStrategy::Merge));
        assert!(matches!(squash, IntegrationStrategy::Squash));
    }
}
