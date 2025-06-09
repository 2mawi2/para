use crate::config::ConfigManager;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{IntegrationStateManager, IntegrationStep, SessionManager};
use crate::utils::{ParaError, Result};
use std::path::PathBuf;

pub fn execute() -> Result<()> {
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

    let integration_manager = git_service.integration_manager();
    let conflict_manager = git_service.conflict_manager();

    if !integration_manager.is_any_operation_in_progress()? {
        return Err(ParaError::git_operation(
            "No Git operation in progress. Cannot continue integration.".to_string(),
        ));
    }

    let conflicts = conflict_manager.detect_conflicts()?;
    if !conflicts.is_empty() {
        println!(
            "⚠️  Cannot continue: {} conflicts remain unresolved",
            conflicts.len()
        );
        println!("📁 Conflicted files:");
        for file in &conflicts {
            println!("   • {}", file.file_path.display());
        }
        let summary = conflict_manager.get_conflict_summary()?;
        println!("\n{}", summary);
        return Err(ParaError::git_operation(
            "Resolve all conflicts before continuing. Edit the files above, then run 'para continue' again.".to_string(),
        ));
    }

    println!("🔄 All conflicts resolved. Continuing integration...");
    state_manager.update_integration_step(IntegrationStep::ConflictsResolved)?;

    integration_manager.stage_resolved_files()?;
    println!("📦 Staged resolved files");

    if integration_manager.is_rebase_in_progress()? {
        match integration_manager.continue_rebase() {
            Ok(()) => {
                println!("✅ Rebase completed successfully");
            }
            Err(e) => {
                let conflicts = conflict_manager.detect_conflicts()?;
                if !conflicts.is_empty() {
                    let conflict_paths: Vec<PathBuf> = conflicts.iter()
                        .map(|c| c.file_path.clone())
                        .collect();
                    state_manager.update_integration_step(IntegrationStep::ConflictsDetected {
                        files: conflict_paths,
                    })?;
                    
                    println!("⚠️  New conflicts detected during rebase:");
                    for file in &conflicts {
                        println!("   • {}", file.file_path.display());
                    }
                    let summary = conflict_manager.get_conflict_summary()?;
                    println!("\n{}", summary);
                    return Err(ParaError::git_operation(
                        "New conflicts detected. Resolve them and run 'para continue' again.".to_string(),
                    ));
                } else {
                    return Err(ParaError::git_operation(format!(
                        "Failed to continue rebase: {}",
                        e
                    )));
                }
            }
        }
    } else if integration_manager.is_merge_in_progress()? {
        println!("🔄 Completing merge operation...");
        if let Err(e) = git_service.repository().commit("Complete merge after conflict resolution") {
            return Err(ParaError::git_operation(format!(
                "Failed to complete merge: {}",
                e
            )));
        }
    } else if integration_manager.is_cherry_pick_in_progress()? {
        println!("🔄 Continuing cherry-pick operation...");
        if let Err(e) = integration_manager.continue_rebase() {
            return Err(ParaError::git_operation(format!(
                "Failed to continue cherry-pick: {}",
                e
            )));
        }
    }

    state_manager.update_integration_step(IntegrationStep::IntegrationComplete)?;

    let current_branch = git_service.repository().get_current_branch()?;
    println!("✅ Integration completed successfully!");
    println!("🌿 Final branch: {}", current_branch);

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

    Ok(())
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

fn close_ide_for_session(config: &crate::config::Config, _worktree_path: &PathBuf) -> Result<()> {
    if config.is_wrapper_enabled() {
        return Ok(());
    }

    println!("🚪 IDE session will remain open for review");

    Ok(())
}
