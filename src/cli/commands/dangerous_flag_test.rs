#[cfg(test)]
mod tests {
    use crate::cli::parser::{SandboxArgs, UnifiedStartArgs};
    use crate::core::session::{SessionManager, SessionStatus};
    use crate::test_utils::test_helpers::create_test_config;
    use tempfile::TempDir;

    #[test]
    fn test_dangerous_flag_persistence_start_to_resume() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let session_manager = SessionManager::new(&config);

        // Test 1: Create session state with dangerous flag (simulating what start command does)
        let session_with_flag =
            crate::core::session::state::SessionState::with_parent_branch_and_flags(
                "test-dangerous".to_string(),
                "para/test-dangerous".to_string(),
                temp_dir.path().join("worktree").join("test-dangerous"),
                "main".to_string(),
                true, // dangerous_skip_permissions = true
            );

        // Verify the flag is set correctly
        assert_eq!(session_with_flag.dangerous_skip_permissions, Some(true));
        assert_eq!(session_with_flag.status, SessionStatus::Active);

        // Save the session state
        session_manager.save_state(&session_with_flag).unwrap();

        // Test 2: Load the session state to verify persistence
        let loaded_session = session_manager.load_state("test-dangerous").unwrap();
        assert_eq!(loaded_session.dangerous_skip_permissions, Some(true));

        // Test 3: Resume operation would use this flag
        // In actual resume, this would be passed to ide_manager.launch()
        let skip_permissions = loaded_session.dangerous_skip_permissions.unwrap_or(false);
        assert!(skip_permissions);
    }

    #[test]
    fn test_dangerous_flag_not_set_by_default() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let session_manager = SessionManager::new(&config);

        // Create session state without dangerous flag (simulating normal start)
        let session_without_flag =
            crate::core::session::state::SessionState::with_parent_branch_and_flags(
                "test-safe".to_string(),
                "para/test-safe".to_string(),
                temp_dir.path().join("worktree").join("test-safe"),
                "main".to_string(),
                false, // dangerous_skip_permissions = false
            );

        // Verify session was created without dangerous flag (should be None, not Some(false))
        assert_eq!(session_without_flag.dangerous_skip_permissions, None);

        // Save the session state
        session_manager.save_state(&session_without_flag).unwrap();

        // Load the session state to verify persistence
        let loaded_session = session_manager.load_state("test-safe").unwrap();
        assert_eq!(loaded_session.dangerous_skip_permissions, None);

        // Resume operation would use default false
        let skip_permissions = loaded_session.dangerous_skip_permissions.unwrap_or(false);
        assert!(!skip_permissions);
    }

    #[test]
    fn test_unified_start_with_dangerous_flag() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Unified start with dangerous flag (equivalent to old dispatch)
        let _start_args = UnifiedStartArgs {
            name_or_session: Some("test-start".to_string()),
            prompt: Some("Test prompt".to_string()),
            file: None,
            dangerously_skip_permissions: true,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // Note: unified_start::execute requires Claude Code in wrapper mode
        // For this test, we'll directly create the session as unified start would
        let session_manager = SessionManager::new(&config);

        // Simulate what unified start does - create session with dangerous flag
        let session = crate::core::session::state::SessionState::with_parent_branch_and_flags(
            "test-start".to_string(),
            "para/test-start".to_string(),
            temp_dir.path().join("test-start"),
            "main".to_string(),
            true,
        );
        session_manager.save_state(&session).unwrap();

        // Verify session was created with dangerous flag
        let loaded = session_manager.load_state("test-start").unwrap();
        assert_eq!(loaded.dangerous_skip_permissions, Some(true));
    }

    #[test]
    fn test_backward_compatibility_resume() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let session_manager = SessionManager::new(&config);

        // Create an old-style session without dangerous_skip_permissions field
        let old_session = crate::core::session::state::SessionState::new(
            "old-session".to_string(),
            "para/old-session".to_string(),
            temp_dir.path().join("old-session"),
        );
        session_manager.save_state(&old_session).unwrap();

        // Resume should work with default value (None/false)
        let loaded = session_manager.load_state("old-session").unwrap();
        assert_eq!(loaded.dangerous_skip_permissions, None);

        // This would be treated as false when resuming
        let skip_permissions = loaded.dangerous_skip_permissions.unwrap_or(false);
        assert!(!skip_permissions);
    }
}
