use crate::cli::parser::{IntegrateArgs, IntegrationStrategy};
use crate::core::git::{GitService, StrategyRequest, StrategyResult};
use crate::utils::{ParaError, Result};

pub fn execute(args: IntegrateArgs) -> Result<()> {
    validate_integrate_args(&args)?;

    let git_service = GitService::discover()?;

    let current_branch = if let Some(ref session) = args.session {
        session.clone()
    } else {
        let current_dir = std::env::current_dir()
            .map_err(|e| ParaError::invalid_args(format!("Cannot get current directory: {}", e)))?;

        let env = git_service.validate_session_environment(&current_dir)?;
        match env {
            crate::core::git::SessionEnvironment::Worktree { branch, .. } => branch,
            _ => {
                return Err(ParaError::invalid_args(
                    "Not in a session worktree. Please specify session ID or run from session directory.".to_string()
                ));
            }
        }
    };

    let strategy_manager = git_service.strategy_manager();
    let target_branch = args.target.unwrap_or_else(|| {
        git_service
            .repository()
            .get_main_branch()
            .unwrap_or_else(|_| "main".to_string())
    });

    let strategy = if let Some(strategy) = args.strategy {
        strategy
    } else {
        strategy_manager.detect_best_strategy(&current_branch, &target_branch)?
    };

    println!(
        "🔄 Integrating branch '{}' into '{}'",
        current_branch, target_branch
    );
    println!("📋 Using {} strategy", format_strategy(&strategy));

    let request = StrategyRequest {
        feature_branch: current_branch.to_string(),
        target_branch: target_branch.clone(),
        strategy,
        dry_run: args.dry_run,
    };

    match strategy_manager.execute_strategy(request)? {
        StrategyResult::Success { final_branch } => {
            println!("✅ Integration successful!");
            println!("🌿 Final branch: {}", final_branch);

            if !args.dry_run {
                println!("🎯 Integration completed successfully");
            }
        }
        StrategyResult::ConflictsPending { conflicted_files } => {
            println!("⚠️  Integration paused due to conflicts");
            println!("📁 Conflicted files:");
            for file in &conflicted_files {
                println!("   • {}", file.display());
            }

            let conflict_manager = git_service.conflict_manager();
            let summary = conflict_manager.get_conflict_summary()?;
            println!("\n{}", summary);

            return Err(ParaError::git_operation(
                "Integration paused due to conflicts. Resolve conflicts and run 'para continue' to proceed.".to_string()
            ));
        }
        StrategyResult::DryRun { preview } => {
            println!("🔍 Integration Preview:");
            println!("{}", preview);
            println!("\n💡 Run without --dry-run to execute the integration");
        }
        StrategyResult::Failed { error } => {
            return Err(ParaError::git_operation(format!(
                "Integration failed: {}",
                error
            )));
        }
    }

    Ok(())
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
    let git_service = GitService::discover()?;
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

    match strategy_manager.continue_integration()? {
        StrategyResult::Success { final_branch } => {
            println!("✅ Integration completed successfully!");
            println!("🌿 Final branch: {}", final_branch);
        }
        StrategyResult::ConflictsPending { conflicted_files } => {
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
    let git_service = GitService::discover()?;
    let strategy_manager = git_service.strategy_manager();

    println!("🚫 Aborting integration...");
    strategy_manager.abort_integration()?;
    println!("✅ Integration aborted successfully");
    println!("🌿 Repository state restored");

    Ok(())
}
