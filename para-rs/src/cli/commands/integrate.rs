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
use std::path::PathBuf;

pub fn execute(args: IntegrateArgs) -> Result<()> {
    validate_integrate_args(&args)?;

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    let state_manager = IntegrationStateManager::new(PathBuf::from(config.get_state_dir()));

    if state_manager.has_active_integration() {
        return Err(ParaError::git_operation(
            "Another integration is already in progress. Use 'para continue' to resume or 'para cancel' to abort.".to_string()
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
                let session_name = extract_session_from_branch(&branch)?;
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

    let integration_state = IntegrationState::new(
        session_id.clone(),
        feature_branch.clone(),
        target_branch.clone(),
        strategy.clone(),
        commit_message.clone(),
    );

    state_manager.save_integration_state(&integration_state)?;

    match execute_integration(
        &git_service,
        &session_manager,
        &state_manager,
        &config,
        &feature_branch,
        &target_branch,
        &strategy,
        &session_id,
        &worktree_path,
        commit_message.as_deref(),
    ) {
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

fn execute_integration(
    git_service: &GitService,
    session_manager: &SessionManager,
    state_manager: &IntegrationStateManager,
    config: &crate::config::Config,
    feature_branch: &str,
    target_branch: &str,
    strategy: &IntegrationStrategy,
    session_id: &str,
    worktree_path: &PathBuf,
    _commit_message: Option<&str>,
) -> Result<()> {
    let strategy_manager = git_service.strategy_manager();

    state_manager.update_integration_step(IntegrationStep::BaseBranchUpdated)?;

    println!("📦 Preparing base branch '{}'", target_branch);

    let request = StrategyRequest {
        feature_branch: feature_branch.to_string(),
        target_branch: target_branch.to_string(),
        strategy: strategy.clone(),
        dry_run: false,
    };

    match strategy_manager.execute_strategy(request)? {
        StrategyResult::Success { final_branch } => {
            state_manager.update_integration_step(IntegrationStep::IntegrationComplete)?;

            println!("🌿 Successfully integrated into branch: {}", final_branch);

            cleanup_after_successful_integration(
                git_service,
                session_manager,
                config,
                session_id,
                worktree_path,
                feature_branch,
            )?;

            Ok(())
        }
        StrategyResult::ConflictsPending { conflicted_files } => {
            state_manager.update_integration_step(IntegrationStep::ConflictsDetected {
                files: conflicted_files.clone(),
            })?;

            println!("⚠️  Integration paused due to conflicts");
            println!("📁 Conflicted files:");
            for file in &conflicted_files {
                println!("   • {}", file.display());
            }

            let conflict_manager = git_service.conflict_manager();
            let summary = conflict_manager.get_conflict_summary()?;
            println!("\n{}", summary);

            open_ide_for_conflict_resolution(config, worktree_path)?;

            Err(ParaError::git_operation(
                "Integration paused due to conflicts. Resolve conflicts and run 'para continue' to proceed.".to_string()
            ))
        }
        StrategyResult::Failed { error } => {
            state_manager.update_integration_step(IntegrationStep::Failed {
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
    worktree_path: &PathBuf,
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
    worktree_path: &PathBuf,
) -> Result<()> {
    if config.is_wrapper_enabled() {
        println!(
            "💡 Open your IDE to resolve conflicts in: {}",
            worktree_path.display()
        );
        return Ok(());
    }

    println!("🚀 Opening IDE for conflict resolution...");
    let ide_manager = IdeManager::new(&config);

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

fn close_ide_for_session(config: &crate::config::Config, worktree_path: &PathBuf) -> Result<()> {
    if config.is_wrapper_enabled() {
        return Ok(());
    }

    println!("🚪 IDE session will remain open for review");

    Ok(())
}

fn extract_session_from_branch(branch: &str) -> Result<String> {
    if let Some(stripped) = branch.strip_prefix("pc/") {
        if let Some(pos) = stripped.rfind('-') {
            Ok(stripped[pos + 1..].to_string())
        } else {
            Ok(stripped.to_string())
        }
    } else {
        Err(ParaError::invalid_args(format!(
            "Branch '{}' is not a valid session branch",
            branch
        )))
    }
}

fn validate_integrate_args(args: &IntegrateArgs) -> Result<()> {
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

pub fn execute_continue() -> Result<()> {
    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    let state_manager = IntegrationStateManager::new(PathBuf::from(config.get_state_dir()));

    let integration_state = state_manager.load_integration_state()?.ok_or_else(|| {
        ParaError::git_operation(
            "No integration in progress. Use 'para integrate' to start a new integration."
                .to_string(),
        )
    })?;

    if !integration_state.is_in_conflict() {
        return Err(ParaError::git_operation(
            "No conflicts to resolve. Integration may already be complete.".to_string(),
        ));
    }

    let strategy_manager = git_service.strategy_manager();
    let conflict_manager = git_service.conflict_manager();

    let conflicts = conflict_manager.detect_conflicts()?;
    if !conflicts.is_empty() {
        println!(
            "⚠️  Cannot continue: {} conflicts remain unresolved",
            conflicts.len()
        );
        let summary = conflict_manager.get_conflict_summary()?;
        println!("{}", summary);
        return Err(ParaError::git_operation(
            "Resolve all conflicts before continuing".to_string(),
        ));
    }

    println!("🔄 Continuing integration...");
    state_manager.update_integration_step(IntegrationStep::ConflictsResolved)?;

    match strategy_manager.continue_integration()? {
        StrategyResult::Success { final_branch } => {
            state_manager.update_integration_step(IntegrationStep::IntegrationComplete)?;

            println!("✅ Integration completed successfully!");
            println!("🌿 Final branch: {}", final_branch);

            let session_state = session_manager.load_state(&integration_state.session_id)?;

            cleanup_after_successful_integration(
                &git_service,
                &session_manager,
                &config,
                &integration_state.session_id,
                &session_state.worktree_path,
                &integration_state.feature_branch,
            )?;

            state_manager.clear_integration_state()?;
        }
        StrategyResult::ConflictsPending { conflicted_files } => {
            state_manager.update_integration_step(IntegrationStep::ConflictsDetected {
                files: conflicted_files.clone(),
            })?;

            println!("⚠️  New conflicts detected:");
            for file in &conflicted_files {
                println!("   • {}", file.display());
            }
            let summary = conflict_manager.get_conflict_summary()?;
            println!("\n{}", summary);
            return Err(ParaError::git_operation(
                "New conflicts detected. Resolve them and run 'para continue' again.".to_string(),
            ));
        }
        StrategyResult::Failed { error } => {
            state_manager.update_integration_step(IntegrationStep::Failed {
                error: error.clone(),
            })?;

            return Err(ParaError::git_operation(format!(
                "Integration failed: {}",
                error
            )));
        }
        StrategyResult::DryRun { .. } => {
            unreachable!("Continue should not return dry run result")
        }
    }

    Ok(())
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

    let strategy_manager = git_service.strategy_manager();
    strategy_manager.abort_integration()?;

    state_manager.clear_integration_state()?;

    println!("✅ Integration aborted successfully");
    println!("🌿 Repository state restored");
    println!(
        "📋 Session '{}' remains active for further work",
        integration_state.session_id
    );

    Ok(())
}
