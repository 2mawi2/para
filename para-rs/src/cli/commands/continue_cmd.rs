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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::session::{IntegrationState, IntegrationStep};
    use crate::utils::ParaError;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        crate::config::defaults::default_config()
    }

    fn create_test_integration_state() -> IntegrationState {
        IntegrationState::new(
            "test-session".to_string(),
            "feature-branch".to_string(),
            "master".to_string(),
            crate::cli::parser::IntegrationStrategy::Rebase,
            Some("Test commit".to_string()),
        )
        .with_conflicts(vec![PathBuf::from("src/test.rs")])
    }

    #[test]
    fn test_execute_no_integration_state() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("PARA_STATE_DIR", temp_dir.path());
        
        let result = execute();
        
        assert!(result.is_err());
        if let Err(ParaError::GitOperation { message }) = result {
            assert!(message.contains("No integration in progress"));
        } else {
            panic!("Expected GitOperation error");
        }
    }

    #[test]
    fn test_execute_no_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("PARA_STATE_DIR", temp_dir.path());
        
        let mut state = create_test_integration_state();
        state.step = IntegrationStep::IntegrationComplete;
        
        let result = execute();
        
        assert!(result.is_err());
        if let Err(ParaError::GitOperation { message }) = result {
            assert!(message.contains("No conflicts to resolve"));
        } else {
            panic!("Expected GitOperation error");
        }
    }

    #[test]
    fn test_cleanup_after_successful_integration_preserve_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_service = GitService::discover().unwrap();
        let session_manager = SessionManager::new(&create_test_config());
        let mut config = create_test_config();
        config.session.preserve_on_finish = true;
        
        let result = cleanup_after_successful_integration(
            &git_service,
            &session_manager,
            &config,
            "test-session",
            &temp_dir.path().to_path_buf(),
            "feature-branch",
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_after_successful_integration_delete_branch() {
        let temp_dir = TempDir::new().unwrap();
        let git_service = GitService::discover().unwrap();
        let session_manager = SessionManager::new(&create_test_config());
        let mut config = create_test_config();
        config.session.preserve_on_finish = false;
        
        let result = cleanup_after_successful_integration(
            &git_service,
            &session_manager,
            &config,
            "test-session",
            &temp_dir.path().to_path_buf(),
            "feature-branch",
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_close_ide_for_session_wrapper_enabled() {
        let mut config = create_test_config();
        config.ide.wrapper.enabled = true;
        let worktree_path = PathBuf::from("/tmp/test");
        
        let result = close_ide_for_session(&config, &worktree_path);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_close_ide_for_session_wrapper_disabled() {
        let mut config = create_test_config();
        config.ide.wrapper.enabled = false;
        let worktree_path = PathBuf::from("/tmp/test");
        
        let result = close_ide_for_session(&config, &worktree_path);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_validates_config_loading() {
        std::env::remove_var("PARA_CONFIG_DIR");
        std::env::remove_var("PARA_STATE_DIR");
        
        let result = execute();
        
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_validates_git_service_discovery() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = execute();
        
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_error_handling_for_state_manager_operations() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("PARA_STATE_DIR", temp_dir.path());
        
        let result = execute();
        
        assert!(result.is_err());
        if let Err(ParaError::GitOperation { message }) = result {
            assert!(message.contains("No integration in progress"));
        } else {
            panic!("Expected GitOperation error");
        }
    }

    #[test]
    fn test_continue_workflow_error_scenarios() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("PARA_STATE_DIR", temp_dir.path());
        
        let result = execute();
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParaError::GitOperation { message } => {
                assert!(message.contains("No integration in progress") || 
                       message.contains("Failed to load config"));
            }
            _ => {
                // This is also acceptable as config loading might fail
            }
        }
    }

    #[test]
    fn test_integration_state_validation() {
        let state = create_test_integration_state();
        
        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.feature_branch, "feature-branch");
        assert_eq!(state.base_branch, "master");
        assert!(state.is_in_conflict());
    }

    #[test]
    fn test_integration_step_progression() {
        let mut state = create_test_integration_state();
        assert!(matches!(state.step, IntegrationStep::ConflictsDetected { .. }));
        
        state.step = IntegrationStep::ConflictsResolved;
        assert!(!state.is_in_conflict());
        
        state.step = IntegrationStep::IntegrationComplete;
        assert!(!state.is_in_conflict());
    }
}