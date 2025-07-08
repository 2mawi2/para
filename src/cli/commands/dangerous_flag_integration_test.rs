#[cfg(test)]
mod integration_tests {
    use crate::cli::parser::UnifiedStartArgs;
    use crate::core::session::state::SessionState;
    use crate::core::session::{SessionManager, SessionStatus};
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_dangerous_flag_integration_flow() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create state directory
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Step 1: Create a session with dangerous flag (simulating start/dispatch)
        let worktree_path = git_service.repository().root.join("dangerous-session");
        git_service
            .worktree_manager()
            .create_worktree("para/dangerous-session", &worktree_path)
            .unwrap();

        let session = SessionState::with_parent_branch_and_flags(
            "dangerous-session".to_string(),
            "para/dangerous-session".to_string(),
            worktree_path.clone(),
            "main".to_string(),
            true, // dangerous_skip_permissions = true
        );

        session_manager.save_state(&session).unwrap();

        // Step 2: Verify the session was saved with the flag
        let loaded = session_manager.load_state("dangerous-session").unwrap();
        assert_eq!(loaded.dangerous_skip_permissions, Some(true));
        assert_eq!(loaded.status, SessionStatus::Active);

        // Step 3: Simulate monitor resume (which would call unified start with the session)
        // In the real monitor, this happens via spawned command
        let monitor_resume_args = UnifiedStartArgs {
            name_or_session: Some("dangerous-session".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: true, // Monitor would add this based on session state
            container: false,
            docker_args: vec![],
            allow_domains: None,
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: crate::cli::parser::SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // Verify the args would include the flag
        assert!(monitor_resume_args.dangerously_skip_permissions);

        // Step 4: Test backward compatibility - old session without flag
        let old_worktree_path = git_service.repository().root.join("old-session");
        git_service
            .worktree_manager()
            .create_worktree("para/old-session", &old_worktree_path)
            .unwrap();

        // Create old-style session (no dangerous flag)
        let old_session = SessionState::new(
            "old-session".to_string(),
            "para/old-session".to_string(),
            old_worktree_path,
        );
        session_manager.save_state(&old_session).unwrap();

        // Verify old session loads with None
        let loaded_old = session_manager.load_state("old-session").unwrap();
        assert_eq!(loaded_old.dangerous_skip_permissions, None);

        // Step 5: Test the combined logic (session flag OR args flag)
        // Case 1: Session has flag, args doesn't
        let skip_1 = loaded.dangerous_skip_permissions.unwrap_or(false);
        assert!(skip_1, "Should use session flag");

        // Case 2: Session doesn't have flag, args does
        // Simulating: session_flag.unwrap_or(false) || args_flag
        let session_flag = loaded_old.dangerous_skip_permissions.unwrap_or(false);
        let args_flag = true;
        let skip_2 = session_flag || args_flag;
        assert!(skip_2, "Should use args flag");

        // Case 3: Neither has flag
        let skip_3 = loaded_old.dangerous_skip_permissions.unwrap_or(false);
        assert!(!skip_3, "Should not skip permissions");
    }

    #[test]
    fn test_monitor_dangerous_flag_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create a session with dangerous flag
        let worktree_path = temp_dir.path().join("monitor-test-worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session_state = SessionState::with_parent_branch_and_flags(
            "monitor-test".to_string(),
            "para/monitor-test".to_string(),
            worktree_path.clone(),
            "main".to_string(),
            true, // dangerous_skip_permissions = true
        );

        session_manager.save_state(&session_state).unwrap();

        // Simulate what the monitor does: load session and check flag
        let loaded = session_manager.load_state("monitor-test").unwrap();
        let use_dangerous_flag = loaded.dangerous_skip_permissions.unwrap_or(false);

        assert!(
            use_dangerous_flag,
            "Monitor should detect dangerous flag from session"
        );

        // The monitor would then spawn: para resume monitor-test --dangerously-skip-permissions
        // based on this flag
    }
}
